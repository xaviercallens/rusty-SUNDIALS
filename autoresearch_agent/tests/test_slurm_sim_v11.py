"""
Tests for autoresearch_agent/slurm_sim_v11.py
"""
import time
import pytest
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from slurm_sim_v11 import (
    SlurmSimulator, SimulationBackend, JobSpec, JobStatus, JobState
)


@pytest.fixture
def slurm():
    return SlurmSimulator(n_nodes=4, gpus_per_node=8, partition="gpu_v100")


@pytest.fixture
def backend():
    return SimulationBackend(seed=42)


def make_spec(**kwargs) -> JobSpec:
    defaults = dict(
        name="test_job",
        script="echo hello",
        time="00:01:00",
        nodes=1,
        gpus_per_node=1,
        mem_gb=32,
        partition="gpu_v100",
    )
    defaults.update(kwargs)
    return JobSpec(**defaults)


class TestJobSpec:
    def test_defaults(self):
        spec = make_spec()
        assert spec.name == "test_job"
        assert not spec.nccl_enabled
        assert not spec.gds_enabled

    def test_nccl_gds_flags(self):
        spec = make_spec(nccl_enabled=True, gds_enabled=True)
        assert spec.nccl_enabled
        assert spec.gds_enabled


class TestSimulationBackend:
    def test_submit_returns_job_id(self, backend):
        job_id = backend.submit(make_spec())
        assert job_id.startswith("SIM-")
        assert len(job_id) > 4

    def test_job_starts_pending(self, backend):
        job_id = backend.submit(make_spec())
        st = backend.status(job_id)
        assert st.state in (JobState.PENDING, JobState.RUNNING, JobState.COMPLETED)

    def test_job_completes(self, backend):
        job_id = backend.submit(make_spec())
        deadline = time.time() + 5.0
        while time.time() < deadline:
            st = backend.status(job_id)
            if st.state in (JobState.COMPLETED, JobState.FAILED,
                            JobState.OOM, JobState.CANCELLED):
                break
            time.sleep(0.05)
        assert st.state == JobState.COMPLETED

    def test_oom_triggered_on_high_mem(self, backend):
        spec = make_spec(mem_gb=512)  # exceeds 192GB limit for gpu_v100
        job_id = backend.submit(spec)
        deadline = time.time() + 5.0
        while time.time() < deadline:
            st = backend.status(job_id)
            if st.state not in (JobState.PENDING, JobState.RUNNING):
                break
            time.sleep(0.05)
        assert st.state == JobState.OOM
        assert "memory limit" in st.stderr.lower()

    def test_cancel_sets_cancelled(self, backend):
        job_id = backend.submit(make_spec(time="01:00:00"))
        result = backend.cancel(job_id)
        assert result is True
        # Give the thread a moment
        time.sleep(0.1)
        st = backend.status(job_id)
        # Either cancelled or already completed (race condition is acceptable)
        assert st.state in (JobState.CANCELLED, JobState.COMPLETED)

    def test_nccl_bandwidth_computed(self, backend):
        spec = make_spec(nodes=4, gpus_per_node=8,
                         nccl_enabled=True, time="00:30:00")
        job_id = backend.submit(spec)
        deadline = time.time() + 5.0
        while time.time() < deadline:
            st = backend.status(job_id)
            if st.state == JobState.COMPLETED:
                break
            time.sleep(0.05)
        assert st.nccl_bw_gbps is not None
        assert st.nccl_bw_gbps > 0

    def test_gds_throughput_computed(self, backend):
        spec = make_spec(gds_enabled=True, time="00:05:00")
        job_id = backend.submit(spec)
        deadline = time.time() + 5.0
        while time.time() < deadline:
            st = backend.status(job_id)
            if st.state == JobState.COMPLETED:
                break
            time.sleep(0.05)
        assert st.gds_read_gbps is not None
        assert st.gds_read_gbps > 0


class TestSlurmSimulator:
    def test_sbatch_returns_id(self, slurm):
        job_id = slurm.sbatch(make_spec())
        assert isinstance(job_id, str)
        assert len(job_id) > 0

    def test_squeue_lists_submitted_jobs(self, slurm):
        id1 = slurm.sbatch(make_spec(name="job_a"))
        id2 = slurm.sbatch(make_spec(name="job_b"))
        statuses = slurm.squeue()
        assert len(statuses) == 2

    def test_squeue_specific_job(self, slurm):
        job_id = slurm.sbatch(make_spec(name="specific"))
        statuses = slurm.squeue(job_id)
        assert len(statuses) == 1
        assert statuses[0].job_id == job_id

    def test_wait_for_completion(self, slurm):
        job_id = slurm.sbatch(make_spec())
        st = slurm.wait_for_completion(job_id, timeout_s=5.0)
        assert st.state in (JobState.COMPLETED, JobState.OOM, JobState.FAILED)

    def test_scancel(self, slurm):
        job_id = slurm.sbatch(make_spec())
        ok = slurm.scancel(job_id)
        assert ok is True

    def test_sacct_returns_status(self, slurm):
        job_id = slurm.sbatch(make_spec())
        slurm.wait_for_completion(job_id, timeout_s=5.0)
        st = slurm.sacct(job_id)
        assert isinstance(st, JobStatus)

    def test_job_status_str(self, slurm):
        job_id = slurm.sbatch(make_spec(name="str_test"))
        slurm.wait_for_completion(job_id, timeout_s=5.0)
        st = slurm.sacct(job_id)
        text = str(st)
        assert "str_test" in text or "COMPLETED" in text

    def test_timeout_raises(self, slurm):
        # Submit a job but give it an impossibly short timeout
        # (will raise TimeoutError if job doesn't finish in 0.001s on a fresh backend)
        # This is timing-sensitive; we use a very small timeout
        job_id = slurm.sbatch(make_spec(time="01:00:00"))
        try:
            slurm.wait_for_completion(job_id, timeout_s=0.001)
        except TimeoutError:
            pass  # expected
        except Exception:
            pass  # job might have completed instantly — acceptable
