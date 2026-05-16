# Peer Review Report: rusty-SUNDIALS v12 (Neural-FGMRES & ITER Disruption)

**Target Venue**: *ACM Transactions on Mathematical Software (TOMS)*
**Reviewer**: Anonymous Reviewer #2 (Computational Plasma Physics Expert)
**Overall Verdict**: Major Revision

## 1. Summary
The manuscript proposes a novel mixed-precision "Neural-FGMRES" solver integrated into the SUNDIALS CVODE (BDF) framework to simulate extreme-scale ITER plasma disruptions. The authors claim that by offloading the Krylov subspace projection (SpMV operations) to A100 Tensor Cores using FP8 precision, they bypass the $O(N^3)$ dense Jacobian memory bottlenecks while maintaining strict physical conservation laws via CPU-bound FP64 BDF integration.

## 2. Major Strengths
* **Algorithmic Innovation**: The hybrid approach of preserving the implicit time-stepping backbone in FP64 while heavily compressing the Krylov preconditioner into FP8 is highly innovative.
* **Scale of Validation**: Moving beyond toy models (e.g., Robertson) to 2D Reduced-MHD (168,000 DOFs) is a necessary and impressive leap.
* **Cost Efficiency**: Demonstrating Serverless execution profiles for extreme-scale grids is a valuable contribution to the "democratization of HPC."

## 3. Critical Weaknesses & Required POCs (Proof of Concepts)

While the theoretical framework is sound, the empirical validation lacks rigorous reproducibility data regarding hardware constraints:

### Critique A: PCIe Data Transfer Overhead
The authors propose sending state vectors back and forth between the CPU (FP64 BDF) and the A100 GPU (FP8 Neural Preconditioner) at every Newton iteration. The manuscript completely ignores the latency of the PCIe bus transfer. At 168,000 DOFs, the PCIe bottleneck might negate the Tensor Core speedup entirely.
* **Requested Reproducibility POC**: An explicit benchmark comparing the PCIe transfer latency vs. the CPU-only dense SpMV execution time.

### Critique B: FP8 Subspace Orthogonality Loss
FGMRES relies heavily on maintaining an orthogonal Krylov subspace (e.g., via Arnoldi iteration). FP8 has an extremely limited dynamic range (E4M3 or E5M2). The accumulation of rounding errors in the preconditioner might cause the GMRES solver to stall and fail to converge for stiff MHD gradients.
* **Requested Reproducibility POC**: A residual convergence graph plotting the FGMRES residual norm per iteration, comparing FP64 vs FP8 preconditioners under tearing mode (m=2/n=1) disruption conditions.

### Critique C: Mission Control Reproducibility
The authors mention an "Auto-Research Protocol" but the reproducibility suite is opaque. To satisfy TOMS guidelines, the authors must provide an interactive dashboard where reviewers can trigger these exact POCs (PCIe latency, FP8 residual decay) on-demand.

## 4. Conclusion
The paper presents a disruptive paradigm for computational fusion. However, without empirical proof that the PCIe overhead and FP8 precision loss do not break the solver, it remains purely theoretical. The authors must provide an automated reproduction suite (via their "Mission Control" interface) targeting these specific critiques before acceptance.
