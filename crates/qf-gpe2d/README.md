# qf-gpe2d

Split-step Fourier solver for the 2D Gross–Pitaevskii equation on a periodic box. Pure Rust; the
only dependencies are `rustfft` and `num-complex`.

This is a port of `src/quantumfluids/gpe/solver2d.py` in
[SocrateAI-Scientific-QuantumFluids](https://github.com/xaviercallens/SocrateAI-Scientific-QuantumFluids).
It is a **different** model from this repository's `qf-pgpe` crate: `qf-pgpe` ports the
BKT-specific *projected* GPE (a sharp Fourier cutoff, integrating-factor RK4, no chemical
potential term), used for that project's dual-scale/BKT rounds. This crate is the general
split-step workhorse used across 17 files of the project's PGPE-round energy-budget work,
known-answer tests, and TDA vortex-detection pipeline — a separately-used solver, not a
refinement of `qf-pgpe`.

## The model

```text
i dpsi/dt = -1/2 Lap psi + g |psi|^2 psi - mu psi          (hbar = m = 1)
```

`g = 1`, background density `n0 = 1`, so `mu = 1`, sound speed `c = 1`, healing length `xi =
1/sqrt(2 mu)`. Strang splitting: a pointwise nonlinear phase half-step, a Fourier-diagonal linear
step, then the nonlinear half-step again. Both are phase rotations, so the norm is conserved to
machine precision *by construction* — it cannot fail as a test. Energy drift is the control that
can, and does: a field with energy at the grid scale (white-noise phase) genuinely needs the
`max_dt` bound (`dt * k_max^2 / 2 << 1`), and the upstream Python docs record 33% drift at ~3x too
large a step.

`dt` is complex: real for real-time evolution, `dt = -i*tau` for imaginary-time relaxation (used
to build vortex cores — see `plant_vortices`, which imprints only a phase, letting the solver's
own relaxation create the density dip, rather than assuming a core-profile ansatz).

## What it computes

| Function | Meaning |
|---|---|
| `Grid2D`, `healing_length`, `max_dt` | grid, `xi`, the accuracy-bound timestep |
| `smooth_phase_noise` | a band-limited random phase (physical, not white-noise) |
| `step` / `evolve` | one Strang step / `n_steps` of it, sharing one FFT plan (see below) |
| `energy`, `norm` | `E = int[|grad psi|^2/2 + g|psi|^4/2]`, `int |psi|^2` |
| `plant_vortices` | imprint a phase-only vortex configuration (charges must sum to zero) |
| `radial_density_profile` | azimuthally averaged `|psi|^2`, for measuring a core size |

`evolve` does **not** call `step` in its loop — an earlier version did, which replans the FFT on
every single step (`FftPlanner::plan_fft_forward` is not free). It now builds one plan and one
`k2` array before the loop and shares them across all `n_steps`.

## Validation

- **7 tests ported from `tests/test_gpe_solver.py`**: norm conservation, energy conservation at
  `max_dt`, a 50x-too-large timestep visibly breaking it, drift shrinking with `dt`, a vortex pair
  developing a core of size `~xi` under brief imaginary-time relaxation, and over-relaxation
  destroying the pair (the failure mode the Python docstring warns about, pinned as a test so it
  cannot silently rot). **Not ported**: the dipole-vs-same-sign annihilation test, which depends
  on `quantumfluids.tda.vortex_persistence.extract_vortices` — a separate phase-winding vortex
  detector in a different module, out of scope for this solver port.
- **`tests/python_cross_check.rs`**: an exact-value comparison against the actual Python
  `step`/`energy`/`norm` functions on a fixed 4x4 deterministic field (data pasted from one
  `numpy.random.default_rng(42)` run, not regenerated at test time) — `1e-10` relative agreement,
  not just matching qualitative properties.

**A real bug this caught**: the first draft of the vortex-core test used a smaller grid (`n=128`
vs. the Python suite's `n=256`) to run faster, at the same `dx = xi/8`. Because the vortex centres
are placed at fixed *fractions* of the box (`0.35 L`, `0.65 L`), halving `n` also halves the box
and therefore halves the vortex separation in units of `xi` — a genuinely different physical
setup, not just a smaller version of the same one. The core-formation test failed until the grid
size was matched to Python's exactly. Left as a comment on the affected tests.
