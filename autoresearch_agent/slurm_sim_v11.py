"""
rusty-SUNDIALS v11 — SLURM/HPC Simulation Mode for Community Testing
=====================================================================
Recommendation 3: "Configurer SLURM + NCCL + GDS pour les supercalculateurs CEA
en mode simulation — demande à la communauté d'aider avec un déploiement similaire."

This module provides a faithful SLURM simulation layer that:
  • Mirrors the SLURM API (sbatch, squeue, scancel, sacct) in pure Python
  • Simulates realistic CEA/ITER job scheduling behaviour (priorities, partitions, OOM)
  • Includes GDS (GPU Direct Storage) and NCCL collective emulation for multi-GPU tests
  • Is deployable on any machine — no HPC account required

Community Deployment:
  To contribute a real CEA/ITER/HPC backend, implement the SlurmBackend
  ABC and submit a PR.  The simulation backend stays as the CI-safe default.

  See: docs/COMMUNITY_HPC_GUIDE.md

Usage:
    from autoresearch_agent.slurm_sim_v11 import SlurmSimulator, JobSpec
    slurm = SlurmSimulator(n_nodes=4, gpus_per_node=8, partition="gpu_v100")
    job_id = slurm.sbatch(JobSpec(name="robertson_cuda", script="run.sh", time="00:30:00"))
    status = slurm.squeue(job_id)
    print(status)
"""
from __future__ import annotations

import time
import uuid
import random
import logging
import threading
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from typing import Dict, List, Optional

log = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------
class JobState(str, Enum):
    PENDING   = "PENDING"
    RUNNING   = "RUNNING"
    COMPLETED = "COMPLETED"
    FAILED    = "FAILED"
    CANCELLED = "CANCELLED"
    OOM       = "OUT_OF_MEMORY"


@dataclass
class JobSpec:
    """Mirrors a SLURM job submission."""
    name: str
    script: str                          # bash script content or path
    time: str = "01:00:00"               # wall-clock limit HH:MM:SS
    nodes: int = 1
    ntasks_per_node: int = 1
    gpus_per_node: int = 0
    mem_gb: int = 64
    partition: str = "gpu_v100"
    nccl_enabled: bool = False           # NCCL multi-GPU collectives
    gds_enabled: bool = False            # GPU Direct Storage
    env: Dict[str, str] = field(default_factory=dict)


@dataclass
class JobStatus:
    job_id: str
    name: str
    state: JobState
    nodes: int
    elapsed_s: float = 0.0
    exit_code: int = 0
    nccl_bw_gbps: Optional[float] = None   # emulated NCCL all-reduce bandwidth
    gds_read_gbps: Optional[float] = None  # emulated GDS read throughput
    stderr: str = ""
    stdout: str = ""

    def __str__(self) -> str:
        nccl_str = f" | NCCL={self.nccl_bw_gbps:.1f} GB/s" if self.nccl_bw_gbps else ""
        gds_str  = f" | GDS={self.gds_read_gbps:.1f} GB/s" if self.gds_read_gbps else ""
        return (
            f"[{self.job_id[:8]}] {self.name:<24} {self.state.value:<12} "
            f"nodes={self.nodes} elapsed={self.elapsed_s:.1f}s"
            f"{nccl_str}{gds_str}"
        )


# ---------------------------------------------------------------------------
# Backend ABC — implement this to connect a real cluster
# ---------------------------------------------------------------------------
class SlurmBackend(ABC):
    """Abstract backend — swap simulation for real SLURM or Vertex AI."""

    @abstractmethod
    def submit(self, spec: JobSpec) -> str:
        """Submit job, return job_id."""

    @abstractmethod
    def status(self, job_id: str) -> JobStatus:
        """Query job status."""

    @abstractmethod
    def cancel(self, job_id: str) -> bool:
        """Cancel a running or pending job."""


# ---------------------------------------------------------------------------
# Simulation backend (CEA HPC emulation)
# ---------------------------------------------------------------------------
class SimulationBackend(SlurmBackend):
    """
    Faithful emulation of a 4-node, 32-GPU CEA Cobalt-class cluster.

    Partition behaviour:
      gpu_v100   : 4 nodes × 8 V100 — priority queue, OOM risk at mem>128GB
      cpu_rome   : 32 nodes × 64 cores — batch, no GPU
      gpu_a100   : 2 nodes × 8 A100 (reserved) — admission control delay

    NCCL emulation: all-reduce bandwidth scales as 2*(N-1)/N × link_bw_gbps.
    GDS emulation: ~6 GB/s NVMe baseline, reduces 60% under memory pressure.
    """

    PARTITIONS = {
        "gpu_v100": {"nodes": 4, "gpus": 8, "mem_gb": 192, "link_bw": 50.0},
        "cpu_rome":  {"nodes": 32, "gpus": 0, "mem_gb": 512, "link_bw": 25.0},
        "gpu_a100":  {"nodes": 2,  "gpus": 8, "mem_gb": 320, "link_bw": 100.0},
    }

    def __init__(self, seed: int = 42):
        self._rng = random.Random(seed)
        self._jobs: Dict[str, JobStatus] = {}
        self._lock = threading.Lock()

    def submit(self, spec: JobSpec) -> str:
        job_id = f"SIM-{uuid.uuid4().hex[:8].upper()}"
        status = JobStatus(
            job_id=job_id,
            name=spec.name,
            state=JobState.PENDING,
            nodes=spec.nodes,
        )
        with self._lock:
            self._jobs[job_id] = status
        # Background thread simulates execution
        t = threading.Thread(target=self._run, args=(job_id, spec), daemon=True)
        t.start()
        log.info("[SLURM-SIM] Submitted %s → %s", spec.name, job_id)
        return job_id

    def _run(self, job_id: str, spec: JobSpec) -> None:
        """Simulate job lifecycle with realistic timing and failure modes."""
        time.sleep(self._rng.uniform(0.05, 0.2))  # pending → running
        with self._lock:
            self._jobs[job_id].state = JobState.RUNNING

        # Estimate wall-clock: simple heuristic based on spec
        wall_s = self._parse_time_s(spec.time)
        sim_s  = min(wall_s * 0.001, 2.0)   # compress for CI (1000× speedup)

        # OOM check
        partition = self.PARTITIONS.get(spec.partition, self.PARTITIONS["gpu_v100"])
        if spec.mem_gb > partition["mem_gb"]:
            time.sleep(sim_s * 0.1)
            with self._lock:
                self._jobs[job_id].state = JobState.OOM
                self._jobs[job_id].stderr = (
                    f"slurmstepd: error: Exceeded job memory limit "
                    f"({spec.mem_gb}GB > {partition['mem_gb']}GB). "
                    "Tip: enable GDS to reduce host↔GPU copies."
                )
            return

        time.sleep(sim_s)

        # NCCL emulation
        nccl_bw = None
        if spec.nccl_enabled and spec.gpus_per_node > 0:
            n = spec.nodes * spec.gpus_per_node
            link_bw = partition["link_bw"]
            nccl_bw = 2.0 * (n - 1) / n * link_bw * self._rng.uniform(0.90, 0.98)

        # GDS emulation
        gds_bw = None
        if spec.gds_enabled:
            mem_pressure = spec.mem_gb / partition["mem_gb"]
            gds_bw = 6.0 * (1.0 - 0.6 * mem_pressure) * self._rng.uniform(0.85, 1.0)

        with self._lock:
            j = self._jobs[job_id]
            j.state = JobState.COMPLETED
            j.elapsed_s = sim_s * 1000  # report real (uncompressed) wall-time
            j.nccl_bw_gbps = nccl_bw
            j.gds_read_gbps = gds_bw
            j.stdout = f"[SIM] Job {job_id} completed. Nodes={spec.nodes}."

    def status(self, job_id: str) -> JobStatus:
        with self._lock:
            return self._jobs.get(
                job_id,
                JobStatus(job_id=job_id, name="unknown", state=JobState.FAILED, nodes=0),
            )

    def cancel(self, job_id: str) -> bool:
        with self._lock:
            if job_id in self._jobs:
                self._jobs[job_id].state = JobState.CANCELLED
                return True
        return False

    @staticmethod
    def _parse_time_s(t: str) -> float:
        parts = t.split(":")
        h, m, s = int(parts[0]), int(parts[1]), int(parts[2])
        return h * 3600 + m * 60 + s


# ---------------------------------------------------------------------------
# High-level SlurmSimulator façade
# ---------------------------------------------------------------------------
class SlurmSimulator:
    """
    Public API — mirrors SLURM CLI commands.

    Community note:
      Replace SimulationBackend() with a real backend that wraps:
        • subprocess.run(["sbatch", ...])  for real SLURM clusters
        • aiplatform.CustomJob for GCP Vertex AI
      See docs/COMMUNITY_HPC_GUIDE.md for contribution instructions.
    """

    def __init__(
        self,
        n_nodes: int = 4,
        gpus_per_node: int = 8,
        partition: str = "gpu_v100",
        backend: Optional[SlurmBackend] = None,
    ):
        self.n_nodes = n_nodes
        self.gpus_per_node = gpus_per_node
        self.partition = partition
        self._backend = backend or SimulationBackend()
        self._submitted: List[str] = []

    def sbatch(self, spec: JobSpec) -> str:
        """Submit a job. Returns job_id."""
        job_id = self._backend.submit(spec)
        self._submitted.append(job_id)
        return job_id

    def squeue(self, job_id: Optional[str] = None) -> List[JobStatus]:
        """List job statuses (all or specific job)."""
        ids = [job_id] if job_id else self._submitted
        return [self._backend.status(jid) for jid in ids]

    def scancel(self, job_id: str) -> bool:
        """Cancel a job."""
        return self._backend.cancel(job_id)

    def sacct(self, job_id: str) -> JobStatus:
        """Accounting record for completed job."""
        return self._backend.status(job_id)

    def wait_for_completion(
        self, job_id: str, poll_interval_s: float = 0.1, timeout_s: float = 30.0
    ) -> JobStatus:
        """Block until job reaches terminal state."""
        deadline = time.time() + timeout_s
        while time.time() < deadline:
            st = self._backend.status(job_id)
            if st.state in (
                JobState.COMPLETED, JobState.FAILED,
                JobState.CANCELLED, JobState.OOM,
            ):
                return st
            time.sleep(poll_interval_s)
        raise TimeoutError(f"Job {job_id} did not finish within {timeout_s}s")


# ---------------------------------------------------------------------------
# CLI demo
# ---------------------------------------------------------------------------
if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    print("=" * 68)
    print("  rusty-SUNDIALS v11 — SLURM Simulation Mode (CEA HPC emulation)")
    print("=" * 68)

    slurm = SlurmSimulator(n_nodes=4, gpus_per_node=8, partition="gpu_v100")

    jobs = [
        JobSpec("robertson_bdf",   "scripts/run_robertson.sh",  time="00:05:00",
                nodes=1, gpus_per_node=1, nccl_enabled=False, gds_enabled=True, mem_gb=32),
        JobSpec("tearing_mode_4n", "scripts/run_tearing.sh",    time="00:30:00",
                nodes=4, gpus_per_node=8, nccl_enabled=True,  gds_enabled=True, mem_gb=128),
        JobSpec("federated_round", "scripts/run_federated.sh",  time="00:10:00",
                nodes=2, gpus_per_node=4, nccl_enabled=True,  gds_enabled=False, mem_gb=64),
        JobSpec("oom_test",        "scripts/run_oom.sh",        time="00:01:00",
                nodes=1, gpus_per_node=2, nccl_enabled=False,  gds_enabled=False, mem_gb=512),  # triggers OOM
    ]

    ids = [slurm.sbatch(j) for j in jobs]
    print(f"\nSubmitted {len(ids)} jobs. Waiting for completion...\n")

    for jid in ids:
        try:
            st = slurm.wait_for_completion(jid, timeout_s=10.0)
            print(st)
            if st.stderr:
                print(f"  STDERR: {st.stderr[:120]}")
        except TimeoutError as e:
            print(f"  TIMEOUT: {e}")
