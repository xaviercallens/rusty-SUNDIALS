# Community HPC Contribution Guide

> **rusty-SUNDIALS v11** — Help us test on real supercomputers!

## 🎯 Goal

The CEA/ITER supercomputer is not currently accessible to the open-source maintainers.
We need community contributors to test and validate the `SlurmSimulator` against
real HPC environments so we can calibrate the simulation to match production behaviour.

---

## 📋 What We Need

| Priority | Cluster type | What to test |
|----------|-------------|-------------|
| 🔴 High | SLURM + V100/A100 | `slurm_sim_v11.py` job submission API |
| 🔴 High | NCCL multi-GPU | `nccl_enabled=True` bandwidth measurement |
| 🟡 Medium | GDS (GPU Direct Storage) | `gds_enabled=True` throughput numbers |
| 🟢 Low | CPU-only ROME/EPYC | Fallback `cpu_rome` partition tests |

---

## 🚀 Quick Start (Simulation Mode)

No HPC account needed — runs on any machine:

```bash
git clone https://github.com/xaviercallens/rusty-SUNDIALS
cd rusty-SUNDIALS
pip install -e autoresearch_agent/

# Run SLURM simulation demo
python autoresearch_agent/slurm_sim_v11.py
```

Expected output:
```
[SIM-XXXXXXXX] robertson_bdf     COMPLETED   nodes=1 elapsed=300.0s | GDS=5.8 GB/s
[SIM-XXXXXXXX] tearing_mode_4n   COMPLETED   nodes=4 elapsed=1800.0s | NCCL=93.4 GB/s | GDS=4.1 GB/s
[SIM-XXXXXXXX] federated_round   COMPLETED   nodes=2 elapsed=600.0s | NCCL=48.2 GB/s
[SIM-XXXXXXXX] oom_test          OUT_OF_MEMORY nodes=1 elapsed=...
```

---

## 🔧 Connecting a Real SLURM Cluster

Implement the `SlurmBackend` ABC:

```python
# my_cea_backend.py
import subprocess
from autoresearch_agent.slurm_sim_v11 import SlurmBackend, JobSpec, JobStatus, JobState

class CeaSlurmBackend(SlurmBackend):
    """Real SLURM backend for CEA Cobalt cluster."""

    def submit(self, spec: JobSpec) -> str:
        script = self._render_sbatch_script(spec)
        result = subprocess.run(
            ["sbatch", "--parsable", script],
            capture_output=True, text=True, check=True
        )
        return result.stdout.strip()  # returns "12345" job ID

    def status(self, job_id: str) -> JobStatus:
        result = subprocess.run(
            ["squeue", "-j", job_id, "--format=%T", "--noheader"],
            capture_output=True, text=True
        )
        state_str = result.stdout.strip() or "COMPLETED"
        return JobStatus(
            job_id=job_id, name="cea_job",
            state=JobState(state_str), nodes=1
        )

    def cancel(self, job_id: str) -> bool:
        result = subprocess.run(["scancel", job_id], capture_output=True)
        return result.returncode == 0

    def _render_sbatch_script(self, spec: JobSpec) -> str:
        lines = [
            "#!/bin/bash",
            f"#SBATCH --job-name={spec.name}",
            f"#SBATCH --time={spec.time}",
            f"#SBATCH --nodes={spec.nodes}",
            f"#SBATCH --ntasks-per-node={spec.ntasks_per_node}",
            f"#SBATCH --gres=gpu:{spec.gpus_per_node}" if spec.gpus_per_node else "",
            "#SBATCH --partition=gpu_v100",
        ]
        if spec.nccl_enabled:
            lines += ["export NCCL_DEBUG=INFO", "export NCCL_IB_DISABLE=0"]
        if spec.gds_enabled:
            lines += ["export PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True"]
        lines.append(spec.script)
        return "\n".join(filter(None, lines))

# Usage:
from autoresearch_agent.slurm_sim_v11 import SlurmSimulator
slurm = SlurmSimulator(backend=CeaSlurmBackend())
```

---

## 📊 Benchmarks We're Collecting

Please run this benchmark script and submit results via PR or GitHub Discussion:

```bash
python autoresearch_agent/slurm_sim_v11.py > my_cluster_results.txt 2>&1
```

Add to your PR:
- Cluster name and node spec (GPU model, memory, interconnect)
- Measured NCCL all-reduce bandwidth vs our simulation estimate
- Measured GDS throughput vs our simulation estimate
- Any OOM errors — include `--mem` values that triggered them

---

## 🏆 Contributors

| Contributor | Cluster | Date | Notes |
|-------------|---------|------|-------|
| *Your name here* | *Your HPC* | — | — |

---

## 📬 Contact

- GitHub Discussions: https://github.com/xaviercallens/rusty-SUNDIALS/discussions
- Issue tracker: label `hpc-community`
- Maintainer: [@xaviercallens](https://github.com/xaviercallens)
