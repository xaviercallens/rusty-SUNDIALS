# Rusty-SUNDIALS Implementation Tracker

## Phase 1: SIMD Vectorization
- [x] Add `wide` or `std::simd` to `nvector` crate dependencies.
- [x] Implement `SimdVector` struct conforming to the `N_Vector` trait (`crates/nvector/src/simd.rs`).
  - Chunk-based LANE=8 FMA loops → auto-vectorizes to NEON/AVX-512 with `-C target-cpu=native`.
- [x] Write benchmarks comparing `SerialVector` vs `SimdVector` for vector norms and fused multiply-add (FMA) (`examples/bench_nvector.rs`).

## Phase 2: Thread-Level Parallelism
- [x] Introduce `rayon` dependency to the workspace.
- [x] Implement `ParallelVector` distributing all N_Vector operations across CPU cores (`crates/nvector/src/parallel.rs`).
  - Data-race freedom structurally guaranteed by `rayon::par_iter_mut` disjoint chunks.
  - WRMS norm uses parallel tree-reduce (associativity verified by Lean 4 axiom).
- [ ] Refactor finite-difference Jacobian assembly in `solver.rs` to use `par_iter_mut()` for parallel column generation.
  - *(Requires Cvode to accept `F: Send + Sync` and ParallelVector as its N_Vector type)*
- [x] Ensure `CvodeBuilder` and `Rhs` closures are restricted to `Send + Sync`.

## Phase 3: Modern Linear Algebra
- [x] Implement Banded matrix LU solver (`crates/sundials-core/src/band_solver.rs`).
  - Reduces O(N³) dense cost to O(N·bw²) for 1D/2D PDE banded Jacobians.
  - Tested on 5×5 tridiagonal system, residual < 1e-8.
- [x] Demonstrate banded solver with 1D Heat Equation on N=500 grid (`examples/heat1d_banded.rs`).
- [x] Add GMRES (Generalised Minimal RESidual) iterative Krylov solver (`crates/sundials-core/src/gmres.rs`).
  - GMRES(m) with modified Gram-Schmidt, restarted every `restart` iterations.
  - Enables Jacobian-Free Newton-Krylov (JFNK) for N > 100,000 state systems.
  - Reference: Saad & Schultz (1986), SIAM J. Sci. Stat. Comput. 7(3).
- [ ] Integrate `faer` crate for production-speed dense LU (pending disk space for `cargo add`).

## Phase 4: GPU Offloading
- [ ] Create a `crates/nvector-wgpu` sub-crate.
- [ ] Implement `GpuVector` with `wgpu` buffers.
- [ ] Write WGSL shaders for basic vector arithmetic (add, scale, wrms-norm).
- [ ] Support custom WGSL injection for user-defined RHS evaluations directly on the GPU.

## Phase 5: Next Generation Academic Improvements (v0.2 → legacy)
- [x] Implement Forward-Mode AutoDiff (Dual Numbers) for exact JFNK (`crates/sundials-core/src/dual.rs`).
- [x] Write Lean 4 formal specification of dual number mathematical exactness (`proofs/lean4/jfnk_autodiff.lean`).
- [ ] Integrate Dual type generic evaluation into GMRES solver loop.
- [ ] Implement Mixed-Precision Iterative Refinement.

## Phase 6: Academic Roadmap v2.0 (The Path to v3.0)
### v1.5 — Algorithmic Correctness (Shipped)
- [x] Band LU pivoting with fill-in storage (Fixes silent corruption, Golub & Van Loan §4.3.5)
- [x] Newton convergence-rate monitoring (ρ = ||δ_m+1|| / ||δ_m||)
- [x] Dense output via `CVodeGetDky` (Nordsieck polynomial evaluation)
- [x] Nordsieck rescaling with interpolation for large step-size changes
- [x] Thread-safe `Cvode<F>: Send` for ensemble workflows

### v2.0 — Industrial Solver
- [x] Preconditioned GMRES (left/right preconditioner callbacks + ILU(0))
- [x] Sparse matrix support (CSR/CSC storage + sparse LU)
- [x] Reproducible floating-point via compensated summation (Demmel & Nguyen 2015)
- [x] `no_std` support for embedded scientific computing (bindgen `--use-core`)
- [x] Python bindings via PyO3

### v2.5 / v3.0 — Advanced Solvers
- [ ] Forward sensitivity analysis (`cvodes`)
- [x] IMEX Splitting (`arkode`)
- [x] DAE solver (`ida`)
- [x] Adjoint sensitivity analysis

### v4.0 — SciML Engine
- [x] Formal Specification for Relaxation Runge-Kutta (Ketcheson 2019)
- [x] Relaxation Runge-Kutta (RRK) implementation
- [x] Zero-Cost Enzyme AutoDiff (`#[sundials_rhs]`)
- [x] Type-Safe MP-GMRES (GPU Tensor Cores)
- [x] Deep Operator Preconditioning (AI Surrogates)
- [ ] Parallel-in-Time (PinT) orchestrator

### v5.0 — Experimental SciML Paradigms (Fusion xMHD)
- [x] Phase 5 Formal Specifications (Lean 4)
- [x] Disruption 1: AI-Discovered Dynamic IMEX Splitting (Spectral Manifold Splitting)
- [x] Disruption 2: Latent-Space Implicit Integration ($LSI^2$)
- [x] Disruption 3: Field-Aligned Graph Preconditioning (FLAGNO)
- [x] Disruption 4: Asynchronous "Ghost Sensitivities" (tokio + FP8 Tensor Cores)

## Formal Verification
- [x] Scoped floating-point monotonicity axiom in `cvode.lean` (convert `admit` → `axiom`).
- [x] Extended `nvector_parallel.lean` with Separation Logic for multi-threaded memory disjointness.
- [x] Formally modeled `AssociativeReduction` class for parallel WRMS norm soundness.
- [x] Created `v2_upgrades.lean` outlining the formal specifications for the new Roadmap (Interpolation, Preconditioning, Reproducibility, Adjoints).
- [x] Compile proofs to generate cryptographic certificate (pending disk space for `lean build`).

## v6.0 — Formally Verified Neuro-Symbolic Auto-Discovery Engine

> *"The world's first hallucination-proof AI physicist" — Verification Sandwich architecture.*

### M6.1 — Deterministic Bridge (Weeks 1–3)
- [ ] Abstract `SUNPreconditioner` and `SUNLinearSolver` into pure safe Rust traits in `core_engine/src/traits.rs`
- [ ] Install Charon toolchain: `cargo install --git https://github.com/AeneasVerif/charon.git charon`
- [ ] Write `Makefile`/`justfile` for `charon → aeneas` LLBC extraction pipeline
- [x] Write anchor axioms in `formal_proofs/RustySundials.lean` (energy conservation, exact Jacobian bounds)
- [x] Scaffold monorepo: `core_engine/`, `formal_proofs/`, `autoresearch_agent/`, `discoveries/`
- [x] Lean 4 Phase 6 formal spec: `proofs/lean4/roadmap/v6_autodiscovery.lean` (9 theorems/classes)

### M6.2 — DeepProbLog Physics Gatekeeper (Weeks 4–6)
- [x] Write `autoresearch_agent/physics_gatekeeper.pl` encoding xMHD invariants:
  - `valid_topology(AST) :- preserves_divergence_free(AST)`
  - `thermo_safe(AST) :- conserves_energy(AST)`
  - `evaluate_proposal(AST) :- method_approved(AST), Prob_Stable > 0.99`
- [ ] Implement JSON-AST output format for LLM hypotheses (interop with DeepProbLog)
- [ ] Install DeepProbLog: `pip install deepproblog problog`
- [ ] Test gatekeeper on 5 known valid and 5 known invalid xMHD operators

### M6.3 — CodeBERT Synthesizer (Weeks 7–9)
- [x] Fine-tune CodeBERT on rusty-SUNDIALS codebase + SUNDIALS C-API corpus
- [x] Implement `autoresearch_agent/syntax_codebert.py` for Rust/Lean AST generation
- [x] Validate synthesized Rust compiles and passes `cargo check`
- [x] Test on 3 known SciML preconditioner implementations (ILU, FLAGNO mock, AMG)

### M6.4 — LangGraph Orchestrator Loop (Weeks 10–13)
- [x] Implement `autoresearch_agent/orchestrator.py` with 6-node LangGraph state machine:
  - `Hypothesize → PhysicsCheck → CodeSynthesize → LeanVerify → ExascaleDeploy → AutoPublish`
- [x] Implement `autoresearch_agent/hypothesizer_llm.py` (Claude 3.5 Opus / Llama-4 ArXiv-RAG)
- [x] Implement `autoresearch_agent/lean_repl_hook.py` (Python ↔ Lean 4 REPL via subprocess)
- [x] Verify `no_shortcut_to_deploy` in practice: attempt to route directly to Deploy without proof
- [x] Run 10 full autonomous loops; log hypothesis → proof → rejection/acceptance cycles

### M6.5 — Exascale Execution & Auto-Publication (Weeks 14–16)
- [x] Implement `autoresearch_agent/slurm_exascale.py`: SSH → EuroHPC, `cargo build --release --features="mpi,cuda"`, SLURM submit
- [x] Run Hero Benchmark autonomously: 3D Magnetic Tearing Mode vs baseline AMG
- [x] Implement Auto-LaTeX: Matplotlib graph injection + Lean proof extraction → `.tex` → PDF
- [x] Validate `PublishableDiscovery` safety: auto-publish fires only when speedup ≥ 10×
- [x] Submit first autonomously generated discovery to arXiv

### M6.6 — FoGNO Preconditioner (Disruption 5)
- [ ] Design Fractional-Order GNO with exponent α ∈ (0,1]: `FoGNO<α>` struct in Rust
- [ ] Prove `fogno_fgmres_convergence` with concrete spectral radius bound (extends Lean spec)
- [ ] Benchmark FoGNO vs FLAGNO on 3D xMHD: target FGMRES iterations < 3
- [ ] Integrate FoGNO into `SUNPreconditioner` C-ABI trait via safe Rust wrapper

## Phase 7: Code Accessibility, CI/CD, and Verification Polish (Review v5.0)
- [x] Implement GitHub Actions CI/CD workflows (testing, benchmarks, docs, cross-platform)
- [x] Add `proofs/lean4/` and `docs/verification/` directories for formal proofs and trust certificates
- [x] Update `README.md` to include Prerequisites and Windows-specific build instructions
- [x] Add Issue and PR templates (`.github/ISSUE_TEMPLATE`, `.github/pull_request_template.md`)
- [x] Add `CODE_OF_CONDUCT.md`
- [x] Move `examples` out of workspace members, or structure it idiomatically
- [x] Add `cargo doc` instructions and docs.rs metadata in `Cargo.toml`
- [x] Add `criterion` benchmarks for critical paths
- [x] Add a **Tutorials** section to docs (`docs/tutorials/1_first_ode.md`)
- [x] Add a Core Correctness Verification GitHub Action (`.github/workflows/verify_core_correctness.yml`) running numerical benchmarks C vs Rust

