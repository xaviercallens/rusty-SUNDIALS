# Master Plan for v7.0 and Beyond (SPECS.md)

## 📊 Executive Summary
This document acts as the Master Plan for the v7.0 and beyond improvements, addressing the gaps highlighted in the recent "Improvement report for v7.0". Rusty-SUNDIALS has reached an incredible state of maturity (v6.0.0), bringing formal verification, CI/CD, documentation, and experiments together.

This specification tracks the **Critical**, **High-Priority**, and **Medium-Priority** steps to bridge the final gaps for production readiness.

## 🎯 Priority Action Plan

### 🔴 Phase 1: Critical Fixes
- [x] **Add `examples/run_benchmarks.sh`** to the repository.
- [x] **Add a `README.md` to `proofs/lean4/`** with Lean 4 setup and run instructions.
- [x] **Add Issue/PR Templates** (`.github/ISSUE_TEMPLATE/`, `.github/PULL_REQUEST_TEMPLATE.md`).
- [x] **Add `CHANGELOG.md`** (using Keep a Changelog format).
- [x] **Clarify Roadmap**: Move unimplemented v2.0 features (sparse matrices, PyO3, `no_std`) back to the roadmap and adjust ETAs.

### 🟡 Phase 2: High-Impact Improvements
- [x] **Publish to `crates.io`**: Publish `cvode`, `nvector`, and `sundials-core`. Enable `docs.rs` for all crates.
- [x] **Windows CI**: Add a Windows job to `.github/workflows/ci.yml`.
- [x] **Coverage CI**: Integrate `grcov` or `tarpaulin` into CI.
- [x] **Linting in CI**: Add `clippy` and `rustfmt` to CI.
- [x] **Expand Tutorials**: Add tutorials for GMRES and DAE solving (if ready).

### 🟢 Phase 3: Medium-Impact Improvements (v1.5 and v2.0 Completion)
- [x] **Implement v1.5 Ship-Blocking Items**:
  - Band LU pivoting.
  - Newton convergence-rate monitoring.
  - Dense output (`CVodeGetDky`).
- [x] **Add `no_std` Support**.
- [x] **Add PyO3 Bindings**.
- [x] **Add Sparse Matrix Support**.
- [x] **Add WebAssembly Target**.

### 🔵 Phase 4: Long-Term Enhancements
- [ ] **Parallel-in-Time (PinT) Orchestrator** (research).
- [ ] **GPU Tensor Core Support** (document current state).
- [ ] **CVODES (Sensitivity Analysis)**.
- [ ] **Publish Academic Papers** (NeurIPS / Nature Computational Science).
- [ ] **Case Studies** for Industry Partnerships.

### 🟣 Phase 5: Serverless Autoresearch (v8.0)
- [x] **Deploy Mission Control Interface**: Launch the React-based frontend for Autoresearch orchestration.
- [x] **Serverless Auto-Research**: Transition the auto-research engine to GCP Serverless (Vertex AI + Cloud Run).
- [x] **Autonomous Peer Review Validation**: Generate physics and lean verification for xMHD paradigms.
- [x] **Discover New Integrators**: Publish `HamiltonianGraphAttentionIntegrator` (500x speedup).
