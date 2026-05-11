# Performance Improvement Specification: Next-Gen Rusty-SUNDIALS

## 1. Vision & Context
The original SUNDIALS was designed for supercomputers utilizing MPI and highly optimized Fortran/C BLAS libraries. With hardware advancements in the 2020s, a modern powerful Mac (e.g., Apple Silicon M-series) or a modern GPU possesses the computational throughput of a 1970s supercomputer cluster. 

This specification outlines how `rusty-SUNDIALS` will leverage modern Rust features—Zero-Cost Abstractions, Fearless Concurrency, Portable SIMD, and GPU compute—to achieve unparalleled ODE integration performance for massive systems (e.g., 3D PDE method-of-lines discretizations).

## 2. Architectural Roadmap

### Phase 1: Vectorized `N_Vector` (SIMD)
* **Objective**: Accelerate all core vector operations (linear combinations, norms, pointwise multiplication) used in the Nordsieck array and error weight calculations.
* **Implementation**: Replace `SerialVector` backing arrays with portable SIMD data structures (`std::simd` or `wide` crate). 
* **Impact**: 4x to 8x speedup on pointwise operations utilizing NEON (Apple Silicon) or AVX-512 (x86_64).

### Phase 2: Fearless Concurrency (`rayon`)
* **Objective**: Parallelize the RHS function evaluations and finite-difference Jacobian assembly.
* **Implementation**: 
  * Introduce a `ParallelVector` implementing the `N_Vector` trait.
  * Use `rayon` to chunk vector operations and iterate over them using `par_iter()`.
  * Parallelize the column-by-column Jacobian generation: `J.cols.par_iter_mut().enumerate().map(...)`.
* **Impact**: Near linear scaling with CPU core count for massive state spaces ($N > 10,000$).

### Phase 3: Advanced Linear Algebra (`faer` / BLAS)
* **Objective**: Replace the current naive `dense_getrf` and `dense_getrs` with state-of-the-art linear algebra backends.
* **Implementation**: Integrate the `faer` crate (a pure Rust, highly optimized linear algebra library that matches OpenBLAS/MKL performance) for LU factorization and back-substitution.
* **Impact**: Drastic reduction in matrix inversion times for $O(N^3)$ dense systems.

### Phase 4: Massively Parallel GPU Acceleration (`wgpu` / Metal / CUDA)
* **Objective**: Offload the entire iterative solver and RHS evaluations to the GPU.
* **Implementation**: 
  * Create a `GpuVector` trait implementation mapping to `wgpu` buffers.
  * Write WGSL (WebGPU Shading Language) compute shaders for the Nordsieck updates, $M \Delta = b$ iterative solves (e.g., GMRES/Krylov), and error bounds.
  * This allows the solver to run natively on Apple Metal, Vulkan, and DirectX 12.
* **Impact**: Capability to solve $N = 1,000,000+$ coupled systems (like massive 3D fluid dynamics or neuronal networks) in real-time.

## 3. Formal Verification Extensions
To maintain the "axiomatic" safety guarantee during parallelization:
* The `N_Vector` abstract memory model in Lean 4 has been extended (`proofs/lean4/nvector_parallel.lean`) to include associative guarantees for parallel reduction operations (e.g., parallel dot products).
* Data-race freedom is inherently guaranteed by Rust's `Send` and `Sync` traits, which map to disjoint memory separation logic axioms in the Lean 4 formal specification.
