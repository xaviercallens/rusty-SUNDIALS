# rusty-SUNDIALS: A Memory-Safe Rust Implementation of CVODE with Formally Verified Newton Convergence

**Xavier Callens**¹

¹ Independent Researcher

**Target Journal:** ACM Transactions on Mathematical Software (TOMS)
**Article Type:** Algorithm Paper with Software

---

## Abstract

We present *rusty-SUNDIALS*, a from-scratch Rust implementation of the CVODE solver from the SUNDIALS suite (v7.4.0). Our implementation achieves behavioral equivalence with the LLNL C reference on the Robertson chemical kinetics benchmark while providing compile-time memory safety guarantees absent from the C original. Through a systematic, falsification-driven auto-research methodology, we identify and correct a critical convergence rate persistence bug in the nonlinear solver, achieving Newton iteration efficiency that *exceeds* the C reference (1.40 iterations/step vs. 1.44). We formalize the C↔Rust behavioral equivalence via Lean 4 refinement proofs covering the nonlinear solver API surface. The solver is structured as a modular Rust workspace with zero `unsafe` blocks, builder-pattern configuration, and Cargo feature flags enabling experimental convergence heuristics. We discuss the architectural advantages of Rust's ownership model for ODE solver design and outline a path toward GPU acceleration via Rust's emerging CUDA ecosystem. All results are CI-validated on every commit across three platforms (Linux, macOS, Windows) with automated regression thresholds.

**ACM CCS Concepts:** Mathematics of computing → Ordinary differential equations; Software and its engineering → Software verification and validation

**Keywords:** ODE solver, BDF methods, Rust, memory safety, SUNDIALS, Newton convergence, formal verification, Lean 4

---

## 1. Introduction

The SUNDIALS suite [1] has been the *de facto* standard for solving stiff and non-stiff ordinary differential equations (ODEs) in scientific computing for over two decades. Its CVODE component implements variable-order, variable-step BDF and Adams-Moulton methods with sophisticated adaptive error control, serving as the backbone of simulations in climate science [2], systems biology [3], and chemical engineering [4].

However, the C implementation carries inherent risks common to systems-level numerical software:

1. **Memory safety**: Null pointer dereferences, buffer overflows, and use-after-free bugs are structurally possible and have historically caused silent numerical corruption [5].
2. **Concurrency hazards**: Shared mutable state in the solver's internal arrays creates data races when parallelized naïvely.
3. **API fragility**: The C API relies on opaque `void*` pointers and integer return codes, making misuse easy and silent.

Rust [6] eliminates these classes of defects at compile time through its ownership and borrowing system, while achieving C-equivalent performance through zero-cost abstractions. We demonstrate that a Rust reimplementation of CVODE can achieve *numerical equivalence* with the C reference while providing:

- **Zero `unsafe` blocks** in the solver core
- **Compile-time thread safety** via `Send + Sync` bounds
- **Builder-pattern API** preventing misconfiguration
- **Feature-gated experimental heuristics** for safe research iteration

### 1.1 Contributions

This paper makes the following contributions:

1. **A complete Rust implementation of CVODE** supporting BDF orders 1–5 and Adams-Moulton orders 1–12 with Nordsieck history arrays, adaptive step-size control, and dense direct linear solvers (§3).

2. **A falsification-driven auto-research methodology** (§4) that systematically tests convergence hypotheses against CI-validated benchmarks, transparently documenting both rejected and accepted experiments.

3. **Discovery and correction of a convergence rate persistence bug** (§5): we show that the LLNL solver persists the Newton convergence rate `cv_crate` across steps, and that resetting it per-step causes a 1.69× Newton iteration overhead. Our corrected implementation achieves 0.98× the C reference's Newton iterations.

4. **Lean 4 formal verification** (§6) of the C↔Rust behavioral equivalence for the nonlinear solver API, proving refinement at the return-code level.

5. **Architectural analysis** (§7) of Rust's suitability for numerical ODE solvers, with a concrete roadmap for GPU acceleration on NVIDIA A100 hardware.
