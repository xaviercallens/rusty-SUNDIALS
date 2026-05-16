# TODO — rusty-SUNDIALS Post-v17 Roadmap

## Status Key
- ✅ Complete
- 🔧 In Progress
- 🔬 Auto-Research Candidate
- 🤝 Community Contribution
- 🔜 Planned

---

## Priority 1: Implementation — 3D Toroidal Extension 🔬🔧

**Goal**: Extend `iter_disruption.rs` from 2D $(ρ, θ)$ to 3D $(ρ, θ, φ)$ with toroidal mode coupling ($n=1$), producing high-fidelity 3D PyVista visualizations.

- [x] 2D proxy model (200×400 = 80K plasma DOF)
- [ ] 3D extension: add $N_φ$ toroidal slices (target: $N_φ = 16$)
  - New DOF: $200 × 400 × 16 = 1,280,000$ plasma + vessel
  - Toroidal coupling term: $n=1$ helical perturbation $\cos(mθ - nφ)$
- [ ] 3D PyVista visualization: full torus cutaway with `Te` and `j_phi` scalar fields
- [ ] 3D disruption sequence animation (temporal evolution)
- [ ] Export 3D VTK files for ParaView compatibility

**Feasibility**: HIGH — the existing `make_3d_torus()` in `iter_disruption_viz.py` already generates 3D geometry. The Rust solver simply needs a toroidal index loop and coupling term.

---

## Priority 2: GPU-Native Baseline Ablation 🔬

**Goal**: Isolate algorithmic vs. hardware speedup by benchmarking the GNN preconditioner against `cuSPARSE` ILU0 on the *same* H100 GPU.

- [ ] Implement `cuSPARSE` ILU0 baseline via Rust `cudarc` bindings
- [ ] Run identical 168K DOF proxy model on both paths
- [ ] Generate ablation chart: GNN-FP8 vs ILU0-GPU vs ILU0-CPU
- [ ] Quantify algorithmic contribution (GNN quality) vs hardware contribution (bandwidth)

**Feasibility**: MEDIUM — requires H100 GPU access on GCP. Estimated cost: ~$5-10.

---

## Priority 3: Adaptive Eisenstat-Walker Precision Forcing 🔬

**Goal**: Dynamically tighten the FP8 inner Krylov tolerance as the outer Newton iteration converges, recovering superlinear convergence near the solution.

- [ ] Implement `EisenstatWalkerForcing` trait in `crates/cvode/src/solver.rs`
- [ ] Track outer Newton residual ratio $η_k = \|F(x_k)\| / \|F(x_{k-1})\|$
- [ ] Switch from FP8 → FP16 → FP32 as $η_k$ decreases below thresholds
- [ ] Benchmark Newton iteration count reduction
- [ ] Update Lean 4 proof to cover multi-precision transitions

**Feasibility**: MEDIUM — algorithmic change is well-understood from [Eisenstat-Walker 1996].

---

## Priority 4: Alternative Neural Architectures 🔬

**Goal**: Compare GNN (MPNN) preconditioner against Fourier Neural Operators (FNO) and DeepONet.

- [ ] Implement FNO preconditioner (spectral convolutions in Fourier space)
- [ ] Implement DeepONet branch-trunk architecture
- [ ] Benchmark: convergence rate, inference latency, parameter efficiency
- [ ] Publish comparison table in manuscript update

**Feasibility**: MEDIUM — requires ML engineering effort. FNO may excel at multi-scale operators.

---

## Priority 5: Formal Verification Completion 🤝

**Goal**: Mechanize the two `sorry` markers in `proofs/NeuralFGMRES_Convergence.lean`.

- [ ] `fp8_preconditioner_stability`: Requires Mathlib bilinear form decomposition over `Matrix.mulVec`
- [ ] `fp8_indefinite_stability`: Requires Cauchy-Schwarz for operator-norm bounded perturbations
- [ ] Submit PR to Mathlib if missing lemmas are identified

**Feasibility**: LOW (for us) — best suited for Lean/Mathlib specialists. Open for community contribution.

---

## Priority 6: Hardware Portability 🔜

**Goal**: Port FP8 offloading beyond NVIDIA.

- [ ] AMD MI300X via ROCm / HIP
- [ ] Intel Gaudi 3 via SYCL / oneAPI
- [ ] Apple M-series via Metal Performance Shaders (MPS)

**Feasibility**: LOW — requires significant cross-platform GPU engineering.

---

## Auto-Research Execution Plan

The following items are tagged 🔬 and can be executed via the Mission Control auto-research loop:

1. **3D Toroidal Extension** (Priority 1) — execute NOW
2. **GPU-Native Ablation** (Priority 2) — execute when H100 is provisioned
3. **Adaptive Precision** (Priority 3) — execute after 3D extension
4. **Alternative Architectures** (Priority 4) — execute after ablation
