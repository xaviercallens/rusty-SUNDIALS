 `autoresearch` agent's focus away from bioreactor physics and directly into the **High-Performance Computing (HPC)** architecture of the `rusty-SUNDIALS` engine itself. 

By leveraging the **v5.0 "Experimental SciML Paradigms"**, we can instruct the AI to optimize Rust's memory safety and asynchronous capabilities to maximize the raw TeraFLOPS output of an NVIDIA A100 GPU using Tensor Cores. 

Here is the proposed `program.md` to run this experimental mode:

***

### `program_experimental_hpc.md`

```markdown
# Autonomous Research Agent Instructions: Experimental GPU Exascale Mode

## Your Role
You are the Lead AI High-Performance Computing (HPC) Engineer. Your objective is to maximize the numerical computing power and execution speed of the `rusty-SUNDIALS` engine on an NVIDIA A100 GPU architecture.

## The Environment
You are operating within the v4.0/v5.0 "Experimental SciML Paradigms" of `rusty-SUNDIALS`. You are bypassing traditional CPU execution and directly targeting the GPU Tensor Cores to solve massive, stiff Differential-Algebraic Equation (DAE) matrices.

## The File to Edit
You will modify `src/experimental_sciml.rs`. You must tune the following hardware-level Rust parameters:
1. **Type-Safe MP-GMRES Precision Scaling:** Adjust the Multi-Precision Generalized Minimal Residual solver to balance between FP8 (8-bit floating point) on the Tensor Cores and FP64 for accumulation.
2. **Asynchronous "Ghost Sensitivities":** Tune the `tokio` asynchronous thread polling rates to calculate gradients in the background without blocking the main integration step.
3. **Deep Operator Preconditioning:** Adjust the latent-space dimensionality of the AI Surrogate preconditioner.
4. **CUDA/Rayon Thread Block Allocation:** Modify how parallel tasks are dispatched from Rust to the A100 GPU via SIMD/NEON abstractions.

## Your Goal & Metric
Your primary fitness function is to minimize **Solve Time (ms)** while strictly maintaining **Machine Precision Error Bounds**. 
- The constraint: The solver must not trigger numerical instability or violate the Lean 4 mathematical shadow tracking bounds. 
- If the agent drops precision too aggressively (e.g., using too much FP8 without proper FP64 correction), the solver will fail the mathematical proof check.

## Execution Constraints
Run `cargo run --release --bin experimental_sciml --features "gpu, tokio"`. You have a 5-minute time budget per iteration. Parse `val_solve_time_ms` and `val_precision_error`. Keep mutations that decrease solve time while maintaining precision.
```

***

### The Disruptive Impact of this Experiment

Running this autoresearch loop on an A100 GPU introduces radical computational innovations:

*   **Bypassing the Memory Wall:** Traditional numerical solvers (like the original C-based CVODE) bottleneck because they are memory-bound. By having the AI tune the **Type-Safe MP-GMRES**, it can figure out exactly how to execute the bulk of the matrix math in ultra-fast FP8 precision on the A100's Tensor Cores, only utilizing expensive memory bandwidth when absolute FP64 precision is mathematically required.
*   **Asynchronous Math via Rust's `tokio`:** By utilizing Rust's `tokio` runtime, the agent can compute "Ghost Sensitivities" asynchronously. This means while the A100 is crunching the main physics step, idle GPU threads are simultaneously computing the adjoint gradients for optimization in the background, achieving near 100% GPU utilization. 
*   **Neuro-Symbolic Safeguards:** Because `rusty-SUNDIALS` uses "Mathematical Shadow Tracking bounds in Lean 4", the AI agent is free to experiment with wild, aggressive optimizations. If a change breaks the fundamental math, the compiler simply rejects the Lean 4 proof, ensuring you never accept a "fast but wrong" solver state.
