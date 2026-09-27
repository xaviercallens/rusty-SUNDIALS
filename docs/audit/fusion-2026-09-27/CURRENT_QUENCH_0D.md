# 0-D ITER current-quench model (`examples/iter_current_quench_0d.rs`)

- **Worktree:** `/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-fusion-audit`, branch `audit/fusion-2026-09-27`.
- **Purpose:** replace the prescribed-trajectory "disruption" audited in `B_numerics.md` (right-hand side never reads the state, 3-D variant bypasses the solver, eta 44x too high, I_p 6 MA) with the smallest model that is physically standard and genuinely state dependent, solved by this repo's `cvode` crate, with controls that fail when the physics is wrong.
- **Files created:** `examples/iter_current_quench_0d.rs` (model, runner, 9 `#[test]` controls), this document. `examples/Cargo.toml` gained one `[[example]]` block with `test = true`.
- **Hardware:** CPU only. Every number below is copied from the logged runs listed in section 7.
- **Caveat on the solver state:** `crates/cvode/src/solver.rs` was being edited by another agent while this work ran (the tout-truncation Nordsieck rescale). The final test + run pair was executed back to back against one pinned state, `solver.rs` md5 `762a6f1eac514ea6a16a55863335ded6` (recorded in `solver_state_before.md5` / `_after.md5`, identical). One earlier run against an intermediate solver state gave different step counts (see 5.4).

## 1. Model

Two coupled resistive loops, plasma (p) and vessel (v):

```
L_p dI_p/dt + M   dI_v/dt = -R_p(T_e) I_p
M   dI_p/dt + L_v dI_v/dt = -R_v      I_v
```

The state carried by CVODE is `y = [I_p, I_v, Q_p, Q_v]` with `dQ_p/dt = R_p I_p^2`, `dQ_v/dt = R_v I_v^2`, so the Ohmic dissipation is integrated by the same BDF method as the currents and the energy balance

```
W_mag(0) - W_mag(t) = Q_p(t) + Q_v(t),   W_mag = 1/2 L_p I_p^2 + M I_p I_v + 1/2 L_v I_v^2
```

is an identity of the continuous system that the discrete solution must satisfy to solver accuracy. The 2x2 linear system in the derivatives is inverted analytically (`det = L_p L_v - M^2 > 0` is enforced). An analytic Jacobian is supplied to CVODE (`.jacobian(...)`).

**Plasma resistance.** `R_p = eta_par(T_e) 2 pi R0 / (pi a^2 kappa)`, with the Spitzer parallel resistivity in the NRL Plasma Formulary form

```
eta_perp = 1.03e-2 Z lnLambda T_e^{-3/2} Ohm cm,   eta_par = 0.51 eta_perp   (T_e in eV)
=> eta_par = 5.25e-5 Z_eff lnLambda T_e^{-3/2} Ohm m
```

Coulomb logarithm from the NRL electron-ion formula (`23 - ln(n_e^{1/2} Z T_e^{-3/2})` for `T_e < 10 Z^2 eV`, `24 - ln(n_e^{1/2} T_e^{-1})` above; `n_e` in cm^-3), evaluated at the local `T_e`. Check built into the tests: `eta_par(25 keV, Z=1, lnLambda=17) = 2.259e-10 Ohm m` against the NRL reference `2.26e-10` quoted in `B_numerics.md` section P. Approximation stated in the code: the `0.51` factor is the Z=1 Spitzer-Haerm value; linear `Z_eff` scaling overestimates eta by ~10-25 % at `Z_eff` = 2-4 (exact factors 0.44 at Z=2, 0.38 at Z=4). Neoclassical corrections are neglected (the post-TQ plasma is collisional).

**Plasma inductance.** `L_p = mu0 R0 (ln(8 R0 / (a sqrt(kappa))) - 2 + l_i/2)`, the standard large-aspect-ratio external+internal inductance of an elongated ring. With the ITER numbers below: `L_p = 1.0483e-5 H`.

**Electron temperature after the thermal quench.** Two modes, selectable per run:
- `TeMode::Fixed` (the primary mode, used for the scan): `T_e` is a parameter, scanned 5-20 eV.
- `TeMode::OhmicRadiative`: quasi-static balance `R_p(T_e) I_p^2 = P_rad`, `P_rad = n_e n_imp L_z V` with constant `L_z`. Solved algebraically inside the RHS (8 fixed-point iterations on lnLambda), floored at 1 eV. Justification for quasi-static: the thermal energy of a 1e20 m^-3, ~10 eV plasma in 832 m^3 is ~2e5 J against ~1e10 W of Ohmic power, so `T_e` relaxes in 1e-5..1e-4 s, three to four orders below `t_CQ`. This is the textbook "radiative equilibrium during the current quench" picture; the constant `L_z` is the honest weak point (real cooling curves are strongly T-dependent) and this mode is reported as illustrative, not as a prediction.

## 2. Parameters, sources and assumptions

| Symbol | Value | Status |
|---|---|---|
| R0, a, kappa | 6.2 m, 2.0 m, 1.7 | ITER Physics Basis (Nucl. Fusion 39 (1999) 2137) / ITER design; kappa95 ~1.7 (separatrix ~1.85). Used 1.7. |
| B0 | 5.3 T | ITER (reported only; the circuit model does not use B0). The paper's 6.37 T is wrong, see `A_paper_claims.md`. |
| I_p(0) | 15 MA | ITER baseline scenario. |
| l_i | 0.8 | Assumed. A post-TQ profile is flatter (l_i ~0.5-1); only enters `L_p` through `l_i/2`. |
| n_e | 1e20 m^-3 | Assumed; consistent with `data/fusion/iter_plasma_parameters.csv`. Post-TQ densities with massive gas injection can be higher; enters only via lnLambda. |
| Z_eff | 1, 2, 3 | Assumed scan values; impurity-dominated post-TQ plasmas sit at Z_eff ~2-4. |
| T_e (Fixed) | 5, 7.5, 10, 12.5, 15, 20 eV | Scan requested by the task. |
| n_imp/n_e, L_z | 0.02, 1e-31 W m^3 | OhmicRadiative mode only. Order of magnitude of Ne/Ar coronal cooling rates near 10-30 eV; **assumption**, not a fit. |
| L_v | 5.0e-6 H | **Assumption.** `mu0 R0 (ln(8R0/b) - 2)` with an effective vessel minor radius b ~3 m gives 3-6 uH; the large-aspect-ratio formula is marginal at R0/b ~2. |
| M | 4.0e-6 H | **Assumption.** M/L_v = 0.8 sets the ideal-wall induced current at 80 % of the lost plasma current; coupling coefficient k = M/sqrt(L_p L_v) = 0.552. |
| R_v | 8.0e-6 Ohm | **Assumption.** The ITER double-wall vessel one-turn resistance is of order 1e-5 Ohm (I recall ~7-8 uOhm but could not verify a citation here; treat as uncertain). Gives tau_v = L_v/R_v = 0.625 s. |

`W_mag(0) = 1.1794e9 J`, `V = 832.2 m^3`, `R_p(10 eV, Z_eff=2) = 5.842e-5 Ohm`, `tau_LR = L_p/R_p = 0.1795 s`.

## 3. Solver setup

`Cvode::builder(Method::Bdf).rtol(1e-6).atol(1e-3).max_steps(5_000_000).jacobian(analytic)`; outputs every 0.2 ms; integration stops when `I_p < 1 % I_p(0)` or at 2 s. `rtol >= 1e-7` throughout, because `B_numerics.md` C1/C3/C4 showed an error-test failure of this crate at `rtol <= 1e-8` (being fixed by another agent). Absolute tolerance 1e-3 A / 1e-3 J is ~1e-10 of the scales involved, so the runs are effectively rtol-controlled.

`t_CQ` follows the ITER convention: `t_CQ = (t20 - t80)/0.6`, with `t80`, `t20` the first downward crossings of 0.8 and 0.2 `I_p(0)`, linearly interpolated on the 0.2 ms output grid.

## 4. Controls (real results)

All nine tests pass: `cargo test --release -p examples --example iter_current_quench_0d` -> `test result: ok. 9 passed; 0 failed`. The example's `main` re-runs the same code paths and prints the numbers below (`run_stdout.txt`).

| # | Control | Assertion | Measured |
|---|---|---|---|
| 1 | Positive: M = 0, fixed T_e = 10 eV, compare with `I_p0 exp(-t R_p/L_p)` for t <= 3 tau (over >100 grid points) | rel err < 1e-4 at rtol 1e-6; error at rtol 1e-4 must be larger; `t_CQ = tau ln4/0.6` to 1e-4 | max rel err **7.75e-6**; `t_CQ` analytic 0.41462 s vs solver 0.41462 s; I_v stays 0 |
| 2 | Energy balance, coupled reference run, Q from ODE quadrature states and independently by trapezoid on the output grid | both residuals < 1e-3 relative to W_mag(0); I_v peak > 1 MA; > 50 % of W_mag(0) dissipated by run end | quadrature residual **3.96e-6**, trapezoid **6.5e-6**; I_v peak 7.617 MA at 0.244 s |
| 3 | Perturbation: eta x2. (a) M = 0: fitted tau and `t_CQ` must halve; (b) coupled: `t_CQ` must fall to 40-80 % | (a) ratio 0.5 +- 1e-3; (b) 0.4 < ratio < 0.8 | (a) tau ratio **0.499996**, `t_CQ` 0.4146 -> 0.2073 s; (b) `t_CQ` 0.3329 -> 0.1551 s, ratio **0.4659** |
| 4 | Ideal wall R_v = 0: vessel flux `M I_p + L_v I_v` conserved; `I_v -> (M/L_v)(I_p0 - I_p)`; plasma flux must still decay; resistive vessel must *not* conserve it (negative control); drift must shrink with rtol | drift < 1e-5, shrinks at rtol 1e-7; resistive drift > 1e-2 | drift **1.967e-6** (rtol 1e-6), **4.18e-7** (rtol 1e-7); I_v(end) 11.8801 MA vs 11.8801 MA predicted; resistive-vessel drift 0.896 |
| 5 | T_e scan 5-20 eV, Z_eff = 2: `t_CQ` finite, positive, strictly increasing in T_e; uncoupled tau(20 eV)/tau(5 eV) = 8 x lnLambda ratio to 1e-3; band comparison printed, **not** asserted | see section 5 | monotone; 1 of 6 points inside 50-150 ms |
| 6 | Audit C2: +1 MA initial-condition offset must evolve, not persist | offset at run end < 20 % of initial, sign preserved | 1.000e6 A -> **9.998e3 A** at t = 1.692 s (ratio 0.0100) |
| 7 | Spitzer/NRL and Coulomb-log values | eta(25 keV) = 2.26e-10 +- 1 %; eta(10 eV, lnL 10) = 1.661e-5 +- 1 %; lnLambda(1e20, 10 eV) in 9-11.5 | pass (eta 2.259e-10) |
| 8 | Inductance formula and k < 1 | `L_p` matches the closed form to 1e-12, 9-12 uH; 0.3 < k < 1 | `L_p` 1.0483e-5 H, k 0.552 |
| 9 | Ohmic-radiative T_e closes `R_p(T) I^2 = P_rad` to 1e-6 and the run still satisfies the energy balance | pass | T_e(15 MA) = 8.39 eV, P_rad 1.664e10 W, residual 1.03e-7 |

On control 4: the task text asked for "I_p L_p + M I_v constant" in the R_v -> 0 limit. That quantity is the *plasma* flux and decays at rate `R_p I_p` regardless of the vessel; the invariant of the ideal-wall limit is the *vessel* flux `M I_p + L_v I_v` (second circuit equation with R_v = 0). The test checks the correct invariant and also asserts that the plasma flux does decay. The 2e-6 residual is the modified-Newton stopping tolerance (iteration matrix reused across steps), not a conservation defect; it scales with rtol as required.

## 5. Results

### 5.1 Reference run (fixed T_e = 10 eV, Z_eff = 2)
`t80 -> t20` gives **t_CQ = 0.3329 s**; fitted e-folding time over the 80-20 % window 0.1436 s (shorter than tau_LR = 0.1795 s: the decaying plasma current induces vessel current, which the vessel resistance then dissipates; the plasma sees the vessel as an extra sink early on). Peak vessel current 7.617 MA at 0.244 s. Energy residual 3.96e-6. CVODE: 50,614 steps for 1.69 s of physical time at 0.2 ms output spacing (the crate truncates each step to land on tout; see `B_numerics.md` C4).

### 5.2 T_e scan (fixed T_e), t_CQ in seconds

| T_e | Z_eff = 1 | Z_eff = 2 | Z_eff = 3 |
|---|---|---|---|
| 5 eV | 0.2353 | 0.1211 | **0.0832** |
| 7.5 eV | 0.4360 | 0.2165 | **0.1465** |
| 10 eV | 0.7021 | 0.3329 | 0.2221 |
| 12.5 eV | 1.0212 | 0.4707 | 0.3098 |
| 15 eV | 1.3808 | 0.6296 | 0.4100 |
| 20 eV | 2.1736 | 1.0041 | 0.6474 |

Bold: inside the published ITER band. Peak vessel current ranges 3.8 MA (20 eV, Z_eff 1) to 9.9 MA (5 eV, Z_eff 3).

### 5.3 Comparison with the published ITER current-quench range
What I can cite with confidence: the ITER Physics Basis (1999) chapter 3 and Hender et al., Nucl. Fusion 47 (2007) S128 (Progress in the ITER Physics Basis, chapter 3, "MHD stability, operational limits and disruptions") give ITER current-quench times of roughly **50-150 ms**: the fast end set by the multi-machine database lower bound on the area-normalised quench time (`t_CQ/S` of about 1.7 ms/m^2, which for S = pi a^2 kappa = 21.4 m^2 gives ~36-40 ms; ITER design uses 50 ms as the fastest case for vessel-force loads) and the slow end by runaway-electron and halo-current considerations. I am confident of the 50-150 ms band and of the ~1.7 ms/m^2 normalisation from memory; the exact figure/table numbers should be checked against the paper before being quoted further (marked uncertain).

What the model says:
- With Spitzer resistivity and the ITER geometry, `t_CQ` lands in 50-150 ms only for **T_e of about 3-8 eV with Z_eff of 2-3** (this table: 5 eV / Z_eff 3 -> 83 ms; 7.5 eV / Z_eff 3 -> 147 ms; 5 eV / Z_eff 2 -> 121 ms). At 10-20 eV the quench is 0.2-2 s, far too slow.
- This is the standard conclusion of 0-D quench analyses: the observed 50-150 ms times imply a post-TQ plasma of a few eV with substantial impurity content, which is why the Ohmic-radiative balance matters. The model therefore **reproduces the published band only under a specific and physically plausible subset of the scanned parameters**; it does not independently predict it, because T_e and Z_eff are inputs.
- The Ohmic-radiative run (n_imp/n_e = 0.02, L_z = 1e-31) gives `t_CQ = 0.0504 s` with T_e starting at 8.39 eV and falling as I_p^{4/3} while the current decays; the current then follows the faster-than-exponential `I ~ sqrt(1 - t/t_end)` shape expected for constant radiated power, and the vessel current peaks at 11.2 MA. This sits at the fast edge of the band, but with an assumed `L_z` it is illustrative only.

No parameter was tuned to hit the band; the defaults (10 eV, Z_eff 2) were chosen before any run and give 0.33 s, outside it.

### 5.4 Tolerance refinement (reference run)

| rtol | energy residual | steps | t_CQ |
|---|---|---|---|
| 1e-4 | 1.26e-3 | 51,254 | 0.3329 |
| 1e-5 | 5.97e-5 | 49,955 | 0.3329 |
| 1e-6 | 3.96e-6 | 50,614 | 0.3329 |
| 1e-7 | 3.66e-7 | 51,148 | 0.3329 |

The residual falls by ~10x per decade of rtol, as it should for an rtol-controlled quantity. The step count is set by the output truncation, not by accuracy. `rtol = 1e-8` was not attempted (known crate failure, being fixed elsewhere). One run performed against an intermediate solver state during the other agent's edit gave 2-2.4x more steps (e.g. 119,668 vs 50,614 for the reference run) and ideal-wall flux drift 7.3e-14 instead of 1.97e-6, with all physical results (`t_CQ`, peaks) identical to 4 digits; only the pinned-state numbers are reported here.

## 6. Limitations (explicit)
- **0-D.** No current profile, no `l_i` evolution, no resistive diffusion; `R_p` is a single lumped number.
- **No halo currents, no vertical displacement.** Wall forces are not computed; the vessel current is a single filament.
- **No runaway electrons.** Avalanche gain during a 50-150 ms quench at 15 MA would dominate the real current evolution; here I_p decays to zero resistively.
- **No MHD, no thermal quench.** The thermal quench is assumed complete at t = 0; T_e is either fixed or in quasi-static radiative balance with a constant `L_z`.
- **Vessel parameters are assumptions** (section 2). Peak vessel current and the coupled `t_CQ` shift move with M, L_v, R_v; the uncoupled controls do not.
- **Spitzer Z scaling** is linear with the Z=1 factor (10-25 % high at Z_eff 2-4); no neoclassical or trapped-particle corrections.
- **Numerics.** Linear interpolation of the 80/20 crossings on a 0.2 ms grid; energy and flux invariants hold to ~1e-6 at rtol 1e-6, limited by the modified-Newton stopping tolerance.

## 7. Reproduction and artefacts
```
cd /home/callensxavier_gmail_com/rusty-SUNDIALS-wt-fusion-audit
nice cargo test --release -p examples --example iter_current_quench_0d -- --nocapture --test-threads=1   # 9 passed
nice cargo run  --release -p examples --example iter_current_quench_0d                                     # ~2 s wall
nice cargo test --release -p examples                                                                      # lib 0, example 9 passed
```
Outputs (bulk, disk 2): `/mnt/disks/disk-socrateai-local-1/AutoevolveAI/fusion_audit_2026-09-27/current_quench/` - 30 trajectory CSVs (`t_s,Ip_A,Iv_A,Qp_J,Qv_J,Te_eV,Rp_Ohm,Wmag_J`, 23 MB), `summary.json`, `run_stdout.txt`, `test_stdout.txt`, `solver_state_before.md5`, `solver_state_after.md5`. Small summary in the repo: `data/fusion/current_quench_0d/summary.json` (21 KB). Override the bulk directory with `CQ0D_OUT_DIR`.
