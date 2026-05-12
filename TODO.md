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
- [ ] Ensure `CvodeBuilder` and `Rhs` closures are restricted to `Send + Sync`.

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
- [ ] Nordsieck rescaling with interpolation for large step-size changes
- [ ] Thread-safe `Cvode<F>: Send` for ensemble workflows

### v2.0 — Industrial Solver
- [ ] Preconditioned GMRES (left/right preconditioner callbacks + ILU(0))
- [ ] Sparse matrix support (CSR/CSC storage + sparse LU)
- [ ] Reproducible floating-point via compensated summation (Demmel & Nguyen 2015)
- [ ] `no_std` support for embedded scientific computing
- [ ] Python bindings via PyO3

### v2.5 / v3.0 — Advanced Solvers
- [ ] Forward sensitivity analysis (`cvodes`)
- [ ] IMEX splitting (`arkode`)
- [ ] DAE solver (`ida`)
- [ ] Adjoint sensitivity analysis

## Formal Verification
- [x] Scoped floating-point monotonicity axiom in `cvode.lean` (convert `admit` → `axiom`).
- [x] Extended `nvector_parallel.lean` with Separation Logic for multi-threaded memory disjointness.
- [x] Formally modeled `AssociativeReduction` class for parallel WRMS norm soundness.
- [x] Created `v2_upgrades.lean` outlining the formal specifications for the new Roadmap (Interpolation, Preconditioning, Reproducibility, Adjoints).
- [ ] Compile proofs to generate cryptographic certificate (pending disk space for `lean build`).

