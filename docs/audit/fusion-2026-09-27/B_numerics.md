# B: Numerical audit of the rusty-SUNDIALS fusion / ITER-disruption code

- **Worktree:** `/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-fusion-audit`, branch `audit/fusion-2026-09-27`, HEAD `af4886f`.
- **Changes:** no tracked file was edited or committed. Every tracked file that a run modified was copied out first and then restored with `git checkout --`.
- **Hardware:** CPU only.
- **Run artefacts:** `B_runs/` (all paths below are relative to `.../fusion_audit/`).
- **Audit-only code** (not part of the repo):
  - `B_runs/controls/src/main.rs`: the control experiments. Final output is `B_runs/controls_run4.txt`; `controls_run3.txt` holds the R×2 failure at rtol 1e-8. Earlier attempts, which panicked on CVODE failures, are `controls_run.txt` and `controls_run2.txt`.
  - `B_runs/provenance_check.py`: compares the committed CSVs with the closed form. Output is in `B_runs/provenance_check.txt`.
- **Process limits:**
  - A worktree guard blocked `nice` and `/usr/bin/time`, so builds ran un-niced and wall times come from `date` deltas.
  - The first `iter_disruption` run was uncapped. Every later binary ran under `ulimit -v` (8 GB for the examples, 4 GB for the controls).

## 0. Where the examples live
All five examples belong to the `examples` crate (`examples/Cargo.toml`), registered as `[[example]]` at lines 95-101, 127-129 and 147-153. Run them with `cargo run --release -p examples --example <name>`. The build succeeded (rc=0, 15.2 s) with 9 warnings, all unused imports or variables. Tellingly, one of them is the unused `y` inside the iter_disruption right-hand side.

## 1. Runs
Every binary ran from the worktree root (W). Exact commands:
- **Build:** `cargo build --release -p examples --example iter_disruption --example iter_disruption_3d --example fusion_mhd_benchmark --example fusion_sciml_phase5 --example exp5_fogno_xmhd`, logged to `B_runs/build_full.log`.
- **Each example:** `( ulimit -v 8000000; timeout 900 target/release/examples/<name> )`. Output goes to `B_runs/<name>.run1.txt` and `.run2.txt`. The capped iter_disruption run is `iter_disruption.run1_capped.txt`; `iter_disruption.run1.txt` is the uncapped run.
  - **Incident:** the uncapped iter_disruption run lasted 214.6 s before the helper agent SIGKILLed it (rc=137). It was on a shared box. dmesg/journalctl showed no OOM line, and `free -g` afterwards showed 26 GB available.
- **3D negative control:** `RUSTY_SUNDIALS_ARCHITECTURE=FNO RUSTY_SUNDIALS_GPU_ABLATION=0 RUSTY_SUNDIALS_ADAPTIVE_PRECISION=0 target/release/examples/iter_disruption_3d`, output to `iter_disruption_3d.negctrl_FNO.txt`. The md5 lists are `md5_3d_default_committed.txt` and `md5_3d_negctrl_FNO.txt`.
- **Python:**
  - `MPLBACKEND=Agg CUDA_VISIBLE_DEVICES= python3 scripts/iter_disruption_viz.py` and `python3 scripts/fusion_data_integration.py`.
  - `MPLBACKEND=Agg python3 -s scripts/iter_disruption_3d_viz.py`.
  - The numpy loader one-liner is in `numpy_grid_check_rustgen.txt`.
  - Output lands in `B_runs/<script>.txt`.
- **Controls:** `cargo build --release --manifest-path B_runs/controls/Cargo.toml --target-dir B_runs/controls/target`, then `( ulimit -v 4000000; timeout 600 B_runs/controls/target/release/fusion_controls )`, output to `B_runs/controls_run4.txt`.
- **Provenance:** `python3 B_runs/provenance_check.py`, output to `provenance_check.txt`.
- **Tests:** `cargo test --release -p examples | -p sundials-core | -p cvode`, output to `B_runs/test_*.txt`.
- **Worktree state at the end:** `git status` shows only `?? docs/audit/`. That directory was created at 15:44 by neither this agent nor its helper, and it was left untouched.

| Example | Exit code | Wall time | Key output (verbatim) | Two runs identical? |
|---|---|---|---|---|
| iter_disruption | **137** when uncapped (SIGKILL at 214.6 s); **134** under the 8 GB cap (2.65 s) | – | `[Solver Setup] Explicitly bypassing dense Jacobian memory allocation.` … `[Data] Saved …iter_state_t0.00.csv` then `memory allocation of 1344000 bytes failed`, core dump | Yes: the t=0 CSV is byte-identical. **It never reaches t=0.3.** |
| iter_disruption_3d | 0 | 98.9 s / 96.3 s | `Grid: 100×200×16 = 320000 plasma DOF`, `Total system DOF: 672000`, `Offloading SpMV to H100 Tensor Cores (Target: 157x speedup vs CPU)`, `Newton residual proxy ~2.2e-1 -> … FP8 (E4M3)` | Only the timing lines differ. All 119 CSVs are byte-identical to the committed ones. |
| fusion_mhd_benchmark | 0 | 1.03 s | `Vanilla time: 1.002s  SciML time: 0.021s  Speedup: 46.7x`, `✅ MATHEMATICALLY VERIFIED 47x SPEEDUP` | Run 2 gave 46.0x. |
| fusion_sciml_phase5 | 0 | 1.11 s | `Speedup: 145x (Stiffness completely bypassed). Time: 155.441µs`, `FGMRES converged in 3 iterations (Vanilla AMG takes 5,000+)`, `Concurrent execution complete in 401.6ms` | Only the timing lines differ. |
| exp5_fogno_xmhd | 0 | about 0.01 s | `Baseline iters: 4`, `FoGNO iters: 1`, `Target < 3 iterations: ✓`, `Speedup vs Baseline: 2.2x` | Iteration counts identical; the speedup was 2.0x on run 2. |
| exp1…exp4, tearing_mode_hero_test (context only) | all 0 | 0.02–2.3 s | exp3: `FLAGNO aligned: 6933 RHS evals` vs `Isotropic baseline: 4628`, `Iteration reduction: -49.8%`, `Speedup: 0.9×`, then `✓ FLAGNO EXPERIMENT VALIDATED`. exp4: `Improvement Δ\|θ\| = -0.12448 rad` (worse), then `✓ … VALIDATED`. tearing_mode: island width W = 1/(2.5·step), hard-coded | – |

Python scripts (system `python3`; `.venv` is absent):

| Script | Result |
|---|---|
| iter_disruption_viz.py | rc=1, `No module named 'pyvista'`. A numpy-only replica of its loader, run on the CSV the Rust code produces, fails with `IndexError: index 180 is out of bounds for axis 1 with size 180`: the script's grid is 80×180, but the Rust output is 200×400. On the committed (old-grid) CSV it loads fine. |
| iter_disruption_3d_viz.py | Needed `python3 -s` to run; rc=0, 32 s, wrote 5 PNGs. The Rust data is loaded and then **never used**. |
| iter_disruption_3d_video*.py | Skipped: ffmpeg is absent, and the script downloads an unsplash stock photo. |
| fusion_data_integration.py | rc=0. imas, freegs and plasmapy are all missing, so it falls back to hard-coded parameters: v_A 8.18e6 m/s, η 1e-8 Ω·m, τ_R 502.65 s, τ_A 2.44e-7 s, S 2.06e9, γ_tearing 10.57 s⁻¹. The only change it makes to the committed CSV is CRLF line endings. |

## 2. What the code actually solves (file:line)

| Target | Equations actually coded | Solver | Verdict |
|---|---|---|---|
| iter_disruption.rs | **No MHD.** The right-hand side (99-132) is the analytic time-derivative of a *prescribed* trajectory (comment at 112):<br>• Te = Te_b·e^{-3t}(1+(0.05+0.35t)·island) + Te0·0.15·t·edge<br>• j = j_b(1-0.6t)(1+0.4t·redist)<br>• j_vessel ∝ d/dt[4t e^{-2t}]<br>**`y` is never read**, so ∂f/∂y ≡ 0 and CVODE is only doing quadrature. Time t∈[0,1] is dimensionless. There are no L, R, η or Ohm's law terms. | cvode BDF with default rtol 1e-4 (builder.rs:29). n=168,000. solver.rs:324 and :347 allocate two dense n×n matrices (about 225 GB each) and loop a finite-difference Jacobian over all columns (332-343). | Cannot run. A toy model at best. |
| iter_disruption.rs status text | Prints "Explicitly bypassing dense Jacobian" (134), "AI-preconditioned FGMRES (FLAGNO)" (135-137), "Offloading to Tensor Cores / FP8 … Preconditioner" (151-154) and "traversed extreme gradients successfully" (210). The linear-solver option is commented out (141). The builder has no linear-solver option at all, and the band/diag modules under cvode `generated/` are not wired in. | – | Status messages describe components that do not exist. |
| iter_disruption_3d.rs | "Bypass dense CVODE solve to avoid OOM. Evaluate analytically." (163-182). `Cvode` is imported but never constructed. Vessel values are computed but never written out. | none | Closed-form evaluation labelled "3D Toroidal simulation". |
| iter_disruption_3d.rs environment knobs | GPU_ABLATION, ADAPTIVE_PRECISION and ARCHITECTURE (108-110) only select `println!` strings (117, 128, 147-160). The "precision" printed is derived from `exp(-5t)`. The closing banner is hard-coded to "GPU Ablation=ON, AdaptivePrec=ON" (249-250). | – | Measured: flipping all three flags leaves all 119 CSV md5 sums **identical** (`md5_3d_negctrl_FNO.txt`), and the banner still says ON. |
| fusion_mhd_benchmark.rs | ẏ=−y with n=10 (21-28). Both paths run the identical CVODE call. The only difference is `thread::sleep(100ms)` (50) versus `sleep(2ms)` (81). The "zero loss" assertion compares the final time t, not the state (109-113). The pass band is 10-55× (116). | cvode BDF | The speedup comes from the sleeps. Control C5 below. |
| fusion_sciml_phase5.rs | Many numbers are string literals:<br>• `145x` (35)<br>• FGMRES residuals 1.0e0 / 3.4e-4 / 1.2e-9 and "3 iterations" (71-76)<br>• "99.9%" (55)<br>The remaining activity is sleeps of 150/200/50/300 ms plus the tokio 400/350 ms pair (14, 45, 52, 65, 89, 95). The only real computation is a 10-DOF Adams solve. | trivial | Hard-coded metrics. |
| exp5_fogno_xmhd.rs | A is **diagonal** with entries 1e6/1 (25-30). "FoGNO" (sundials-core/src/fogno.rs:27-32) is v·w^α with the weights set by hand to the exact inverse (61-65). α is changed to 1 "for perfect preconditioning to hit <3 iters!" (70-72). There is no graph, no neural network and no fractional operator. | real GMRES | 1 iteration by construction. Control C6 below. |
| iter_disruption_viz.py | Loader arrays are 80×180 (50-74) and 8×200 (110-140), so they do not fit the current Rust output (IndexError, see section 1). A missing CSV silently becomes Te=2 eV (75-77). The 3-D torus (151-191) is closed form, ignores the Rust output and drops the edge term. Captions say "JOREK-style MHD \| CVODE BDF-5 \| FLAGNO O(1)" (236). | – | – |
| iter_disruption_3d_viz.py | `data` (69, 143) is never used. Every panel recomputes the closed form (102-104, 156-165, 232-238, 304-306). | – | – |
| iter_disruption_3d_video_torus.py | Mesh warp is "Amplified for extreme visibility" (79-85). The flow arrows are invented (96-110). The background is an unsplash photo (18-21). | – | Illustration only. |
| fusion_data_integration.py | • FreeGS: 6 arbitrary coils, no constraints; a failed solve is swallowed with "Continuing with partial solution" (104-149).<br>• Te, ne and j are fixed parabolas, **not derived from ψ** (171-175).<br>• η=1e-8 Ω·m is hard-coded (274).<br>• Output `iter_plasma_constants.rs` is claimed to feed two examples (220-221), but grep finds no consumer. | – | Parameters are dead ends. |

Physical content of the prescribed trajectory (measured by control C1):
- **Core temperature barely drops.** Core Te goes 24995 → 10162 → 5577 → 1244 eV at t = 0, 0.3, 0.5, 1. A real thermal quench collapses to about 1–10 eV, so the 2 eV clip never binds.
- **The edge heats during the "quench".** Te at r=0.89 rises from 1081 to 3249 eV.
- **Current quench runs on the same timescale.** Σj(t=1)/Σj(0) = 0.4245, a linear decay on the same clock as the thermal quench.
- **The vessel pulse is not coupled to the plasma current.** It is a fixed 3.3e5·4t·e^{-2t}·pol·skin with no dependence on dI_p/dt. Its peak is 3.154e5 at t=0.5, where max pol·skin = 1.2991.
- **Plasma current is too low for ITER.** j_b = 1.2e6·(1−r²)^1.5 on a = 2 m gives I_p = 6.03 MA (circular) or 10.25 MA with κ=1.7, not 15 MA.

## 3. Controls (`B_runs/controls_run4.txt`; C1, C2, C5, C6 and P are identical in `controls_run2.txt`)

**C1: tolerance refinement.** This is a verbatim port of the iter_disruption right-hand side on a reduced grid (10×20 + 4×20, n=480), compared against its closed form.

| rtol / atol | nst | nfe | max rel err Te(t=1) | worst over all output times |
|---|---|---|---|---|
| 1e-4 / 1e-8 | 576 | 7261 | 2.093e-3 | 3.322e-4 |
| 1e-5 / 1e-8 | 496 | 6616 | 2.751e-4 | 4.335e-5 |
| 1e-6 / 1e-8 | 773 | 9785 | 3.955e-6 | 1.016e-6 |
| 1e-7 / 1e-8 | 937 | 12169 | 2.796e-6 | 4.194e-7 |
| 1e-8 / 1e-8 | **FAILED** | | `ErrTestFailure` at tout=0.7 | |
| 1e-8 / 1e-10, 1e-10 / 1e-12 | **FAILED** | | `ErrTestFailure` at tout=0.3 | |

- **Convergence stalls.**
  - The Te error falls from 2.09e-3 to 3.96e-6 as rtol goes from 1e-4 to 1e-6.
  - At rtol 1e-7 it improves only to 2.80e-6.
  - At 1e-8 the solve fails outright.
  - nst is not monotone either: 496 steps at 1e-5 against 576 at 1e-4.
  - This is evidence of a solver defect in its own right.
- **At rtol ≤ 1e-8 the cvode crate cannot integrate a smooth, state-independent quadrature.** It hits "ERROR FAIL 3: local error test failed > max times" (solver.rs:699-703, MAX_ERR_TEST_FAILS=7).
- The same failure occurs in C3 at rtol=1e-10 and in C4 at rtol=1e-8.
- **Suspected cause (not verified):**
  - After an error-test failure the solver only shrinks h. Unlike LLNL CVODE, it never drops to order 1 after repeated failures.
  - On an order increase it zeroes z[q] (solver.rs:723-728) instead of building it from acor.

**C2: perturbation.**
- *Positive:* TE0 ×1.1 scales Te(t=1) by 1.100000–1.100015.
- *Negative control, and the red flag:* shifting the initial Te by +1000 eV gives dTe = +1000.000–1000.17 at *every* output time, and shifting the vessel initial current by +1e5 gives dj = 99995–99999.96. The shift never decays or grows: nothing in the "simulation" is dynamic, and the state only carries an additive constant.

**C3: L/R positive control.** dI/dt = −(R/L)·I with τ = 0.12 s.

| rtol | rel err at t=τ | rel err at t=5τ | fitted τ |
|---|---|---|---|
| 1e-4 | 2.329e-5 | 2.329e-4 | 0.119998 |
| 1e-6 | 6.502e-7 | 7.600e-6 | 0.120000 |
| 1e-8 | 4.714e-8 | 2.790e-7 | 0.120000 |
| 1e-10 | **ErrTestFailure** | | |

The solver is correct at moderate tolerances. The negative control, R×2, gives a fitted τ of 0.060000 s against 0.060000 expected, but only at rtol 1e-6. At rtol 1e-8 the stiffer R×2 problem already fails with ErrTestFailure (`controls_run3.txt`).

**C4: coupled plasma/vessel L-R circuits.** This is the model the examples ought to contain. The stiffness ratio is about 42.

| rtol | nst | peak I_v | W+Q−W₀ (relative, t=2 s) | I_p(20 ms)/I_p0 |
|---|---|---|---|---|
| 1e-4 | 27583 | 11.0975 MA at t=0.036 s | −2.557e-4 | 0.1073 |
| 1e-6 | 29933 | 11.0979 MA | −1.299e-5 | 0.1072 |
| 1e-8 | **ErrTestFailure** | | | |

- Energy balance converges with tolerance.
- The ideal-wall limit is checked numerically: with r_v = 1e-12, I_v(0.5 s) = 12.0000 MA and I_p = 9.6e-3 A, matching the analytic +(M/L_v)·I_p0 = 12.000 MA. With the finite r_v used above, the peak is 11.1 MA.
- Taking about 28k steps for a 2×2 linear system (output every 1 ms) is inefficient: `solve_normal` truncates h to land exactly on each tout (solver.rs:291-296) instead of interpolating.

**C5: fusion_mhd_benchmark without the sleeps.** "Vanilla" 6.08e-4 s versus "SciML" 6.06e-4 s, a **ratio of 1.00**. y(1) = 0.36803 versus e^{-1} = 0.36788, a relative error of 4.1e-4 at the default rtol of 1e-4. The sleeps alone predict 48.6×, which matches the example's printed 46–47×.

**C6: GMRES.**
- The example's diagonal matrix is solved without any preconditioner in 4 iterations, and in 1 with the exact inverse. This matches exp5's printed 4 and 1 exactly.
- On a non-diagonal operator (tridiagonal, conductivity contrast 1e6:1, n=1024) neither identity nor diagonal preconditioning converges: residuals 3.13e1 and 2.35e1 after 330 iterations. The "FoGNO" is just Jacobi and does not address anisotropy.

**P: reference numbers.**
- NRL Spitzer resistivity at 25 keV with lnΛ=17 gives η∥ = 2.26e-10 Ω·m (η⊥ = 4.43e-10). The script's 1e-8 is about 44× too high.
- cos(π/4) = 0.7071067812 > 0.707.
- 2000²/64² = 976.56.

## 4. Claims versus produced numbers

| Claim (source) | What the code produces | Status |
|---|---|---|
| "168K DOF MHD simulation", run `cargo run --release --example iter_disruption` (paper/manuscript_v16.md:215-216, 246) | Allocation failure right after writing the t=0 CSV. The right-hand side has no state dependence and no MHD. | **Not reproducible / false** |
| "~150× speedup" and "parity with C-SUNDIALS at 168K DOF, Sparse ILU" (manuscript_v16:156, Fig 2) | No ILU and no C comparison in the code. No generator for `fig_c_vs_rust_benchmark` / `fig_pcie_scaling_v14` / `fig_newton_convergence_v14` was found in any repo `.py` file; other file types were not searched. | **Unsupported** |
| Fig 5/6: "2D reduced-MHD proxy", "eddy currents" (manuscript_v16:174-178) | The committed 2D CSVs match the closed form to 2.0e-9 (four-decimal rounding) at t = 0, 0.4 and 1.0. CVODE at the example's rtol would differ by about 2e-3 (C1). They were also written on an 80×180 grid, while the current code uses 200×400 (changed in 620cef0). The 3D t=0.40 CSV matches the closed form to 0.0. | **The figures come from analytic evaluation, not a solve.** Measured in the same Te0 units, CVODE at rtol 1e-4 would deviate by about 2.7e-4 (C1's 2.09e-3 relative to max Te(t=1) = 3249 eV, converted to Te0), five orders of magnitude above the committed CSVs. Direct evidence: the CSVs were committed in 89d4efc, and at that commit examples/iter_disruption.rs already had `N_RHO=200, N_THETA=400` and wrote values with full-precision `{}`. So the Rust example **could not have written** the committed 80×180 files with 4-decimal values. |
| 672K DOF 3D, "157x" H100, FP8/FP16 adaptive precision (iter_disruption_3d output; 3d_viz caption line 274) | No solve, no GPU. The flags have no effect on output (md5 identical). | **False** |
| 10–50× speedup, "MATHEMATICALLY VERIFIED" (fusion_mhd_benchmark:3, 114) | 46.7×, all from sleep; 1.00× without the sleeps. | **Fabricated** |
| 145×; FGMRES 3 iterations versus AMG 5,000+ (phase5:35, 75) | String literals. | **Fabricated** |
| FLAGNO FGMRES iterations: paper Table 3 "5 (projected)", SOP JSON "6", Final Submission v2 "≤7", phase5 "3" | exp3 measures FLAGNO as **worse**: 6933 vs 4628 RHS evaluations, 0.9×, yet prints "VALIDATED". exp5 gets 1 iteration only on a diagonal matrix with the exact inverse. | **Inconsistent and unsupported** |
| Table 1: 2,448/3,133 fevals, 53/11 modes, condition number 10,000→2,003 (paper:59-67) | exp1: baseline `MaxSteps{max:200,t:0.00888}`; BDF reaches t=3000. None of the Table 1 numbers appears. Grep finds no code that produces them. | **No source** |
| Table 2: 977× (paper:137-142) | exp2 prints 31.7–1808.5× for N=64–512 with k=4. 977 is plain arithmetic (2000²/64² = 976.56); the paper's own Lean statement says 976. | **Arithmetic, not measured** |
| Table 3: 492,096 edges, B=6.37 T | No code produces it (grep). | **No source** |
| Tables 4/5: 52.5 µs / 1.1 µs, 50×, ∂/∂p = 7.87e-4 …, energy 6.39e-5 | phase5 is sleep(400/350 ms). exp4 gives FD −0.09178 versus augmented −0.09898. | **No source** |
| SOP JSON (discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json:47-75): `benchmark_monopole_suppression`, `benchmark_flagno`, `benchmark_lss_shadowing`, `benchmark_hdc_trigger`; "REPRODUCED" | No such binaries exist in any `.rs` or `.toml` (grep). Commit 9712004 does exist. | **Commands do not exist** |
| Paper intro: γ_tearing ~ 1e3 s⁻¹ (paper:23) | fusion_data_integration gives γ = 10.57 s⁻¹, with an η that is itself 44× off. | **Mismatch** |
| Lean snippets in the fusion paper | `field_alignment` (paper:244-249) needs cos(π/4) < 0.707, which is false (P), so it cannot be proved. `fp8_direction_preservation` ends in `sorry` (342). Four statements are `True := trivial` or return their hypothesis. The rest is arithmetic on the reported numbers. Nothing here was Lean-checked. | **Not evidence** |
| Paper figures in general | scripts/generate_paper_figures.py builds its curves from formulas plus `np.random` (for example fig1:51-84, AMG "mean 4750" and FLAGNO "3.2" drawn from normal distributions; fig2:95-115, island width and κ as literal arrays). | **Synthetic** |

## 5. Test suite
- `cargo test --release -p examples`: **0 tests**.
- `cargo test --release -p sundials-core`: 95 passed, 0 failed.
- `cargo test --release -p cvode`: 8 passed, 0 failed, 1 doc-test ignored.
- A grep over `crates/` finds "fusion"/"MHD" only in comments ("diffusion", "fusion plasma"). **No test anywhere checks the fusion physics or the fusion examples.**
- The only "assertions" in the examples are:
  - fusion_mhd_benchmark:109, which compares t with t (a tautology);
  - exp5:104, a target met by construction.
- No cvode test covers tight tolerances, which is how the ErrTestFailure in C1/C3/C4 went unnoticed.

## 6. Top 10 fixes, in priority order

1. **Retract or relabel the claims.**
   - *Change:* docs/Fusion_Disruptions_Scientific_Paper.md Tables 1–5; manuscript_v16 lines 156, 174-178 and 246; the SOP JSON "REPRODUCED" verdict. Mark them unsupported until a script regenerates each number.
   - *Verify:* every table cell maps to a command plus an artefact.
2. **Remove the sleeps and literal metrics.**
   - *Change:* delete the `thread::sleep` / tokio `sleep` calls and the literal "145x", "3 iterations", "99.9%" and FGMRES residuals from fusion_mhd_benchmark.rs (50, 81, 114-117) and fusion_sciml_phase5.rs (14-97). If there is no real comparison, delete the examples.
   - *Verify:* `grep -n sleep examples/fusion_*.rs` is empty, and the speedup is ≈1 (C5).
3. **Make iter_disruption.rs a real, state-dependent model.**
   - *Change:* replace the prescribed-derivative right-hand side (99-132) with one that reads `y`: at minimum a 0-D circuit model (C4: L·dI/dt = −R(Te)·I with plasma/vessel mutual inductance) plus a Te energy equation, or a 1-D resistive diffusion equation.
   - *Verify:* C2-style shifts in the initial condition must evolve rather than persist as a constant offset; also require τ = L/R (C3) and energy balance W+Q = W₀ (C4) to within tolerance.
4. **Add a non-dense linear solver to cvode.**
   - *Change:* wire the existing band or diagonal preconditioner, or the sundials-core GMRES, into `CvodeBuilder` as a matrix-free Newton-Krylov path, replacing `DenseMat::zeros(n,n)` at solver.rs:324/347 whenever n is large. Until then, refuse n > ~5000 with a clear error.
   - *Verify:* iter_disruption completes under `ulimit -v 8000000`.
5. **Fix the cvode error-test failure at tight tolerances.**
   - *Change:* compare solver.rs:696-709 (error-test failure handling) and :723-728 (order increase that zeroes z[q]) against LLNL `cvDoErrorTest` / `cvSetEta` and the order-increase logic. The suspected differences are that LLNL drops to order 1 after MXNEF1=3 failures and builds z[q+1] from acor. This is a hypothesis and has not been verified.
   - *Verify:* add a regression test in which the C1, C3 and C4 problems pass at rtol 1e-8 and 1e-10.
6. **Delete the fabricated status text.**
   - *Change:* remove the FLAGNO/Tensor-Core/FP8/"bypassing dense Jacobian" println lines (iter_disruption.rs:134-137, 151-154, 210; iter_disruption_3d.rs:106-136, 147-160, 249-250). Rename iter_disruption_3d to `*_analytic` and state that it evaluates a closed form.
   - *Verify:* grep the output for "Tensor|FP8|FGMRES".
7. **Make the visualisations plot the data they load.**
   - *Change:* in iter_disruption_viz.py (50-51, 110) and iter_disruption_3d_viz.py (69, 143, 156-165), size arrays from the CSV header or dimensions, plot the loaded `data`, and fail when the CSV is missing (instead of Te=2 eV). Regenerate the figures and data/fusion CSVs from the fixed example, and add a manifest entry recording the commit and command.
   - *Verify:* `provenance_check.py` should show solver-level (~rtol) deviations from any closed form, and the grid should match the current code.
8. **Add real tests to the `examples` crate or `tests/`.**
   - *Change:* cover the L/R decay, the coupled-circuit energy balance, the tolerance-refinement order, and an initial-condition perturbation. Remove the t==t assertion (fusion_mhd_benchmark:109).
   - *Verify:* each test fails when the physics term is removed (negative control).
9. **Make exp5 test anisotropy honestly.**
   - *Change:* use a non-diagonal anisotropic operator (C6b), compare against the Jacobi/ILU baselines that already exist, and do not print "VALIDATED" when the target is met by construction. Also stop exp3 and exp4 from printing "VALIDATED" on a regression (−49.8%, −0.12 rad).
   - *Verify:* C6b-style runs report their iteration counts truthfully.
10. **Fix the plasma parameters.**
    - *Change:* in fusion_data_integration.py, compute Spitzer η(Te) instead of 1e-8 (274), derive profiles from ψ, and fail loudly when FreeGS does not converge (147-149). Either consume `iter_plasma_constants.rs` in the Rust models or delete it. Normalise j_b so that ∫j dA = 15 MA with κ.
    - *Verify:* η(25 keV) ≈ 2.3e-10 Ω·m and I_p = 15 MA are printed and asserted.
