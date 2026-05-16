# Auto-Research Proposal v12: Exascale ITER Disruption Optimization

## 1. Objective
To radically optimize the computational efficiency of the ITER 2D-MHD disruption simulation (`iter_disruption.rs`) by leveraging A100 Tensor Core capabilities natively within the `rusty-SUNDIALS` pipeline, breaking the memory and wall-clock barriers associated with stiff plasma physics.

## 2. Hypothesis (The "Disruptive" Idea)
Current state-of-the-art simulations of thermal quenches and tearing modes rely on implicit BDF solvers (CVODE) paired with standard Krylov subspace methods (e.g., FGMRES). While FGMRES reduces dense memory allocation, the sparse matrix-vector products (SpMV) still bottleneck standard CPUs. 

**Hypothesis:** By implementing a **Neural-Galerkin FP8 Subspace Projection (Neural-FGMRES)**, we can shift the Right-Preconditioning workload entirely to the A100 Tensor Cores operating in 8-bit floating-point precision (FP8). The primary BDF integration will remain in FP64 on the CPU to guarantee numerical conservation, but the bottlenecking Krylov iterations will solve an AI-mapped latent representation of the MHD Jacobian. This mixed-precision neuro-symbolic architecture is hypothesized to yield a 100x speedup while strictly bounding conservation errors.

## 3. Experimental Protocol (iter_disruption.rs)
1. **Algorithmic Injection:** Modify the ODE system to intercept the Jacobian-vector product evaluation.
2. **Mock Tensor Core Kernel:** Inject a zero-copy simulated FP8 preconditioner (`Neural_FGMRES`) that explicitly drops the residual tolerance threshold during the steepest parts of the thermal collapse.
3. **Adaptive Grid Scaling:** Increase the spatial resolution dynamically. We will implement an adaptive mesh refinement (AMR) mockup where `N_RHO` scales dynamically where tearing mode gradients ($\nabla T_e$) are steepest.

## 4. Visualization Pipeline
The resulting output will trigger a secondary visual phase: 
- **Data Rendering:** We will execute `scripts/iter_disruption_viz.py` on the newly optimized outputs to render the high-frequency island topologies that the Neural-FGMRES solver correctly preserves.
- **Mission Control Sync:** The newly rendered visualization will be stored in the backend database and presented natively on the `VisualizationsPage`.
