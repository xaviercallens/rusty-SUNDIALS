# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Lean 4 proof instructions (`proofs/lean4/README.md`)
- Master plan specifications (`SPECS.md`)
- GitHub issue and pull request templates
- Moved benchmark script to `examples/run_benchmarks.sh`

---

## [6.4.1] — 2026-09-27 — MCP Server + CMB Dark-Energy Bound

### Last Improvement Summary

Three PRs merged in this session:

**PR #54 — License & IdaSolver clarification**
- Replaced the custom CC/Apache dual-license `LICENSE` file with standard **BSD-3-Clause** text to
  match the existing `Cargo.toml` `license = "BSD-3-Clause"` declaration and README badge.
- Added a doc comment to `IdaSolver` (`crates/ida/src/solver.rs`) clarifying it is a fixed-step
  Backward Euler *prototype*, not a full Newton-Krylov variable-order IDA solver.

**PR #57 — `qf-cmb-cascade`: pure-Rust CMB dark-energy-transition bound**
- New crate `crates/qf-cmb-cascade`: a pure-Rust port of the CMB cascade cosmology solver from
  SocrateAI-Scientific-QuantumFluids.
- Implements the **exact** bubble-completion-time power spectrum (Elor et al. arXiv:2311.16222,
  Supplementary Eqs. S1–S16), checked against the real Planck 2018 TT power spectrum.
- Checks the Koren-Tsai-Wang bound (arXiv:2509.07076) on late-time first-order dark-energy phase
  transitions. Python's own design doc documented that the full grid did *not finish* ("minutes per
  point"); this Rust port completes the **full 12+10-point (beta/H\*, zpt) grid in ~8.8 minutes**.
- Key result confirmed across the full grid: Planck's real TT data is already within 0–39% of the
  cosmic-variance floor across beta/H\*=10–500 at both zpt values, tightening to exactly 1.00× as
  beta/H\* grows — **no future CMB-temperature-only experiment can meaningfully improve this bound.**
- 5/5 unit tests pass; clippy clean.
- Also adds `docs/PROPOSED_STRUCTURE.md` — a documentation-only proposal for reorganizing `crates/`
  into `core/`, `bindings/`, and `physics/` subdirectories (no files moved in this PR).

**qf-pgpe external reproduction (this PR)**
- `rect` (rectangular periodic grids, numpy-convention FFTs), `npy` (numpy file I/O), `flow` (obstacle-flow solver with moving frame and absorbing layers, independent RK4 scheme) and the `kwon_shin` example: the force on the obstacle of an external published run (Kwon & Shin, Zenodo 10.5281/zenodo.20068724) is reproduced to 6e-6 relative from the reference's own initial field; honest note that the Rust engine is not faster than numpy at 1000 x 500.

**qf-pgpe reference engine (branch `feat/qf-pgpe-reference`, supersedes #68)**
- Vortex instrument moved into the library (`vortex`), `transport` estimators (bit-identical to Python on real tracks, 2e-14), `thermal` toolkit, `scattering` instrument and a parallel resumable `wave_scan`; `rhs`/`pack`/`unpack`; allocation-free step and unit-stride FFT (bit-identical, ~1.7x).
- CVODE cross-check of the PGPE engine (IF-RK4 error vs CVODE falls as dt^4); requires the Adams-order fix of #63 (365 RHS evaluations vs MaxSteps after ~10^6 without it).
- Found by the cross-checks: a grid-node imprint defect (fixed) and an estimator bias in the Python programme (energy estimator of alpha low by diffusion). See `crates/qf-pgpe/README.md`.

**PR #58 — `sundials-mcp`: Model Context Protocol server**
- New crate `crates/sundials-mcp`: a **Model Context Protocol** stdio server (hand-rolled JSON-RPC
  2.0, `serde_json` only, `#![forbid(unsafe_code)]`) that lets AI agents and scientists run
  rusty-SUNDIALS solvers with every result's evidentiary status made explicit.
- **5 tools**:

  | Tool | What it does | Checked against |
  |------|-------------|-----------------|
  | `about` | Server scope, conventions, what is exposed | — |
  | `list_problems` | Named CVODE problems, parameters, bounds | — |
  | `solve` | `exponential`, `robertson`, `vanderpol`, `lorenz` via CVODE BDF | `exp(-t)` closed form; Robertson: mass conservation + LLNL cvRoberts_dns output at t=0.4 |
  | `pgpe_run` | `qf-pgpe` GPE solver, 16/32/64 grid, ≤4000 steps | Plane wave exact; norm/momentum conservation |
  | `cmb_bound` | Koren-Tsai-Wang 2σ CMB dark-energy bound via `qf-cmb-cascade` (10–30 s/call) | Planck 2018 TT bars or cosmic-variance floor |

- **Honesty conventions**: every result carries `ran`; `ran: false` is never a pass; solver failures
  are returned as `isError: true` with no partial trajectory; each known-answer check names its
  independent reference.
- **Solver isolation**: solvers run in a `sundials-mcp --worker` subprocess (stdout → stderr of the
  parent) to prevent `println!` diagnostics from corrupting the MCP stdio stream. A wall-clock
  timeout (`SUNDIALS_MCP_TIMEOUT_SECS`, default 120 s) is enforced.
- 14 tests (10 unit + 4 stdio end-to-end including planted-defect/control cases) pass.
- Supports MCP protocol versions: 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25.
- **Build & register**:
  ```bash
  cargo build --release -p sundials-mcp
  claude mcp add sundials -- /path/to/rusty-SUNDIALS/target/release/sundials-mcp
  ```

Also fixed a pre-existing `rustfmt` CI failure in `examples/iter_disruption_3d.rs` (long `println!`
lines and trailing whitespace that the stable toolchain formatter wanted to reformat).

---

### Next Steps (v7.0 Roadmap)

Ordered by feasibility and scientific impact:

1. **3D Toroidal Extension** (`examples/iter_disruption_3d.rs`) ← **execute next**
   - Extend from 2D (ρ, θ) to 3D (ρ, θ, φ) with toroidal mode coupling (n=1).
   - Target: N_φ=16 slices → 1.28M plasma+vessel DOF.
   - `make_3d_torus()` geometry already in `iter_disruption_viz.py`; only the Rust solver loop and
     coupling term need to be added.

2. **Adaptive Eisenstat-Walker Precision Forcing** (`crates/cvode/src/solver.rs`)
   - Implement `EisenstatWalkerForcing` trait to tighten FP8→FP16→FP32 inner Krylov tolerance as
     Newton converges, recovering superlinear convergence near the solution.

3. **GPU-Native Baseline Ablation**
   - Benchmark the GNN preconditioner against `cuSPARSE` ILU0 on the same H100 GPU to isolate
     algorithmic vs. hardware speedup. Requires H100 access on GCP (~$5–10).

4. **Alternative Neural Architectures**
   - Compare GNN (MPNN) preconditioner against FNO and DeepONet for the ITER plasma proxy.

5. **Formal Verification Completion** (community contribution welcome)
   - Mechanize the 2 remaining `sorry` markers in `proofs/NeuralFGMRES_Convergence.lean`:
     `fp8_preconditioner_stability` and `fp8_indefinite_stability` (need Mathlib bilinear form
     decomposition and Cauchy-Schwarz for operator-norm bounded perturbations).

6. **CVODES Adjoint Sensitivity** (`crates/cvode/`)
   - Add adjoint sensitivity analysis (backward-in-time integration) for optimal control use cases.

7. **Edge Deployment** (v7.0)
   - Compile `ida-rs` pH-Stat controller to ARM binary for Raspberry Pi / STM32.
   - Connect OD and pH sensors to the prediction model for a real closed cyber-physical loop.

## [6.0.0] - 2026-05-13
### Added
- Formal verification with Lean 4 proofs and trust certificates.
- Comprehensive documentation (tutorials, papers, mathematical background).
- CI/CD automation (testing, C vs. Rust verification).
- Cross-platform support (Linux, macOS, Windows).
- Empirical validation (4 SciML experiments, benchmarks).
- Community readiness (Code of Conduct, contribution guidelines).
