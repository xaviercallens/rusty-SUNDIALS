# rusty-SUNDIALS v10 — Auto-Research Engine Roadmap

**SocrateAI Lab | SymbioticFactory Research | OpenCyclo Project**  
**Author:** Xavier Callens  
**Date:** May 14, 2026 | Based on v9.1.0 state

> **Vision:** Transform rusty-SUNDIALS from a manually-triggered benchmark platform into a fully autonomous scientific discovery engine — generating, validating, executing, and peer-reviewing hypotheses without human intervention, at exascale, for less than $1 per full discovery cycle.

---

## Current State (v9.x Baseline)

| Layer | Status | Gap to v10 |
|-------|--------|-----------|
| SUNDIALS integration | ✅ v6.1 wrappers | Automated pipeline (no manual triggers) |
| Lean 4 verification | ✅ sorry-free certs | Proof caching + auto-tactics |
| GPU compute | ✅ L4 Cloud Run FP8 | cuSPARSE, AMGX, TensorRT, SYCL |
| SOP reproducibility | ✅ 5 protocols | Federated multi-site execution |
| Peer review | ⚠️ Manual + mock LLM | Automated with open-source models |
| Hypothesis generation | ⚠️ Static | LLM-in-the-loop with feedback |
| Auto-validation | ❌ None | DeepProbLog + SymPy physics check |
| Exascale / SLURM | ❌ None | CEA/ITER cluster deployment |
| Explainability | ❌ None | SHAP + symbolic regression |

---

## v10 Architecture: The Auto-Research Loop

```
┌─────────────────────────────────────────────────────────────────┐
│                   AUTO-RESEARCH LOOP (v10)                       │
│                                                                   │
│  ① LLM Hypothesis ──→ ② Physics Validator ──→ ③ SUNDIALS Sim   │
│       ↑                     │ reject               │             │
│       │                     ↓                     ↓             │
│  ⑦ Fine-tune           Feedback            ④ Result Analysis   │
│       ↑                                          │             │
│  ⑥ Knowledge Base ←── ⑤ Lean 4 Proof ←─────────┘             │
│                                                                   │
│  Automated Peer Review: Gwen/Llama2 scores every step           │
└─────────────────────────────────────────────────────────────────┘
```

**Target:** Full discovery cycle in <2 min, <$0.50, with machine-verifiable provenance at every step.

---

## Component Roadmap

### Component 1: Reinforced Auto-Validation Engine

**Problem:** LLM-generated hypotheses are frequently non-physical or mathematically incoherent.

**Solution Stack:**
- **DeepProbLog** — probabilistic symbolic verification of causal structure
- **SymPy** — algebraic validation of proposed differential equations
- **Physics Check Module** — energy conservation, symmetry invariants, positivity bounds

**Rust/Python Interface:**
```rust
// physics_validator.rs — PyO3 bridge to SymPy
use pyo3::prelude::*;

#[pyfunction]
pub fn validate_physics(hypothesis: String) -> PyResult<bool> {
    Python::with_gil(|py| {
        let sympy = py.import("sympy")?;
        let result = sympy.call_method1("validate_hypothesis", (hypothesis,))?;
        result.extract::<bool>()
    })
}
```

**Validation Pipeline (5 gates):**
1. Rust syntax validity
2. Mathematical coherence (SymPy)
3. Physical bounds (DeepProbLog)
4. SUNDIALS convergence
5. Lean 4 proof obligation

**Estimate:** 2–3 weeks | Priority: ⭐⭐⭐⭐⭐

---

### Component 2: Fully Automated SUNDIALS Execution Pipeline

**Problem:** Manual simulation triggers are slow and error-prone.

**Solution:** End-to-end automation via Rust `tokio` + Docker containers.

```rust
// sundials_runner.rs
use std::process::Command;
use tokio::fs;

pub async fn run_sundials_simulation(config: &SimConfig) -> Result<SimResult> {
    // Write config
    fs::write("sundials_config.toml", config.to_toml()?).await?;

    // Execute via MPI (local) or submit to SLURM (CEA)
    let output = Command::new("mpirun")
        .args(["-np", &config.n_procs.to_string()])
        .arg("./target/release/sundials_runner")
        .arg("sundials_config.toml")
        .output()?;

    // Parse convergence metrics
    SimResult::parse(&output.stdout)
}
```

**Pipeline stages:**
1. Template → TOML config generation from hypothesis
2. Docker container isolation per simulation
3. MPI local / SLURM CEA execution
4. JSON result capture + auto-commit to `discoveries/`

**SLURM Target (CEA/ITER):**
```bash
#SBATCH --job-name=rusty-sundials-v10
#SBATCH --nodes=4
#SBATCH --ntasks-per-node=8
#SBATCH --gres=gpu:8
#SBATCH --time=02:00:00
module load cuda/12.0 openmpi/4.1.5
mpirun -np 32 ./target/release/sundials_runner config.toml
```

**Estimate:** 3–4 weeks | Priority: ⭐⭐⭐⭐⭐

---

### Component 3: Automated Peer Review (Gwen / Llama)

**Problem:** Manual peer review is a bottleneck; mock LLM review is not credible.

**Solution:** Open-source LLM peer review via Mistral Gwen-7B or Llama 3.

```python
# peer_review.py
from transformers import AutoModelForCausalLM, AutoTokenizer

MODEL = "mistralai/Mistral-7B-Instruct-v0.3"  # or local Gwen

def automated_peer_review(hypothesis: str, results: dict) -> dict:
    prompt = f"""
    [INST] You are a plasma physics expert peer reviewer.
    Evaluate the scientific validity of this hypothesis and its SUNDIALS results.
    
    Hypothesis: {hypothesis}
    Results: {results}
    
    Score from 0.0 to 1.0. List specific failures if score < 0.7. [/INST]
    """
    score = extract_score(generate(prompt))
    return {"score": score, "model": MODEL, "timestamp": utcnow()}
```

**Integration:**
- Every SOP execution auto-submits to the review model
- Score and rationale stored in the execution JSON telemetry
- Mission Control `/sop` shows the automated review score alongside the Lean 4 cert

**Estimate:** 2 weeks | Priority: ⭐⭐⭐⭐

---

### Component 4: GPU Stack Upgrade

| Technology | Role | Integration |
|-----------|------|-------------|
| **cuSPARSE** | Block-sparse sensitivity matrices in FP8 | Replace dense CUDA kernels for Jacobian assembly |
| **AMGX** | Algebraic multigrid linear solvers on GPU | Replace experimental AMG with NVIDIA-optimized solver |
| **TensorRT** | LLM inference optimization | Accelerate Gwen peer-review inference 10× |
| **SYCL (oneAPI)** | AMD/Intel GPU portability | Experimental backend for non-NVIDIA clusters |
| **JAX** | Gradient computation via XLA | Alternative to Enzyme-RS for adjoint autodiff |

**cuSPARSE FP8 Block-Sparse Example:**
```rust
// sparse_sensitivity.rs
// FP8 block-sparse storage for ghost sensitivity vectors
// Reduces OOM (Issue #42) by 8× vs FP64 dense storage
use cust::prelude::*;

pub struct BlockSparseSensitivity {
    pub values: DeviceBuffer<half::f8e4m3fn>,  // FP8 storage
    pub col_indices: DeviceBuffer<i32>,
    pub row_pointers: DeviceBuffer<i32>,
}
```

**Estimate:** 4–6 weeks | Priority: ⭐⭐⭐⭐

---

### Component 5: Lean 4 Proof Cache + Auto-Tactics

**Problem:** Re-proving identical structural obligations on each SOP run is expensive.

**Solution:**
- **Redis proof cache**: Hash the theorem statement → cache compiled proof term
- **Auto-tactics**: `decide` + `norm_num` + `omega` cover >80% of bound obligations automatically
- **Lean 4 metaprogramming**: `macro_rules!` for standard DEC cohomology patterns

```lean
-- proof_cache_pattern.lean
-- Use `decide` for discrete arithmetic bounds (auto-closes in <1ms)
theorem flagno_iters_le_7 : 6 ≤ 7 := by decide

-- Cache key: SHA256(theorem_statement) → proof_term
-- Redis TTL: 30 days (proofs are deterministic — eternal if axioms unchanged)
```

**Estimate:** 6–8 weeks | Priority: ⭐⭐⭐⭐

---

### Component 6: Federated Auto-Research

**Problem:** Scale discovery across multiple sites (CEA, ITER, university HPC).

**Solution:** Flower (flwr) federated learning over SUNDIALS experiment results.

```python
# federated_research.py
import flwr as fl

class SundialsResearchClient(fl.client.NumPyClient):
    """Each HPC site runs experiments locally; only aggregated results are shared."""
    
    def fit(self, params, config):
        hypotheses = params[0]
        validated = [h for h in hypotheses if validate_physics(h)]
        results = [run_sundials_async(h) for h in validated]
        return [aggregate_gradients(results)], len(validated), {}

# Central server aggregates with FedAvg — no raw data leaves each site
fl.server.start_server("[::]:8080", 
    config=fl.server.ServerConfig(num_rounds=10),
    strategy=fl.server.strategy.FedAvg())
```

**Privacy guarantee:** Raw plasma geometry data (CEA/ITER) never leaves local nodes.

**Estimate:** 6 weeks | Priority: ⭐⭐⭐

---

### Component 7: Neuro-Symbolic RL Agent

**Problem:** Parameter search (coil currents, mesh refinement) is still manual.

**Solution:** PPO agent with symbolic constraint enforcement from DeepProbLog.

```python
# sundials_rl_env.py
class SundialsEnv:
    """RL environment wrapping the SUNDIALS execution pipeline."""
    
    def step(self, action: np.ndarray):
        # action = proposed coil configuration or solver parameters
        if not validate_physics_constraints(action):
            return self.state, -1.0, False, {"invalid": True}
        
        results = run_sundials_simulation(action_to_config(action))
        reward = results["stability_score"] - 0.1 * results["cost_usd"]
        return results["state"], reward, results["converged"], results
```

**Estimate:** 8 weeks | Priority: ⭐⭐⭐

---

### Component 8: Explainable AI Layer

**Problem:** Physicists cannot trust black-box RL decisions for plasma control.

**Solution:** SHAP feature importance + symbolic regression over SUNDIALS outputs.

```python
# explainability.py
import shap
from pysr import PySRRegressor

def explain_simulation_result(X: np.ndarray, y: np.ndarray):
    """Fit interpretable model and explain coil parameter contributions."""
    
    # Symbolic regression: discovers human-readable equations
    model = PySRRegressor(
        niterations=100,
        binary_operators=["+", "*", "/", "-"],
        unary_operators=["exp", "log", "sqrt"],
    )
    model.fit(X, y)
    print(f"Discovered equation: {model.get_best()['equation']}")
    
    # SHAP for feature importance
    explainer = shap.TreeExplainer(RandomForestRegressor().fit(X, y))
    shap.summary_plot(explainer.shap_values(X), X)
```

**Estimate:** 3 weeks | Priority: ⭐⭐⭐

---

## Prioritized Execution Plan

| Phase | Component | Duration | Cost Target |
|-------|-----------|----------|------------|
| **P1** | Auto-Validation (DeepProbLog + SymPy) | Wk 1–3 | — |
| **P1** | SUNDIALS Pipeline Automation (tokio + Docker) | Wk 2–5 | — |
| **P2** | Gwen Peer Review Integration | Wk 4–6 | <$0.01/review |
| **P2** | cuSPARSE + AMGX GPU Stack | Wk 5–11 | — |
| **P3** | Lean 4 Proof Cache (Redis) | Wk 6–13 | — |
| **P3** | Neuro-Symbolic RL Agent | Wk 8–16 | — |
| **P4** | Federated Auto-Research (Flower) | Wk 10–16 | — |
| **P4** | SLURM / CEA Deployment | Wk 12–16 | — |
| **P5** | Explainable AI (SHAP + PySR) | Wk 14–17 | — |

**Total timeline:** ~17 weeks | **Budget per discovery cycle target:** <$0.50

---

## v10 Capability Comparison

| Criterion | v8 | v9.x | **v10** |
|-----------|----|----|---------|
| Auto-Research loop | ❌ | ⚠️ Partial | ✅ Continuous |
| Hypothesis validation | ❌ Manual | ⚠️ Basic | ✅ DeepProbLog + SymPy |
| SUNDIALS execution | ❌ Manual | ⚠️ Semi | ✅ Fully automated |
| Peer review | ❌ Manual | ❌ Manual | ✅ Gwen/Llama (open source) |
| Lean 4 proofs | ❌ | ✅ sorry-free | ✅ Cached + auto-tactics |
| GPU optimization | ❌ | ✅ L4 FP8 | ✅ cuSPARSE + AMGX + TensorRT |
| Exascale / SLURM | ❌ | ❌ | ✅ CEA/ITER deployment |
| Federated learning | ❌ | ❌ | ✅ Flower multi-site |
| Explainable AI | ❌ | ❌ | ✅ SHAP + PySR |
| Cost per cycle | N/A | $0.05 | <$0.50 (full loop) |

---

## Open Issues to Resolve Before v10

These map directly to known GitHub issues:

| Issue | Description | v10 Resolution |
|-------|-------------|---------------|
| #42 | OOM with FP64 sensitivity matrices | cuSPARSE FP8 Block-Sparse (Component 4) |
| #38 | AMGX integration missing | Component 4 GPU stack |
| #35 | SYCL experimental support | Component 4 portability layer |

---

## References

- SUNDIALS v7.4 Benchmarks: https://sundials.readthedocs.io/en/v7.4.0/developers/benchmarks/
- LLNL Exascale SUNDIALS+hypre: https://www.exascaleproject.org/highlight/sundials-and-hypre-exascale-capable-libraries-for-adaptive-time-stepping-and-scalable-solvers/
- Flower Federated Learning: https://flower.dev
- AMGX: https://developer.nvidia.com/amgx
- PySR Symbolic Regression: https://github.com/MilesCranmer/PySR
- Mistral/Gwen: https://github.com/mistralai

---

*rusty-SUNDIALS v10 Roadmap · SocrateAI Lab · Xavier Callens · May 2026*  
*License: CC BY 4.0 — citation required for all derived publications*
