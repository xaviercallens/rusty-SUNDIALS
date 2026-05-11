# 🚀 Academic Performance Improvement Specification: The Next Leap

This document outlines the roadmap for the next generation of performance enhancements in **Rusty-SUNDIALS**, moving beyond traditional compiler optimizations (SIMD/Parallelism) to algorithmic innovations based on recent academic literature (2020–2026).

These improvements aim to yield an additional **10× to 50× speedup** for large-scale systems (PDEs, molecular dynamics, massive neural networks).

---

## 1. Mixed-Precision Iterative Refinement (MPIR)

### The Academic Context
Modern hardware (Apple Silicon AMX matrices, Nvidia Tensor Cores) is fundamentally optimized for low-precision matrix multiplication (FP16/FP32). Pure FP64 (double precision) operations are often artificially throttled or require significantly more energy/cycles. *Higham & Mary (2021)* and *Abdelfattah et al. (2020)* have demonstrated that you can solve $Ax = b$ to full FP64 accuracy while doing the expensive $O(N^3)$ factorization in FP32 or FP16.

### Proposed Implementation in Rusty-SUNDIALS
The bottleneck for stiff systems with dense Jacobians is the `LU_factor` step inside the Newton iteration.
- **Step 1:** Cast the Jacobian $J$ to FP32.
- **Step 2:** Perform LU factorization in FP32 (taking advantage of Apple AMX hardware via the Accelerate framework).
- **Step 3:** Use the FP32 factors as a preconditioner for GMRES running in FP64, or use classic iterative refinement: $r = b - A x_0$ (in FP64), solve $A \Delta x = r$ (using FP32 LU), $x_1 = x_0 + \Delta x$.
- **Expected Speedup:** **3× to 5×** for dense linear solves ($N > 1000$).

---

## 2. Exponential Integrators (EPIRK)

### The Academic Context
For highly stiff, semilinear PDEs (e.g., reaction-diffusion equations where $y' = Ay + N(y)$), traditional BDF methods struggle because the nonlinear solver (Newton) must invert $I - \gamma J$ at every step. *Gaudreault et al. (2018)* and *Luan & Ostermann (2014)* pioneered Exponential Integrators, which compute the exact solution of the linear part using the matrix exponential $e^{tA}$.

### Proposed Implementation in Rusty-SUNDIALS
- **Mechanism:** Instead of solving linear systems, we compute the action of the matrix exponential on a vector $e^{tA}v$.
- **Algorithmic Shift:** We will implement the **Krylov subspace method for matrix exponentials** (similar to the `expmv` algorithm by Al-Mohy and Higham).
- **Why it matters:** EPIRK methods completely eliminate the Newton iteration step. They are explicit methods that possess the stability properties of implicit methods.
- **Expected Speedup:** **10× to 20×** for stiff PDEs like the 2D Brusselator or Navier-Stokes.

---

## 3. Physics-Informed Neural Network (PINN) Augmented Initial Guesses

### The Academic Context
In implicit BDF steps, the Newton solver requires an initial guess for the next state $y_{n+1}$. Currently, CVODE uses a Taylor expansion (predictor polynomial) based on the Nordsieck history array. Recent work by *Raissi et al. (2019)* and *Karniadakis (2021)* on Neural ODEs suggests that a lightweight neural network can learn the flow map of the dynamical system on-the-fly.

### Proposed Implementation in Rusty-SUNDIALS
- **Mechanism:** Train a very small, online Neural Network (e.g., 2 hidden layers, 32 neurons) running asynchronously on the Apple Neural Engine (ANE).
- **Integration:** The ANE predicts the initial guess for the Newton solver. If the prediction is highly accurate, the Newton solver converges in **0 or 1 iterations** instead of the usual 3 to 5.
- **Safety:** Because we still pass the guess through the rigorous Newton iteration, **the final solution remains mathematically exact and formally verified.** We only use AI to guess the answer, not to prove it.
- **Expected Speedup:** **2× to 3×** reduction in RHS evaluations for highly chaotic systems (like 3-body or turbulence).

---

## 4. Jacobian-Free Newton Krylov (JFNK) with Automatic Differentiation

### The Academic Context
Currently, Rusty-SUNDIALS approximates the Jacobian-vector product $Jv$ using finite differences: $Jv \approx \frac{f(y + \epsilon v) - f(y)}{\epsilon}$. This is prone to round-off error and requires careful tuning of $\epsilon$. *Revels et al. (2016)* (ForwardDiff.jl) proved that dual numbers can compute this exact derivative with zero truncation error and minimal overhead.

### Proposed Implementation in Rusty-SUNDIALS
- **Mechanism:** Introduce a `Dual<f64>` type for Forward-Mode Automatic Differentiation.
- **Integration:** Users write their RHS function generically over `T: Real`. The GMRES solver passes `Dual` numbers into the RHS function to extract the exact $Jv$ product in a single pass.
- **Expected Speedup:** Eliminates numerical instability in Krylov methods, allowing for **2× larger time steps** and flawless Newton convergence.

---

## Summary and Roadmap

1. **Phase 1 (v0.2):** Implement Automatic Differentiation (Dual numbers) for JFNK.
2. **Phase 2 (v0.3):** Integrate Apple AMX (Accelerate) for Mixed-Precision Iterative Refinement.
3. **Phase 3 (v0.4):** Introduce the `epirk` solver module for matrix-exponential PDE integration.
4. **Phase 4 (v1.0):** Experimental Apple Neural Engine integration for Newton predictor.

*By integrating modern numerical analysis and AI-driven predictive techniques, Rusty-SUNDIALS will push the absolute theoretical limits of Apple Silicon hardware.*
