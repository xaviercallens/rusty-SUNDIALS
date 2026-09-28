# Fusion / ITER paper-claims audit — rusty-SUNDIALS

Worktree: `/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-fusion-audit` (branch `audit/fusion-2026-09-27`), read-only.
Auditor date: 2026-09-27. Paths below are relative to the worktree root.

**How this was done.** Bash was refused in this session: a PreToolUse hook pins it to an unrelated AutoevolveAI worktree. So the evidence comes from direct `Read` of named files, plus a repo-wide grep sweep that a helper agent ran (§8). I did not compile or run anything. When I say a number "is not produced by X", that comes from reading the source of X, not from running it.
Verdict key: SUPPORTED, UNSUPPORTED (no producing code or output found), CONTRADICTED (the repo's own code or data says otherwise), FABRICATION-PATTERN (hardcoded, printed, `sleep`-simulated, or a scripted formula presented as a measurement).

---

## 0. Headline

1. **The "ITER disruption simulation" contains no physics.** `examples/iter_disruption.rs:99-132` gives CVODE a right-hand side that depends only on `t`. The `y` argument is never read, so CVODE just integrates a hand-written closed-form trajectory (Te·e^{-3t}, j·(1-0.6t), and so on). I checked one point against the formula. `data/fusion/rust_sim_output/iter_state_t0.40.csv:2` has Te = 7528.3494 and j = 911863.2. The formula gives 25000·(1-0.01²)²·e^{-1.2} = 7528.3 and 1.2e6·(1-0.01²)^1.5·0.76 = 911,863. They match exactly. Because ∂f/∂y = 0, the Jacobian is zero and there is no linear system for any FGMRES, GNN, FP8 or "FLAGNO" solver to speed up. Lines 134-154 only *print* "Injecting AI-preconditioned FGMRES… Offloading to Tensor Cores". The FGMRES builder call is commented out (line 141).
2. **The 3D "672K DOF" run solves nothing.** `examples/iter_disruption_3d.rs:163-182` writes out the closed form directly ("Bypass dense CVODE solve… Evaluate analytically"). The "Newton residual proxy → FP8/FP16/FP32" messages come from `exp(-5t)` (lines 148-160). The run prints "Offloading SpMV to H100 Tensor Cores (Target: 157x speedup)" (line 128).
3. **The H100 / ~150× / 0.9 ms / 2.5 GPU-hour numbers are hardcoded literals.** They sit in `backend/main.py:159-186` and `:292-316`. The endpoint docstrings say "Simulate GPU ablation…" (`:158`, `:190`, `:241`), yet the payloads carry `"status": "completed"` and are saved as auto-research results. Manuscript v17 reports these values as measured (`paper/manuscript_v17.md:19, 96-99, 162, 169, 196`). The "Round 2" peer review praises them (`paper/Fusion Iter - PEER REVIEW REPORT – ROUNd 1 .md:17, 46, 51, 67`).
4. **Nothing in the repo produces the fusion-paper tables (D1-D4).** The four Rust experiments whose names match the paper's sections solve unrelated toy problems (§2): a 2-variable Van der Pol, a 1-D heat equation with k=4, 2-D isotropic diffusion, and a damped pendulum. The paper names **LSODA** as the solver (`docs/Fusion_Disruptions_Scientific_Paper.md:53`). LSODA is an ODEPACK/SciPy solver, not rusty-SUNDIALS. The repo's only real SciPy plasma code (`autoresearch_agent/tearing_mode_*.py`, 1-D Harris sheet, BDF) computes other quantities. A repo-wide grep found 2,448 / 3,133 / 492,096 / 6.37 / 52.5 only in the documents (§8). Nothing stores these numbers.
5. **The paper's own B-field and tearing growth rate contradict the repo's ITER data.** The paper uses B = 6.37 T (lines 11, 209). ITER's B0 is 5.3 T, which is what `data/fusion/iter_plasma_parameters.csv:2` and `scripts/fusion_data_integration.py:92` use. The paper gives γ_tearing ~ 10³ s⁻¹ (line 23); the repo's CSV has 10.57 s⁻¹ (line 10).
6. **The Lean blocks prove nothing, and one states a false theorem.** They contain `sorry` (paper:342) and `True := trivial` (paper:254, :332). `field_alignment` (paper:244-249) rests on the claim `cos(π/4) < 0.707`, which is false: cos(π/4) = 0.70711 > 0.707. Its hypothesis also does not imply its conclusion. None of these blocks is checked through a real Lean gate.
8. **The SOP "reproduction log" is fabricated, and its Lean "certificates" are inconsistent.** `discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json:47-76` gives results for four `cargo run --bin benchmark_*` targets that do not exist in the repo. `proofs/lean4/fusion_sop_flagno.lean:56` and `fusion_sop_monopole.lean:97` declare axioms from which `False` follows. The Phase-3 ITER "discoveries" are literals in `autoresearch_agent/iter_phase3_serverless.py:21-70`, run behind `time.sleep(1.0)` (:79). Details in §8.
9. **Neither peer review is independent.** Both documents pair a critique with a rewritten "publication-ready" manuscript produced in the same pass. Neither checks code. The rewrites *strengthen* claims:
   - "5 (projected)" becomes "reduced … from 99 to 5" (Contrarian:118 vs paper:213).
   - "function evaluations" is relabelled "Non-linear Solver Iters" (Contrarian:78).
   - Methods are renamed (LSI² → "DEIM", Ghost → "LSS shadowing adjoints") while the timings stay byte-identical (SOP v2:53-56 vs paper:368-371).

---

## 1. Claims table — `docs/Fusion_Disruptions_Scientific_Paper.md` (the .pdf is the same text rendered; I checked pp.1-2 visually)

| # | Claim | Location | Evidence found | Verdict |
|---|---|---|---|---|
| 1 | Dynamic IMEX cuts Jacobian condition 10,000 → 2,003 (5.0×) | :11, :64-66, :385 | `examples/exp1_dynamic_imex.rs` is a 2-variable Van der Pol (μ=100, :19). The classifier result is thrown away (`let _split`, :62). Plain BDF order 1, no IMEX split, no condition number computed. The Lean at :103-108 *assumes* κ=10000 and 2003 as hypotheses. | UNSUPPORTED / FABRICATION-PATTERN (the Lean "proof" of an input constant) |
| 2 | 64 modes, 53 implicit / 11 explicit | :50, :62-63 | No 64-mode system anywhere in the Rust examples read | UNSUPPORTED |
| 3 | Function evals 2,448 (static) vs 3,133 (dynamic) | :61 | Not produced by exp1 (2 variables). Named solver is "LSODA" (:53), which is not in this crate. The helper's grep found no other source (§8). | UNSUPPORTED |
| 4 | Magnetic energy 0.442 in both runs | :67 | Not found | UNSUPPORTED |
| 5 | ω ∈ [10², 10⁶] s⁻¹, 10 ms of plasma evolution | :51-52 | exp1 integrates to t=3000 (dimensionless) | UNSUPPORTED |
| 6 | LSI²: 2000 → 64-dimensional state, 977× Jacobian compression, 30.5 MB → 32 KB | :11, :139-141 | `examples/exp2_lsi2_latent.rs` uses N∈{64..512}, k=4 (:123-124), and an exact sine basis, not an autoencoder (:77-79). The arithmetic 2000²/64² = 976.6, 4e6·8 B = 30.5 MiB, 4096·8 B = 32 KiB is self-consistent, but it is arithmetic only: no 2000→64 run exists. | UNSUPPORTED (numbers are arithmetic, not an experiment) |
| 7 | "BDF evaluations 108 / 108" | :142 | Not found | UNSUPPORTED |
| 8 | "Orthogonal neural autoencoder" | :119 | There is no trained encoder. exp2 uses an analytic sine basis. | CONTRADICTED |
| 9 | FLAGNO on a 32×32×32 ITER-scale tokamak, 32,768 cells | :11, :205-206 | `examples/exp3_flagno.rs` is **2-D** 32×32 = 1024 cells (:21-23). There is no graph and no GNN. The "FLAGNO" run simply integrates a *different* PDE with κ⊥ set to κ∥ (:114). | CONTRADICTED |
| 10 | 196,608 Cartesian vs 492,096 field-aligned edges | :207-208 | 196,608 = 32768·6, which is plausible. 492,096 has no producer in exp3 (no graph code). | UNSUPPORTED |
| 11 | B = 6.37 T, q = 1.5, R0 = 6.2 m (ITER) | :11, :209-211 | ITER B0 = 5.3 T (published), and the repo's own `data/fusion/iter_plasma_parameters.csv:2` = 5.3. No 6.37 in exp3. | CONTRADICTED (B0) |
| 12 | FGMRES 99 iterations (Cartesian) → 5 (FLAGNO, "projected") | :212-213 | No FGMRES in exp3; it counts RHS evaluations. The "5" is explicitly "projected". | UNSUPPORTED (the "5" is a projection) |
| 13 | FP8 Tensor Core precision | :214, :197 | No FP8 or GPU code in exp3. Cloud Run (the claimed platform, :11) has no Tensor Cores in the default configuration. | UNSUPPORTED |
| 14 | "Lean 4 rejection guarantee ✅" | :215, :237-241 | `fgmres_safety` concludes `r_k ≤ r_0` from the hypothesis `h_monotone : r_k ≤ r_0` (tautology). `mixed_precision_safety : True := trivial` (:252-254). | FABRICATION-PATTERN (verification-washing) |
| 15 | Ghost sensitivities: 128 states × 8 coils = 1,152-dim system | :282-284 | `examples/exp4_ghost_sensitivities.rs` is a damped pendulum, 2 states, 1 parameter κ (:31-51). 128 + 128·8 = 1152 is arithmetic only. | UNSUPPORTED |
| 16 | CPU 52.5 µs vs GPU FP8 1.1 µs, "50× async speedup" | :286-288, :388 | No GPU code. The "FP32 ghost" is `s_fp64 * (1.0 + 1e-5)` (exp4:89), run after `thread::sleep(1 ms)` to fake GPU latency (:158). 52.5/1.1 = 47.7, not 50. | FABRICATION-PATTERN |
| 17 | Tearing-mode energy 6.39e-5; per-coil sensitivities 7.87e-4 … 7.70e-4 | :290, :294-303 | exp4 has no coils and no tearing mode. The Optimal Step / sensitivity ratio is a constant ≈ 4.53 in every row, so the table looks generated by a formula. | UNSUPPORTED |
| 18 | "Checkpointing required: No" as a result | :289, :317 | Forward sensitivities never need checkpoints by construction. The Contrarian review says so too (Contrarian:16). | Mis-framed (true but not a finding) |
| 19 | exp4 validation passes | exp4:229 | `let pass_control = true;` is hardcoded. The angle test compares g with g·(1+1e-5), so it always gives 0°. | FABRICATION-PATTERN |
| 20 | Timings D1 1.02 s, D2 8.33 s, D3 6.64 s, D4 1.88 s; total 17.87 s; $0.05 | :366-373 | No run log found (§8). The same four timings reappear under *different method names* in SOP v2:53-56. | UNSUPPORTED |
| 21 | "Combined budget $0.20 of $100" | :375 | Not found | UNSUPPORTED |
| 22 | "All results are formally specified in Lean 4 … precision bounds" | :390 | Blocks are inline markdown only. :342 is `sorry`; :332 and :254 are `True := trivial`; :248 uses the false lemma `cos(π/4) < 0.707`. The paper's `FusionD1-4` namespaces were not found in any checked `.lean` file (§8). | CONTRADICTED |
| 23 | `compression_ratio`: `n*n/(k*k) = 976` | :178-180 | True with ℕ floor division; it is arithmetic on constants | Trivial |
| 24 | ITER grid needs N ~ 10⁹; χ∥/χ⊥ ~ 10⁹; v_A ~ 10⁶ m/s | :21-29 | Order-of-magnitude background. The repo's own v_A = 8.18e6 m/s (CSV:5). Literature usually quotes anisotropy as 10⁸-10¹⁰. | Plausible (not a result) |
| 25 | Tearing growth rate γ ~ 10³ s⁻¹ | :23 | Repo CSV `gamma_tearing_s1 = 10.57` (`iter_plasma_parameters.csv:10`) | CONTRADICTED (internal) |
| 26 | Deployed API `POST /fusion/{1-4}` | :405-406 | The only server on disk (`backend/main.py`) has no `/fusion/` route. Result of the helper's route grep: §8. | UNSUPPORTED |
| 27 | Refs [1]-[5] | :396-400 | Real works (Hindmarsh 2005 TOMS 31(3); Kennedy & Carpenter 2003 ANM 44(1-2), whose real title is "Additive Runge-Kutta schemes for convection-diffusion-reaction equations"; Jardin 2010; Li et al. 2020, which appeared as an arXiv/ICLR-workshop paper, not a main-track ICLR paper; Griewank & Walther 2008). None of them supports a numeric claim in the paper. | Real, but decorative |
| 28 | Ref [6] Halpern F.D. et al., "ITER MHD stability with resistive wall and plasma rotation", Nucl. Fusion 2021 | :401 | No volume, pages or DOI. An alphaXiv search (Halpern / ITER / resistive wall) returned no such paper. The author is known for edge-turbulence (GBS) work, not ITER RWM. | SUSPECTED FABRICATED CITATION (not verified) |

### Arithmetic checks that do hold
31.25× (2000/64); 30.5 MiB and 32 KiB; 1152; the cost column sum 0.000298 + 0.05 = $0.0503; the timing sum 17.87 s. The paper is internally consistent. It just has no generating experiment.

---

## 2. Code-to-claim mapping (what each example actually does)

| File | What it computes | Hardcoded / simulated parts |
|---|---|---|
| `examples/exp1_dynamic_imex.rs` | Van der Pol μ=100 (the doc says μ=1000 at :2; the code has `MU=100`, :19); Adams(max_steps=200) vs BDF order 1 | Classifier output unused (:62). Pass criterion `energy < 1e12` (:115). |
| `examples/exp2_lsi2_latent.rs` | 1-D heat equation, exact sine-mode ODE, k=4 | "Speedup" compares a 4-dimensional diagonal ODE with an N-dimensional one. The index page advertises "54,303× speedup" (`docs/experiments_index.html:45`). |
| `examples/exp3_flagno.rs` | 2-D 32×32 diffusion; "FLAGNO" = the same solver on κ⊥:=κ∥ | Pass `diff < 1.0` (:194) although the header promises < 1e-6 (:12). This compares solutions of two different PDEs. |
| `examples/exp4_ghost_sensitivities.rs` | Pendulum forward sensitivity (the FD check is real, :120-129) | FP32 ghost = FP64·(1+1e-5) (:89); `sleep(1ms)` stands in for the GPU (:158); `pass_control = true` (:229) |
| `examples/exp5_fogno_xmhd.rs` | GMRES on a **diagonal** 1024×1024 matrix with the exact inverse as preconditioner | Comment ":70 Wait, let's just use alpha=1.0 for perfect preconditioning to hit <3 iters!": the target is tuned to pass. Called "Mock xMHD stiffness" (:24). |
| `examples/tearing_mode_hero_test.rs` | ẏ = -1000y and ẏ = -y | FGMRES residuals are `println!` literals (:70-72). `sleep(600/200/300 ms)` (:36, :60, :67). "100x faster" is printed (:90). |
| `examples/fusion_mhd_benchmark.rs` | 10-variable ẏ = -y | The baseline gets `sleep(100ms)` ×10 to "mock" finite-difference cost (:48-50). The header claims it "mathematically verifies the 10x-50x speedup" (:3). |
| `examples/fusion_sciml_phase5.rs` | 10-variable ẏ = -y | Prints "Speedup: 145x" (:35) and "memory reduced by 99.9%" (:55) with no measurement behind either |
| `examples/iter_disruption.rs` | CVODE BDF on ẏ = g(t) (independent of y), 168,000 = 2·200·400 + 20·400 states | Prints FGMRES / Tensor-Core offload (:134-154). No linear solver is configured (:141 is commented out). |
| `examples/iter_disruption_3d.rs` | No ODE solve; direct evaluation of the closed form | `cvode` imported but unused; FP8 schedule from `exp(-5t)`; "157x" target printed |
| `backend/main.py` | FastAPI server | GPU ablation, adaptive precision and MPNN/FNO/DeepONet comparison are all literal dicts or `if step<8` schedules, saved as "completed" (:156-286). `/api/benchmarks` returns constants: C = 150 ms, Rust = 142 ms, FP8 = 0.9 ms (:292-316). |
| `scripts/reproduce_v12_poc.py` | "PCIe benchmark" and "FP8 orthogonality loss" | Pure formulas: `cpu=(dof/1000)**2*0.05` (:17), `fp8_res*=0.78` then `0.95 + sin(i)*1e-4` "noise" (:44-48). Output is committed as `data/fusion/poc_output/v12_poc_results.json` and listed in v17 as "Archived JSON benchmarks" (manuscript_v17:261). |

---

## 3. Physics sanity: ITER parameters

Published ITER baseline values (ITER Physics Basis 1999; Progress in the ITER Physics Basis 2007; ITER design documents): R0 = 6.2 m, a = 2.0 m, B0 = 5.3 T, Ip = 15 MA, κ95 ≈ 1.7, δ95 ≈ 0.33, Q = 10, P_fus ≈ 500 MW. Disruption timescales: thermal quench ≈ 1-3 ms, current quench ≈ 50-150 ms (lower bound set by vessel forces, upper bound by runaway-electron avalanche).

| Quantity | Repo value (location) | Published | Verdict |
|---|---|---|---|
| R0, a, κ, δ | 6.2, 2.0, 1.7, 0.33 (`scripts/fusion_data_integration.py:90-95`, `scripts/iter_disruption_viz.py:24-27`) | same | OK |
| B0 | 5.3 T (CSV:2, rs:12); **6.37 T in the paper** (:11, :209; also repeated in Contrarian:115) | 5.3 T | Paper CONTRADICTED |
| Ip | 15 MA in scripts. `iter_disruption.rs:47` uses j0 = 1.2e6 A/m²·(1-r²)^1.5. On a circular a=2 m section that integrates to π a² j0/2.5 ≈ 6.0 MA (≈10 MA with κ=1.7). | 15 MA | Low by 1.5-2.5× (profile not normalised to Ip) |
| Te0 | 25 keV (`iter_disruption.rs:16`, CSV:4) | ~20-25 keV | OK |
| v_A | 8.18e6 m/s (CSV:5). Deuterium only, n = 1e20; this matches the manual fallback formula at `fusion_data_integration.py:319`. | ~7e6 m/s for D-T | OK (order) |
| η | 1e-8 Ω·m, hardcoded (`fusion_data_integration.py:274, :317`) | Spitzer at 25 keV, lnΛ≈17: ~2-3e-10 Ω·m | ~40× too high. S = 2.06e9 is therefore underestimated (a true S ~ 10¹⁰-10¹¹ with L = a). |
| γ_tearing | 10.57 s⁻¹ from `S^(-3/5)/τ_A` (:289). This drops Δ' and uses τ_A^{-1} S^{-3/5} rather than the FKR τ_A^{-2/5} τ_R^{-3/5} with a Δ' factor. | Order-of-magnitude only | Crude. Contradicts paper:23 (10³ s⁻¹). |
| "Source: PlasmaPy" header | `data/fusion/iter_plasma_constants.rs:4` | — | **Misleading.** The CSV has no `Ti_keV` row, which only the PlasmaPy branch emits (`:244, :297`). So the file came from the no-PlasmaPy fallback `compute_iter_parameters_manual()` (:309). |
| FreeGS equilibrium | `data/fusion/iter_equilibrium.csv` | ψ_norm must be in [0,1] | **Broken.** `psi_norm = 24.0` at ρ=-1 (row 2). The Grad-Shafranov solve evidently did not converge; the script catches the exception and "Continu[es] with partial solution" (`fusion_data_integration.py:147-149`). Te, ne and j are analytic parabolas (:173-175), not FreeGS output. |
| Thermal quench | Te ∝ e^{-3t}, t ∈ [0,1] (`iter_disruption.rs:109`). If t is in seconds, τ_TQ ≈ 333 ms. Core Te at t=1 ≈ 1.2 keV. | 1-3 ms; post-TQ Te ~ 5-20 eV | Timescale off by ~100× and the quench only reaches ~5% of initial Te. Time is never given units. |
| Edge heating during the quench | `+ TE0*0.15*t*edge_shape` adds +3.75 keV at ρ=0.85 by t=1 (:115) | Edge cools first in a TQ | Unphysical |
| Current quench | j ∝ (1-0.6t) → 40% of Ip left at t=1 (:120) | 50-150 ms to ~0 | If t is in seconds: ≥10× too slow. No L/R physics. |
| Vessel currents | 3.3e5·4t e^{-2t}·poloidal·e^{-r/0.3} (:103, :127) | — | Scripted shape; no circuit or eddy-current model |
| Q=10, 500 MW | Not used in any computation. "ITER Q=10 Burning Plasma (Digital Surrogate)" appears only as a label in `docs/Iter Autonomoua Phase 3.md:136`. | — | UNSUPPORTED label |
| Grid for figures | Rust writes 200×400 (`iter_disruption.rs:8-9`). The viz allocates 80×180 (`scripts/iter_disruption_viz.py:50-51`), then writes `Te[ir,it]` for ir ≤ 199 and it ≤ 399 (:73). NumPy raises IndexError on that. | — | The figures (hero, sequence, torus) cannot have been rendered by this script from the current CSVs. `paper/sessions/session_v11_iter_simulation.md:10` records an 80×180 grid, so the grid and the figures are from different code versions. CONTRADICTED. |

---

## 4. Citations and "real data" claims

- **Is there real experimental data on disk?** No. `data/fusion/` holds an analytic-parabola "equilibrium" CSV (with the broken ψ_norm), fallback-computed constants, the scripted-trajectory CSVs (`rust_sim_output/`, `rust_sim_output_3d/`), and the formula-generated `poc_output/v12_poc_results.json`. The helper's shot-name grep (§8) found no DIII-D / JET / MDSplus / MAST shot data. TokaMark is described in `docs/FUSION_DATASETS_SURVEY.md:13-37` with a download command (`--local-dir ./data/tokamark`, :36). `data/fusion/.gitignore` excludes `tokamark/`, and no TokaMark data or reader code was found. The survey itself proposes *adding* "§7.2 Validation Against MAST Experimental Data" (:272-273). That validation has not been done, and none of the papers claims it has.
- **Survey references.** IMAS-Python, IMAS-ParaView, FreeGS/FreeGSNKE, disruption-py, PlasmaPy, FAIR-MAST, OMAS and JOREK are real projects. TokaMark is real: arXiv 2602.10132 (IBM/UKAEA/STFC, Feb 2026), which I confirmed through alphaXiv. I could not verify the Springer DOI `10.1007/s10894-026-00549-z` (:7, :237) or the specific EUROfusion article URL. "IMAS-ParaView v2.3.0 (March 2026)" (:99) is unverified.
- **manuscript_v17 references.** [1] Hindmarsh 2005, [2] Higham & Mary 2022 (Acta Numerica 31:347-414), [3] Eisenstat & Walker 1996 (SISC 17(1):16-32), [4] Saad & Schultz 1986 and [6] Huysmans & Czarny 2007 (NF 47:659) are real. **[5] "ITER Organization, IMAS Data Dictionary and ParaView Integration Standards, ITER Technical Report, 2023" (manuscript_v17:305)** looks like an invented report title. IMAS-ParaView is a GitHub project, not a 2023 technical report.
- **Fusion paper ref [6] (Halpern 2021):** suspected fabricated (claim #28 above).
- **The Lean code in the rewrites cites Mathlib lemmas that I don't believe exist.** `divergence_curl_is_zero` and module `Mathlib.Analysis.VectorCalculus` (Contrarian:150, :161); `mps_truncation_error_bound`, `IsTensorTrainDecomposition`, `HilbertSpace` (Phase 3:115-124). These are presented as mechanised, but they would not compile.
- **Mathematical error in manuscript v17 Theorem 1** (`paper/manuscript_v17.md:122-129`; `proofs/NeuralFGMRES_Convergence.lean:64-75`). The bound |⟨v, AEv⟩| ≤ ε‖v‖² needs ‖AE‖ ≤ ε. The hypothesis only gives ‖E‖ ≤ ε, so the correct condition is ε‖A‖ < α. Theorem 2 has the same gap. As stated, both theorems are false in general, yet the paper calls them "✅ Complete" pen-and-paper proofs (v17:150-151). Both Lean bodies are `sorry` (:75, :108).

---

## 5. Other fusion documents

| Doc | Key claims | Evidence | Verdict |
|---|---|---|---|
| `docs/Iter Autonomoua Phase 3.md` | TT gyrokinetics 14.8 TB → 46.2 MB (320,000×), 412 h → 14.2 s on one L40S (:24-29); d-SPI "billiard" pellets cut wall heat flux 84.2 → 11.4 MW/m² with 98.5% radiated (:47-50); liquid-metal walls "entirely eliminating splashing" (:75); HDC control in 40 ns, 1,375× faster (:89-96); Lean TT energy bound (:115-124); €0.00021 cost (:138) | No TT, SPI, phase-field or HDC code found by name (§8). The Lean calls a non-existent lemma. Ratios are internally consistent (14.8 TB / 46.2 MB = 3.2e5; 55/0.04 = 1375), which is typical of made-up tables. It asks to file "patent drafts" and *Nature Physics* letters (:141). | FABRICATION-PATTERN (LLM-narrated "agent logs" with no execution) |
| `docs/Standard Operating Procedure (SOP)/Fusion Final Submission v2.md` | QTT, DEC/Yee, Coulomb gauge, LSS adjoints, "C∞ bump function preserves 5th-order BDF" (:20); "128³ = 16.7M DOF, FLAGNO ≤ 7 FGMRES iterations, verified O(1) weak scaling" (:33); "Mechanized Structural Truths" list (:43-47) | Same four timings as the fusion paper, now attributed to different methods (:53-56). Lean theorem names (`discrete_de_rham_exactness` etc.): §8. No 128³ run in any example read. exp1-exp4 all use `max_order(1)`, so "5th-order BDF preservation" is untested. | FABRICATION-PATTERN |
| `docs/autoresearch_v12_iter_disruption.md` | Proposal: "Mock Tensor Core Kernel … simulated FP8 preconditioner" (:13), "AMR mockup" (:14), a hypothesised 100× (:9) | Honest *as a proposal*. It documents that the FP8/Tensor-Core path was always meant to be a mock, which the later manuscripts drop. | Evidence of intent |
| `docs/verification/PAPER_EXP3_FLAGNO.md` | L2 diff 1.74e-3, ∫u ≈ 0.061 (:24-25); Lean "proof sketch" in Lean 3 syntax (`begin … end`, :13-19) | The same experiment is listed as L2 diff 1.99e-4 in `docs/experiments_index.html:51`: two different values for one run, with no log. | CONTRADICTED (inconsistent reruns, no log) |
| `paper/sessions/session_v11_iter_simulation.md` | "Validating 168,000 DOF" on "N_ρ=80, N_θ=180" (:4, :10) | 2·80·180 = 28,800 ≠ 168,000; 168,000 needs 200×400 + vessel | CONTRADICTED |
| `paper/manuscript_v17.md` | ~150× speedup, 0.9 ms, H100 FP8, 45k-parameter MPNN, 2.5 GPU-h, frozen weights over 2000 steps, C/Rust parity at 168K DOF, break-even ≈ 50 runs, GNN weights at `data/gnn_weights/` (:258) | All numbers are literals in `backend/main.py`. **`data/gnn_weights/` does not exist** (Read: "File does not exist"). The 168K "proxy model" has no linear solve (§0.1). iter_disruption writes 7 outputs, not "2000 steps" (:148). C-vs-Rust "150 vs 142 ms" are constants (main.py:296-297). | FABRICATION-PATTERN / CONTRADICTED |

---

## 6. The peer reviews

- **`docs/Contrarian Peer Review Fusion Articel`**: PART I is a critique; PART II is a full rewritten manuscript in the same file. Nothing records who or what produced it: no model id, date, reviewer identity or log. Its critiques are correct and substantive (ITER-scale fallacy, ∇·B, forward mode needs no checkpointing, IMEX discontinuity, trivial Lean). But the rewrite answers them **only in prose**:
  - "Divergence-free DF-LSI²" (Contrarian:90) has no implementation. exp2 has no B field.
  - The "temporal hysteresis penalty" (:65) is not in exp1.
  - The rewrite upgrades the "projected" 5 FGMRES iterations to a measured result (:118).
  - It keeps B0 = 6.37 T (:115).
  - It supplies a Lean proof that uses non-existent Mathlib names (:150-161).
  - The original `docs/Fusion_Disruptions_Scientific_Paper.md` was **not** changed. It still says "ITER-scale", still has `sorry` and `True := trivial`, still claims "zero checkpointing" as a result.
- **`paper/Fusion Iter - PEER REVIEW REPORT – ROUNd 1 .md`**: the filename says Round 1, but the content says "ROUND 2" (:1) and reviews manuscript **v16** (the TOMS H100 paper, :5), not the fusion paper. It takes the hardcoded 150× (:67), 0.9 ms (:51), 2.5 GPU-h (:46) and synthetic "Figure 4 Eisenstat-Walker" at face value and calls them "empirically accurate" (:67). No reviewer read code or data. The tone ("masterclass in Open Science", :17) is generated praise. The reviewer asked for a cuSPARSE ILU0 comparison (:69). v17 still calls that ablation "planned as future work" (manuscript_v17:169, :273). Yet `backend/main.py:156-186` returns it as `"status":"completed"`, with invented timings: cuSPARSE 8.3 ms, 17.1× hardware, 9.2× algorithm. The repo contradicts itself.
- **`paper/peer_review_v12_neural_fgmres.md`** (the ITER v12 review, "Reviewer #2") asks for two things: a PCIe-latency-vs-CPU-SpMV benchmark (:20-21) and an FP64-vs-FP8 FGMRES residual plot "under tearing mode (m=2/n=1) disruption conditions" (:24-25). **Both were "answered" by `scripts/reproduce_v12_poc.py`, which generates the curves from closed-form expressions** (`cpu=(dof/1000)**2*0.05`, :17; `fp8_res *= 0.78` then `*0.95 + sin(i)*1e-4`, :44-48). No solver or GPU is involved. The reviewer's Critique C (a dashboard to "trigger these exact POCs") was met by `/api/peer_review/poc`, which reruns the same formula script (`backend/main.py:322-332`). **The criticism was addressed with synthetic data presented as a benchmark.**
- **`paper/highly critical "contractory" peer review report … TOMS.md`** (renamed on this branch to `paper/highly critical contrarian peer review report tailored for ACM TOMS.md`, because `"` and `*` are invalid in Windows paths and broke the Windows CI checkout) reviews the base CVODE paper, not ITER. Its line 1 reads "Here is a formal, highly critical 'contractory' peer review report … As requested, this review assumes the role of…". That is pasted chat-assistant output, and line 4 holds a stray shell command (`paraview data/fusion/vtk_output/iter_midplane.vtk`). SOP v2:1 ("Here is the fully revised, academically rigorous manuscript…") and SOP.md:1 ("Here is the formal Standard Operating Procedure…") have the same signature. **So the reviews and the rewrites did come from real LLM chat sessions, prompted by the author to play reviewer.** They are not canned, but they are not independent either, and nothing records the model, prompt or date. A model-call grep did find genuine multi-LLM review engines: `autoresearch_agent/peer_review_v10.py` (three reviewers, median score) and `peer_review_v11.py` (Mistral "mistral-medium" plus Gemini or a local fallback). I found no output artifact linking either engine to the fusion paper or to the three review documents above. Those documents therefore do not come from these engines, as far as the repo shows. I did not audit the engines themselves.
- **Summary:** the reviews look like LLM text, not an independent reviewer. The fusion paper did not address its review. Manuscript v17 addressed its review textually (sorry-downgrade, caveats), which is good, but backed the addressed points with hardcoded data.

---

## 7. Top-10 remediation (priority order)

1. **Retract or relabel every speed, iteration or latency number that is not measured.** Covers fusion paper Tables 1-6, manuscript v17 "~150×", "0.9 ms", Fig 2/3/4/8, and "C 150 vs Rust 142 ms". *Where:* the docs listed, plus `backend/main.py:156-316`. *How:* delete the literal dicts or mark them `"status":"SIMULATED"`; drop "completed". *Verify:* `grep -n '"completed"' backend/main.py` finds no simulated endpoint, and each number in v17 has a path to a raw log.
2. **Relabel iter_disruption as a scripted-trajectory visualisation, not a "2D reduced-MHD proxy model".** Or replace it with a real RMHD RHS that depends on y (ψ-ω with η, ν, and a J×B coupling). Remove the Neural-FGMRES / Tensor-Core printlns (`examples/iter_disruption.rs:134-154`; `examples/iter_disruption_3d.rs:106-160`). *Verify:* a test that ∂f/∂y ≠ 0 (a finite-difference Jacobian probe has non-zero entries) and a linear-solver iteration count read from the solver stats.
3. **Remove `sleep`, `println!`-residuals and `pass = true` from the examples.** `tearing_mode_hero_test.rs:36-72`, `fusion_mhd_benchmark.rs:48-50`, `fusion_sciml_phase5.rs:14-55`, `exp4:89, :158, :229`, `exp5:70-71`. *Verify:* `grep -rn "sleep\|pass_control = true\|Speedup: 145x" examples/` is empty, and each experiment has a negative control that fails.
4. **Fix the physics constants in the paper.** B0 6.37 → 5.3 T (paper:11, :209; Contrarian:115). Reconcile γ_tearing (paper:23 vs CSV:10). Replace η = 1e-8 with Spitzer η(Te) (`fusion_data_integration.py:274, :317`). Give time units and ITER-consistent TQ (~1 ms) and CQ (50-150 ms) timescales. *Verify:* PlasmaPy `Spitzer_resistivity` output is logged next to the CSV.
5. **Rerun `scripts/fusion_data_integration.py` with PlasmaPy and FreeGS actually installed, and fail hard when FreeGS does not converge** (remove the "Continuing with partial solution" branch at :147-149). *Verify:* ψ_norm ∈ [0,1] in `iter_equilibrium.csv`, a `Ti_keV` row is present, and the header's "Source: PlasmaPy" is true.
6. **Lean.** First, delete the inconsistent oracle axioms (`proofs/lean4/fusion_sop_flagno.lean:56-59`, `fusion_sop_monopole.lean:97-100`) and the vacuous theorems, and withdraw the certificates CERT-FUS-* and the fabricated execution log `discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json`. Then delete the inline Lean from the fusion paper, or move it to real `.lean` files and run it through a compile + `#print axioms` gate (reject `sorryAx`). Fix the false `field_alignment` (use 0.7072, or state `> √2/2`). Fix v17 Theorems 1-2: add ‖A‖ to the hypothesis (ε‖A‖ < α), or bound ‖AE‖ directly. Stop calling them "✅ Complete". *Verify:* `lake build` passes and the axiom set is a subset of {propext, Classical.choice, Quot.sound}.
7. **Citations.** Remove or confirm Halpern 2021 (paper:401) and "ITER Technical Report 2023" (v17:305). Fix the Kennedy-Carpenter title and venue details for Li et al. 2020. Verify the Springer DOI in the survey. *Verify:* every reference has a DOI or arXiv id that resolves.
8. **Archive the Phase-3 doc and the SOP v2 as speculative fiction / idea logs.** Also delete or relabel `autoresearch_agent/iter_phase3_serverless.py` (hardcoded results + `sleep`) and `discoveries/phase3_fusion_telemetry.json`. Otherwise delete their result tables (`docs/Iter Autonomoua Phase 3.md:24-96, :134-141`; SOP v2:33, :43-57). No patent or *Nature Physics* submission should use them. *Verify:* no table in them lacks a code path.
9. **Artifact manifest honesty.** Create or remove `data/gnn_weights/` (manuscript_v17:258). Label `data/fusion/poc_output/` as synthetic (`scripts/reproduce_v12_poc.py`). Fix the viz grid mismatch (`scripts/iter_disruption_viz.py:50-51` vs `iter_disruption.rs:8-9`) and regenerate the figures from the current CSVs. *Verify:* `python3 scripts/iter_disruption_viz.py` runs clean against the fresh `cargo run --example iter_disruption` output.
10. **If an empirical claim is wanted, run a real validation against open data.** Download a TokaMark or FAIR-MAST subset and compare a *computed* 2/1 tearing growth rate against a published linear benchmark (e.g. JOREK/CASTOR3D γ(S) at R/a = 10, as the survey suggests, `FUSION_DATASETS_SURVEY.md:217-220`). Use a positive control (analytic FKR γ ∝ S^{-3/5} slope) and a negative control. Until then, the papers should state that no experimental validation exists.

---

## 8. Grep sweep results (helper agent ran find/grep; I read the hits afterwards)

- **The fusion paper's numbers appear only in the paper.** `2,448`, `3,133`, `492,096`, `6.37`, `6.39×10⁻⁵`, `7.87×10⁻⁴`, `977` and `Halpern` occur only in `docs/Fusion_Disruptions_Scientific_Paper.md`, plus copies in the SOP docs. No script, JSON or CSV produces or stores them, and there is no run log. `52.5 µs` appears only in docs (paper:286, SOP v2:37, SOP.md:141, docs/temp/SOP.md:164).
- **No code anywhere mentions LSODA, `njev` or `np.linalg.cond`** (the only "cond(" hit is inside `precond(`). So nothing in the repo computes the paper's Table 1 (function evaluations under LSODA, condition numbers).
- **Timeline:** `run_gpu_ablation` first appears in commit 627a815 (2026-05-16, "Deploy Mission Control v17"). That is after the v17 manuscript commit 3f8f83a ("addressing Round 2 peer review"), and v17 still calls the ablation future work.
- **There is no `/fusion/` API route anywhere.** Hits for the string are file paths only (`cloudbuild.yaml:19, :34`, manuscripts). Claim #26 is therefore UNSUPPORTED.
- **The paper's Lean is not in the repo.** `FusionD1-4` exist only as inline markdown in the paper.
- **No experimental shot data.** The shot-name grep matched only "screenshot" strings. `data/fusion/.gitignore` ignores `tokamark/`, `*.h5`, `*.hdf5`, and `data/fusion/` contains no TokaMark or MAST data. I did not check a top-level `data/tokamark`, but no code reads one.
- **`vtk_output/iter_midplane.vtk` does not exist.** Manuscript v13 cites it as a 389 KB artifact (manuscript_v13:246-253).
- **The PDF metadata** shows a HeadlessChrome print of `localhost:49717/Fusion_Disruptions_Scientific_Paper.md` (9 pp, 2026-05-13). The text is the same as the .md.
- **The paper was never revised after its review.** Its git history is only `434bb44` (roadmap chore) and `449781c` (copyright notice).

### Additional severe findings from the sweep

- **The SOP "execution log" references binaries that do not exist.** `discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json:47-76` records runs of `cargo run --release --bin benchmark_monopole_suppression | benchmark_flagno | benchmark_lss_shadowing | benchmark_hdc_trigger`. It gives results for each: max ∇·B 1.12e-15, 6 FGMRES iterations at κ=1e8 on 128³, 115.2 FP8 TFLOPs at 98.4% Tensor-Core utilisation, 51.8 µs / 1.18 µs, 38.5 ns. It also reports Lean "errors: 0" and a verdict of "REPRODUCED, deviance 0.00%" (:97-98).
  - No such binaries exist. The find sweep for `*flagno*` returned only `exp3_flagno.rs` and two `.lean` files. `crates/benchmarks/Cargo.toml` defines only `[[bench]] c_vs_rust_suite`, and `examples/Cargo.toml` has none of them.
  - It describes "GCP Cloud Run" hardware as instance type `g2-standard-16` (a Compute Engine machine type) with 8 vCPU (:10-18). That is internally inconsistent: g2-standard-16 has 16 vCPUs.
  - **The commit the log cites contains no benchmarks.** It gives `git_commit: 9712004`. That commit exists ("feat: add SOP reproducibility page and execution APIs"). It adds the SOP markdown, `mission-control/src/api/mockData.js` (+43 lines) and `SopPage.jsx`. `git ls-tree -r 9712004` contains no `benchmark_(monopole|flagno|lss|hdc)` files.
  - `.zenodo.json:57` advertises "Lean 4 formal proofs: sorry-free, 0 tautologies … GCP telemetry: Execution ID L4-SERV-88219-FUS".
  - **Verdict: FABRICATION-PATTERN (fabricated execution log, published in archive metadata).** This is the "reproduction" behind SOP v2 §3's "≤ 7 FGMRES iterations, verified O(1) weak scaling".
- **The Phase-3 "autoresearch" results are hardcoded, and the script only sleeps.** `autoresearch_agent/iter_phase3_serverless.py:21-70` holds the literals (320,000×, 11.4 MW/m², 98.5%, 40 ns, 1,375×). `:79 time.sleep(1.0)` is commented "Simulate API execution time". The script prints "[✔] Integration converged" (:81) and writes `discoveries/phase3_fusion_telemetry.json` with `"status":"success"` and `"protocols_verified"` (:102-108). Every number in `docs/Iter Autonomoua Phase 3.md` comes from this script. **FABRICATION-PATTERN, confirmed at the source.**
- **The fusion SOP Lean files add inconsistent axioms, so the "sorry-free certificates" can prove anything.**
  - `proofs/lean4/fusion_sop_flagno.lean:56-59`: `axiom flagno_l4_telemetry_oracle (iters_measured : ℕ) (h_exec : True) : iters_measured = 6 ∧ …`. Take iters_measured := 0 and you get 0 = 6, i.e. `False`.
  - `proofs/lean4/fusion_sop_monopole.lean:97-100`: `axiom gcp_l4_telemetry_oracle (div_B_sim) (ε) (h : True) : ∀ x, |div_B_sim x| ≤ ε`. Take ε := -1 and you get `False`, provided Ω is inhabited. The flagno axiom needs no such condition.
  - The files present these axioms as replacing `sorry` ("the formal certificate remains sorry-free", monopole:96). An inconsistent axiom is strictly worse than `sorry`. The binding rule's axiom whitelist `{propext, Classical.choice, Quot.sound}` would reject both.
  - **The theorems themselves are vacuous.** `flagno_o1_weak_scaling` (:26-33) concludes `C_iters ≤ 7` about a witness it chooses itself. `cartesian_amg_fails…` (:38-40) proves `∃ b : Bool, b = true`. `async_decoupling_correctness` (lss_hdc:26-32) proves 1.18e-6 < 5.18e-5 from hypotheses that set those values. `lss_shadowing_adjoint_horizon` (:16-22) proves `∃ δ > 0, δ ≤ exp(-λT)` by choosing δ = exp(-λT). `monopole_suppression_bound` (monopole:106-115) restates its hypothesis. `gauge_invariant_latent_bijection` (:64-78) never uses `h_coulomb`: it is `div ∘ curl = 0` after a rewrite, and that is exactly the tautology its own docstring (:60-61) says it avoids.
  - **Verdict: FABRICATION-PATTERN (verification-washing).**
- **`proofs/lean4/roadmap/v8_phase4_disruptive.lean` contains `sorry`** at :43, :65, :149 ("Verified numerically"; the :149 one is a bioreactor claim, not fusion).
- **`autoresearch_agent/phase4_disruptive.py` is not fusion work.** It is the bioreactor "Phase IV", and it also returns literal dicts (e.g. Protocol M, :80-90). Not in scope. It does show the same hardcoded-result pattern repo-wide.
- **The only genuine plasma physics code: `autoresearch_agent/tearing_mode_1d.py` and `tearing_mode_agent.py`.** Both are SciPy `solve_ivp` BDF runs of a linearised 1-D Harris-sheet RMHD (ψ, φ spectral; N=128; S ≤ 10³) with real `nfev` and energy output (`tearing_mode_1d.py:30-56, :94-115`; `tearing_mode_agent.py:112-201`). They do **not** produce the fusion paper's D1-D4 numbers: there is no LSODA, no 64 modes, no condition numbers. The model is normalised (B0=1, a=0.1), not ITER. Problems with the agent:
  - The "symplectic projection" `state *= sqrt(E0/E)` (:190) forces energy conservation by construction. The resulting "improvement ~1e14×" and "ΔE/E0 < 1e-14" in the auto-generated LaTeX (:420-442) are therefore tautological. Uniform rescaling is also not symplectic and changes the dynamics.
  - The "physics gate" trusts the LLM's own self-reported JSON flag `preserves_energy` (:280, :532).
  - `validate_projection()` **never executes the LLM-proposed `projection_code`**. It always runs the same built-in rescaling (:288-291). So every Gemini hypothesis that claims `preserves_energy: true` is logged as "✅ VALIDATED" (:540-545).
  - With no `GEMINI_API_KEY`, the agent returns a canned hypothesis (:247-255).
  - Verdict: **real computation, fake validation loop**.
- **`docs/Standard Operating Procedure (SOP)/Fusion Standard Operating Procedure (SOP).md`**:
  - It tells reviewers to `cd rusty-SUNDIALS/lean_proofs && lake build` (:73-76), `cd ../core`, download `rusty_sundials_weights_v1.safetensors` from a placeholder bucket (:102), `cargo build --features "cuda, fp8_tensor_cores, sundials_ffi, async_adjoints"` (:106), run four `benchmark_*` bins (:117-149), and POST to `/execute_step` (:194). None of these directories, features, bins or routes was found in the worktree.
  - It *pre-states* the expected outputs: 52.5 µs, 1.1 µs, 40 ns, ≤ 7 iterations, 17.8 s (:130-152, :203).
  - The "execution log" JSON then reports slightly jittered versions (51.8 µs, 1.18 µs, 38.5 ns, 6 iterations) plus "deviance 0.00%". That is the classic signature of a fabricated reproduction.
