# qf-pgpe — the quantum-fluids reference engine of rusty-SUNDIALS

A pure-Rust **projected Gross–Pitaevskii equation** (PGPE) on a doubly-periodic 2D grid and the numerical instruments built on it for the
SocrateAI-Scientific-QuantumFluids programme (vortex transport, mutual friction, BKT thermodynamics of a classical Bose field, vortex–phonon
scattering). Every module is a port of a Python original **with a direct numeric cross-check against it**; the cross-checks found three real defects
(listed at the end), and one of them was in the Python programme itself.

```
cargo test --release -p qf-pgpe                       # 27 tests (+2 slow ones behind --ignored), ~40 s
cargo run --release -p qf-pgpe --example vortex_transport -- validate   # the G1 known answers of the campaign
```

## What is in the crate

| module / example | what it does | Python original | validated against |
|---|---|---|---|
| `ComplexField2D` (`lib.rs`) | IF-RK4 PGPE, sharp cutoff `k_cut = frac·k_max`, norm/energy/momentum, `rhs`/`pack`/`unpack` for external integrators | `pgpe.py` | K1 norm 1e-8, K3 momentum 1e-10, K4 plane wave 1e-9, **K2 order 4.0 against CVODE** |
| `vortex` | periodic theta-function imprint (`imprint_v2`), raw plaquette detection with sub-grid refinement, tracker, tracked runs, placements | `vortex_transport.py` | G1 known answers; positions vs Python 2.8e-7 (N = 128), separations 1e-4 (N = 64) |
| `transport` | the registered estimators of `alpha` (energy, regression), `1 − alpha'`, `eta` (residual MSD), block jackknife, point-vortex velocity/energy on the torus, synthetic Langevin generator and gate G0 | `transport_estimators.py` | four real tracks: max relative difference **2e-14**, `alpha_regression` and `alpha_energy` bit-identical |
| `thermal` | seeded RNG, `random_state`, `heat`, thermometer, condensate fraction, current correlators (`n_s/n`), raw vortex count, Fourier `resample`, `run_blocks`, `make_base` | `observables.py`, `round2.py`, `make_*_bases.py` | on a shared field: ≤ 3e-14 (T, `n_s/n`, energy, norm); vortex count exact; statistical check of `make_base` below |
| `scattering` | Bogoliubov-wave dressing of a vortex pair, wave momentum flux, drift slopes, `sigma_par(k)`, `sigma_perp(k)` | `vortex_wave_scattering.py`, `analyze_wave_scan*.py` | positions 7e-14, momentum 8e-13; full probe `sigma_par = 2.03913`, `sigma_perp = 1.58133` identical to 6 digits; 31-run matrix: **4.6e-11** |
| `rect`, `npy`, `flow` | rectangular periodic grids with numpy-convention 2D FFTs; `.npy` I/O; the flow-past-an-obstacle solver (moving frame `v∂ₓψ`, Gaussian obstacle, absorbing layers `Γ(x,y)`) with an independent explicit-RK4 scheme | the reference code of Kwon & Shin (Zenodo 10.5281/zenodo.20068724) | **external**: force on the obstacle reproduced to 6×10⁻⁶ relative (t ≤ 0.3) from the reference's own initial field, identical to an independent numpy implementation (1.127×10⁻⁶ max); see below |
| `examples/` | `vortex_transport`, `transport_estimate`, `thermal_base`, `wave_scattering`, `wave_scan` (parallel over runs, resumable), `round3_finite_size`, `g0_scan`, `bench_step` | the campaign scripts | see each header |
| `tests/cvode_crosscheck.rs` | the same PGPE right-hand side integrated by **rusty-SUNDIALS CVODE** (Adams, rtol 1e-10) as an independent reference | — | IF-RK4 error vs CVODE: 1.2e-4, 7.3e-6, 4.6e-7 at `dt` = 0.04, 0.02, 0.01 (order 4.0); CVODE needs 365 RHS evaluations |

### Using it from the Python programme

`exploration/pgpe/rust_backend.py` (in SocrateAI-Scientific-QuantumFluids) runs `vortex_transport` on this engine and returns the tracks in the Python `npz`
layout, so the existing analysis (`analyze_transport.summarise`, `analyse_tracks`) runs unchanged on Rust output.

## Validation record (numbers measured, 2026-10-07 … 09)

* **Engine order against an independent integrator.** `ComplexField2D::rhs` is handed to `cvode::Cvode` (`Method::Adams`); the IF-RK4 global error against it falls as `dt^4`
  (1.17e-4 → 7.34e-6 → 4.58e-7). *Requires the Adams-order fix of PR #63*: on the unfixed `main` Adams stays at order 1 on this oscillatory Hamiltonian problem, the step collapses and the solve fails
  with `MaxSteps` after ~10⁶ RHS evaluations (measured: 200 000 steps, t = 0.39); with the fix, 365 evaluations in 0.2 s. The test is a regression guard for that fix.
* **Speed.** The step is allocation-free (`Workspace`, `step_into`) and the 2D FFT is rows–transpose–rows (unit stride); results are bit-identical to the previous implementation
  (same checksum), 1.7× faster at 128², 1.2× at 256² on a loaded 8-core machine. End to end, a 50-time-unit T = 0 tracked run at 128² takes 41 s here against 106 s of CPU for the numpy instrument (2.6×).
  The step is FFT-bound (8 FFT2 per step). The bigger lever is throughput: no GIL, so independent runs scale with cores (`wave_scan --workers`).
* **Thermal construction (not exactly reproducible by design).** The RNG is xoshiro256**, not numpy's, so `make_base` is a different realisation of the same ensemble. 9 seeds each, N = 64, L = 32, e = 0.60:
  T = 0.0757 (Rust) vs 0.0865 (Python), per-seed sd 0.018 / 0.010 — a 12.5 % gap at z = −1.6, **not** within the 10 % first asked for, but not significant; a 60-seed comparison at e = 0.8 cannot tell the two apart
  (raw vortex count 0.67 vs 0.57, condensate fraction 0.739 ± 0.009 vs 0.727 ± 0.014).
* **Not ported (declared):** `g1(r)` and its fits, vortex-dipole matching `Q`, pairing, the Onsager dipole order parameter, persistent-homology (TDA) instruments, and the symbolic Godfrin series — they stay in Python.

## Experiments run with this physics (SocrateAI-Scientific-QuantumFluids, all pre-registered; ledger CLAIM-NNN)

| result | where |
|---|---|
| vortex transport, friction, the dielectric relation of the vortex-pair gas (paper, v2.2) | doi:10.5281/zenodo.23262143 |
| friction follows the temperature, not the normal density; Born rival refuted | doi:10.5281/zenodo.23262132 |
| software bundle: Lean library, pre-registrations, scripts, ledger | doi:10.5281/zenodo.23262524 (v1.18.0) |
| BKT classical-field thermometer, finite-size runs (`round3_finite_size`) | doi:10.5281/zenodo.23144587 |

## External reproduction: Kwon & Shin vortex shedding (first target of the reproduction suite)

The reference run of *Dynamic similarity of vortex shedding in a superfluid flowing past a penetrable obstacle* (Phys. Rev. Research 2026; data and GPU code CC-BY-4.0, Zenodo 10.5281/zenodo.20068724) uses a pseudo-spectral split-step scheme in single precision. `flow` solves the same model with a **different** scheme (explicit RK4 on the full right-hand side, spectral derivatives, double precision), starting from the reference's own `psi_time_0.0.npy`, so agreement is a statement about the physics:

```
cargo run --release -p qf-pgpe --example kwon_shin -- --ref-dir DIR --t-end 0.3     # DIR: files extracted from the Zenodo zip
```
* force on the obstacle, t ≤ 0.3: max |F − F_ref| = 1.1×10⁻⁶ (6×10⁻⁶ relative); the numpy implementation of the same scheme gives the same number;
* **the reference holds the frame velocity at the end of each step**; reading the ramp at RK4 stage times instead produces a constant offset of 3.5×10⁻³ (5×10⁻³ relative) that is created in the 0.1 τ ramp — an O(dt) artefact of the reference (Σ v(iΔt)Δt overshoots ∫v dt by 0.00275 ξ), reproduced as `Ramp::StepEnd`;
* **performance is not a win on this geometry**: 1000 × 500 (non-power-of-two lengths) costs ≈ 0.8 s per RK4 step single-threaded in `qf-pgpe`, against ≈ 0.9 s for numpy — the advantage of the square power-of-two engine does not carry over; this size is the natural first target for the GPU phase.
* longer comparisons (ψ snapshots at t = 10 … 50, vortex counts) are tracked in the programme's `PGPE_EXTERNAL_REPRODUCTION.md`.

## What porting found

1. **A real imprint defect in the first Rust version** (found by the cross-check against Python): a vortex centred exactly on a grid node was imprinted with unit modulus instead of zero. Fixed (`unit()`); the N = 64 T = 0 separation now agrees with Python to 1e-4.
2. **An estimator bias in the Python programme.** The port of `transport_estimators.py` was bit-identical to Python on real tracks, and its `g0_scan` example then showed that the *energy estimator* of the friction `alpha` is biased low by the vortex diffusion:
   true `alpha = 0.02` returns 0.0200 at `eta = 0`, 0.0183 / 0.0163 / 0.0156 at `eta = 5e-4 / 1e-3 / 2e-3` (−8 / −18 / −22 %), independently of detection noise; the regression estimator is unbiased. The registered gate G0 had passed on one seed (its energy criterion passes on 2 of 12).
   The published `alpha` values were corrected in the papers (DOIs above).
3. **A hidden dependency**: PR #63 (CVODE Adams order) — see above.

## Reproducing

```
# vortex transport known answers
cargo run --release -p qf-pgpe --example vortex_transport -- validate
# thermal base state (a Fourier-resampled start is allowed: --from BASE.raw --from-n 128)
cargo run --release -p qf-pgpe --example thermal_base -- --n 128 --l 64 --e 0.60 --t-tr 1000 --t-end 1500 --seed 1 --out base
# estimators on tracked CSVs
cargo run --release -p qf-pgpe --example transport_estimate -- --l 64 --q 1,-1 track.csv
# the registered vortex-phonon scattering scan (parallel, resumable)
cargo run --release -p qf-pgpe --example wave_scan -- --out-dir scan --workers 4 [--d0 20,24,28]
# the estimator-bias study
cargo run --release -p qf-pgpe --example g0_scan -- 8 --eta-scan
```
