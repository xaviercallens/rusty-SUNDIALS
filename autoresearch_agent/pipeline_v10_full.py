"""
rusty-SUNDIALS v10 — Full Pipeline Runner
=========================================
Executes all 5 remaining v10 components on a single A100 GPU (Vertex AI)
for <$100 total budget. Simulates CEA/ITER HPC without real SLURM access.

Execution plan (GCP A100 80GB, $3.93/hr):
  Component 2 (SLURM):    <1 min overhead (API only)       ~$0.07
  Component 4 (cuSPARSE): 2-3 min GPU compute              ~$0.20
  Component 6 (Flower):   1-2 min federated sim            ~$0.13
  Component 7 (PPO RL):   5-8 min training loop            ~$0.52
  Component 8 (SHAP+PySR):3-5 min symbolic regression      ~$0.33
  ─────────────────────────────────────────────────────────────────
  Total:                  ~12-19 min                        ~$1.25

Well under $100 budget. Result stored in discoveries/.

Usage:
  python3 pipeline_v10_full.py                   # local simulation
  python3 pipeline_v10_full.py --real-gpu        # deploy to Vertex AI A100
  python3 pipeline_v10_full.py --component 4     # single component only
"""

from __future__ import annotations
import argparse
import json
import logging
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

logger = logging.getLogger(__name__)

DISCOVERIES_DIR = Path(__file__).parent / "discoveries"
BUDGET_USD = float(os.environ.get("BUDGET_USD", "100"))

# Experimental mode: set EXPERIMENTAL=1 or pass --experimental to CLI.
# Activates Gate 2b (Spectral Fourier) + MixedPrecFGMRES + TensorCoreFP8AMG.
EXPERIMENTAL: bool = os.environ.get("EXPERIMENTAL", "0") == "1"

# ── Default hypothesis for all components ─────────────────────────────────────

DEFAULT_HYPOTHESIS = {
    "method_name": "FLAGNO_FP8_Divergence_Corrected",
    "description": (
        "Fractional Graph Neural Operator with FP8 block-sparse Hodge projection "
        "and AMGX algebraic multigrid preconditioning for xMHD tearing modes."
    ),
    "mathematical_basis": "Discrete de Rham Hodge decomposition + FP8 BSRM storage",
    "preserves_magnetic_divergence": True,
    "conserves_energy": True,
    "expected_speedup_factor": 78.3,
    "krylov_iteration_bound": "O(1)",
    # [Proposal 1] Spectral gate tolerance (Gate 2b when experimental=True)
    "fourier_divfree_tol": 1e-10,
}


# ── Component runners ──────────────────────────────────────────────────────────

def run_component_2(manager, hypothesis: dict) -> dict:
    """Component 2: SLURM full integration via Vertex AI BatchJob simulation."""
    logger.info("\n" + "="*60)
    logger.info("COMPONENT 2: SLURM Integration (Vertex AI Simulation)")
    logger.info("="*60)
    from slurm_v10 import SlurmJobManager, JobType

    results = {}
    for job_type, config, gpu_min in [
        (JobType.SUNDIALS_SIM,    {"n_dof": 512, "stiffness_ratio": 1e6}, 3),
        (JobType.CUSPARSE_BENCH,  {"dof_sizes": [128, 256, 512]},          2),
        (JobType.FEDERATED_ROUND, {"n_clients": 3, "round_num": 1},        2),
    ]:
        job = manager.sbatch(job_type, config, gpu_minutes_estimate=gpu_min)
        result = manager.wait_and_collect(job)
        results[job_type.value] = result
        logger.info(f"  ✅ {job_type.value}: {json.dumps(result)[:80]}…")

    return {"component": 2, "jobs": results,
            "slurm_api": "Vertex AI CustomJob (sbatch-compatible)"}


def run_component_4(manager, hypothesis: dict, experimental: bool = False) -> dict:
    """Component 4: cuSPARSE FP8 + AMGX benchmark.

    In experimental mode also benchmarks:
      Proposal 2 — MixedPrecisionFGMRES (CPU FP32 + Chebyshev)
      Proposal 3 — TensorCoreFP8AMG (BF16 pseudo-TC + FP64 refinement)
    with SHAP-optimal defaults: block_size=16, krylov_restart=30.
    """
    logger.info("\n" + "="*60)
    logger.info("COMPONENT 4: cuSPARSE FP8 + AMGX GPU Linear Solver"
                + (" [EXPERIMENTAL]" if experimental else ""))
    logger.info("="*60)
    from cusparse_amgx_v10 import (
        run_cusparse_amgx_benchmark,
        run_experimental_numeric_benchmark,
        DEFAULT_BLOCK_SIZE, DEFAULT_KRYLOV_RESTART,
    )

    logger.info(f"  SHAP-optimal defaults: block_size={DEFAULT_BLOCK_SIZE} "
                f"krylov_restart={DEFAULT_KRYLOV_RESTART}")

    dof_sweep = [128, 256, 512, 1024, 2048]
    results = []
    for n_dof in dof_sweep:
        result = run_cusparse_amgx_benchmark(
            n_dof=n_dof, stiffness_ratio=1e6, scenario="tearing_mode_3d")
        results.append(result.to_dict())
        logger.info(
            f"  n_dof={n_dof:4d}: memory_reduction={result.memory_reduction:.0f}× "
            f"| AMG_iters={result.amgx_iterations} "
            f"| speedup={result.speedup_factor:.1f}× "
            f"| Issue42={'RESOLVED' if result.issue_42_resolved else 'UNRESOLVED'}")

    max_reduction = max(r["memory_reduction"] for r in results)
    max_speedup   = max(r["speedup_factor"] for r in results)
    all_resolved  = all(r["issue_42_resolved"] for r in results)

    summary = {
        "max_memory_reduction": max_reduction,
        "max_speedup": max_speedup,
        "issue_42_resolved": all_resolved,
        "backend": results[0]["backend"],
    }

    # Experimental numeric benchmarks (Proposals 2 & 3)
    experimental_results = {}
    if experimental:
        logger.info("\n  --- Experimental Numeric Benchmarks ---")
        for prop, label in [(2, "MixedPrecFGMRES-CPU"), (3, "TensorCoreFP8AMG-GPU")]:
            try:
                exp_r = run_experimental_numeric_benchmark(
                    n_dof=512, stiffness_ratio=1e6, proposal=prop)
                experimental_results[label] = exp_r
                logger.info(
                    f"  [P{prop}] {label}: "
                    f"{exp_r['baseline_iters']}→{exp_r['experimental_iters']} iters "
                    f"| speedup={exp_r['speedup_vs_baseline']:.1f}× "
                    f"| converged={exp_r['converged']}"
                )
            except Exception as exc:
                logger.warning(f"  [P{prop}] benchmark failed: {exc}")
                experimental_results[label] = {"error": str(exc)}

    return {
        "component": 4,
        "dof_sweep": results,
        "summary": summary,
        "experimental": experimental_results,
    }


def run_component_6(manager, hypothesis: dict) -> dict:
    """Component 6: Flower federated learning."""
    logger.info("\n" + "="*60)
    logger.info("COMPONENT 6: Flower Federated Auto-Research (3 Sites)")
    logger.info("="*60)
    from federated_v10 import run_federated_experiment

    result = run_federated_experiment(
        hypothesis=hypothesis,
        n_clients=3,
        n_rounds=5,
        n_local_experiments=2,
    )
    fm = result["final_model"]
    logger.info(f"  Final aggregated speedup: {fm['final_speedup']:.2f}×")
    logger.info(f"  Convergence rate:         {fm['final_convergence_rate']:.2%}")
    logger.info(f"  Federated converged:      {fm['converged']}")
    logger.info(f"  Privacy:                  {result['privacy_guarantee']}")
    return {"component": 6, **result}


def run_component_7(manager, hypothesis: dict) -> dict:
    """Component 7: PPO RL agent training."""
    logger.info("\n" + "="*60)
    logger.info("COMPONENT 7: PPO RL Agent (SUNDIALS Parameter Optimization)")
    logger.info("="*60)
    from rl_agent_v10 import train_ppo_agent

    result = train_ppo_agent(
        n_episodes=15,
        max_steps_per_episode=25,
        budget_per_episode=2.0,
    )
    logger.info(f"  Backend:           {result.backend}")
    logger.info(f"  Best reward:       {result.best_reward:.3f}")
    logger.info(f"  Converged:         {result.convergence_achieved}")
    logger.info(f"  Final stability:   {result.final_stability_score:.3f}")
    logger.info(f"  Optimal params:    {json.dumps(result.best_action)[:80]}")
    return {"component": 7, **result.to_dict()}


def run_component_8(manager, hypothesis: dict) -> dict:
    """Component 8: SHAP + PySR explainability."""
    logger.info("\n" + "="*60)
    logger.info("COMPONENT 8: SHAP + PySR Explainability Pipeline")
    logger.info("="*60)
    from explainability_v10 import run_explainability_pipeline

    report = run_explainability_pipeline(n_samples=300, top_k_features=4)
    d = report.to_dict()
    logger.info(f"  Top global features: {report.top_global_features}")
    for target, eq in report.discovered_equations.items():
        logger.info(f"  {target}: {eq[:70]}")
    logger.info(f"  Academic grade:      {d['academic_grade']}")
    logger.info(f"  LaTeX:\n{report.to_latex_table()}")

    return {"component": 8, **d}


# ── Component map ──────────────────────────────────────────────────────────────

COMPONENTS = {
    2: run_component_2,
    4: run_component_4,
    6: run_component_6,
    7: run_component_7,
    8: run_component_8,
}


# ── Full pipeline ─────────────────────────────────────────────────────────────

def run_full_pipeline(
    use_real_gpu: bool = False,
    components: Optional[list[int]] = None,
    budget_usd: float = BUDGET_USD,
    experimental: bool = False,
) -> dict:
    """Run selected v10 components under budget constraint.

    Args:
        use_real_gpu: Submit to real Vertex AI A100 (requires GCP creds).
        components:   List of component IDs to run (default: all).
        budget_usd:   Hard budget cap in USD.
        experimental: Enable experimental mode (Gate 2b + Proposals 2 & 3).
    """
    from slurm_v10 import SlurmJobManager
    from neuro_symbolic_v10 import EXPERIMENTAL_GATES

    if experimental:
        # Propagate to env so sub-modules pick it up
        os.environ["EXPERIMENTAL_GATES"] = "1"
        os.environ["EXPERIMENTAL_NUMERIC"] = "1"
        logger.info("🧪 EXPERIMENTAL MODE ENABLED: Gate 2b + Proposals 2 & 3 active")

    if components is None:
        components = [2, 4, 6, 7, 8]

    DISCOVERIES_DIR.mkdir(parents=True, exist_ok=True)
    manager = SlurmJobManager(budget=budget_usd, use_real_gpu=use_real_gpu)

    session_start = time.time()
    all_results = {}

    logger.info("\n" + "🚀 " * 20)
    logger.info("rusty-SUNDIALS v10 — Full Pipeline Runner")
    logger.info(f"Components: {components}")
    logger.info(f"Budget: ${budget_usd:.2f} | GPU mode: {'REAL A100' if use_real_gpu else 'LOCAL SIM'}")
    logger.info("🚀 " * 20)

    for comp_id in components:
        runner = COMPONENTS.get(comp_id)
        if not runner:
            logger.warning(f"Unknown component {comp_id} — skipping.")
            continue
        try:
            # Pass experimental flag to component 4
            if comp_id == 4:
                result = runner(manager, DEFAULT_HYPOTHESIS, experimental=experimental)
            else:
                result = runner(manager, DEFAULT_HYPOTHESIS)
            all_results[f"component_{comp_id}"] = result
        except Exception as exc:
            logger.error(f"Component {comp_id} failed: {exc}", exc_info=True)
            all_results[f"component_{comp_id}"] = {"error": str(exc)}

    session_summary = manager.session_summary()
    wall_time = time.time() - session_start

    # Save full results
    output = {
        "run_timestamp": datetime.now(timezone.utc).isoformat(),
        "wall_time_s": round(wall_time, 1),
        "gpu_mode": "real_a100" if use_real_gpu else "local_simulation",
        "experimental_mode": experimental,
        "components_run": components,
        "budget_usd": budget_usd,
        "session": session_summary,
        "results": all_results,
    }

    out_file = DISCOVERIES_DIR / f"v10_pipeline_{int(time.time())}.json"
    with open(out_file, "w") as f:
        json.dump(output, f, indent=2, default=str)

    logger.info("\n" + "="*60)
    logger.info(f"✅ PIPELINE COMPLETE in {wall_time:.1f}s"
                + (" [🧪 EXPERIMENTAL]" if experimental else ""))
    logger.info(f"   Total jobs:   {session_summary['total_jobs']}")
    logger.info(f"   Succeeded:    {session_summary['succeeded']}")
    logger.info(f"   Total cost:   ${session_summary['total_cost_usd']:.4f}")
    logger.info(f"   Budget left:  ${session_summary['budget_remaining_usd']:.2f}")
    logger.info(f"   Results:      {out_file}")
    logger.info("="*60)

    return output


# ── CLI ───────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(message)s",
        datefmt="%H:%M:%S",
    )

    parser = argparse.ArgumentParser(description="rusty-SUNDIALS v10 Pipeline Runner")
    parser.add_argument("--real-gpu", action="store_true",
                        help="Submit to real Vertex AI A100 (requires GCP credentials)")
    parser.add_argument("--component", type=int, nargs="*",
                        help="Run specific component(s) only. E.g. --component 4 8")
    parser.add_argument("--budget", type=float, default=100.0,
                        help="Budget cap in USD (default: $100)")
    parser.add_argument("--experimental", action="store_true",
                        help="Enable experimental mode: Gate 2b (SpectralFourier) "
                             "+ Proposal 2 (MixedPrecFGMRES) + Proposal 3 (TensorCoreFP8AMG)")
    args = parser.parse_args()

    result = run_full_pipeline(
        use_real_gpu=args.real_gpu,
        components=args.component,
        budget_usd=args.budget,
        experimental=args.experimental,
    )

    print(f"\n💰 Total cost: ${result['session']['total_cost_usd']:.4f}")
    print(f"   Budget remaining: ${result['session']['budget_remaining_usd']:.2f}")
    if result.get("experimental_mode"):
        print("   🧪 Experimental mode was active (Gate 2b + Proposals 2 & 3)")
