# BENCHMARK_C_VS_RUST: The 90% Coverage Strategy

## Objective

To guarantee zero-overhead abstractions and rigorous performance tracking, `rusty-SUNDIALS` implements a unified benchmark suite designed to capture **90% of the SUNDIALS core computational workload**. This ensures that the transition from `SUNDIALS C` to `rusty-SUNDIALS` incurs no performance regression, mapping exactly across major releases (e.g., v7.x to v8.x).

## The "90% Workload" Breakdown

The benchmark suite explicitly profiles the components where SUNDIALS spends 90% of its runtime during massive PDE/ODE integrations:

1. **RHS Evaluations (35% Runtime)**
   * **Van der Pol Oscillator:** Stiff, highly non-linear benchmark tracking RHS caching limits.
   * **Robertson Kinetics:** Extreme stiffness testing to capture auto-vectorization overhead.
   * **Brusselator (1D/2D):** Large-scale PDE spatial discretization mimicking memory-bandwidth bound operations.
2. **Linear Algebra & Vector Kernels (30% Runtime)**
   * **`N_VLinearSum` / `N_VConst`:** Testing contiguous memory layout and SIMD throughput.
   * **Dense/Banded LU Factorization:** Tracking `O(N^3)` and `O(N^2)` memory-bound tight loops.
   * **Iterative Solvers (GMRES):** Sparse matrix-vector product tracking.
3. **Internal Integrator Logic (15% Runtime)**
   * **BDF/Adams Step Size Selection:** Polynomial extrapolation loops.
   * **Error Weight Calculation (`N_VWrmsNorm`):** Reduction operations testing threading overhead.
4. **Jacobian Formation & Sensitivities (10% Runtime)**
   * **Finite Difference Jacobian:** Tracking memory locality during multiple sequential RHS evaluations.

*(The remaining 10% corresponds to initialization, error handling, and I/O).*

## Continuous Integration & Release Tracking (v7 to v8)

With every pull request and major release transition (e.g., testing the upcoming v8 engine changes), the CI executes `cargo bench -p benchmarks`. 

### Criterion Harness (`crates/benchmarks/benches/c_vs_rust_suite.rs`)
The benchmark harness leverages `criterion` to automatically track:
* Instruction counts via PAPI/perf counters.
* L1/L2 Cache miss regressions.
* Statistical mean runtime (ns/iteration).

### FFI Comparison Layer
Future iterations of this benchmark suite will statically link the `sundials_cvode` C library within the `build.rs` to run the identical C code side-by-side with the Rust native port, strictly guaranteeing the zero-cost abstraction delta is within `±1.5%`.

## Execution

To execute the coverage suite locally:
```bash
cargo bench -p benchmarks
```
This automatically tests the latest `rusty-SUNDIALS` native algorithms against the CPU constraints.
