# qf-vlasov1d

1D1V electrostatic Vlasov–Poisson (electrons on a neutralising background). Pure Rust; the only
dependencies are `rustfft` and `num-complex`.

This is a port of `src/quantumfluids/kinetic/vlasov.py` in
[SocrateAI-Scientific-QuantumFluids](https://github.com/xaviercallens/SocrateAI-Scientific-QuantumFluids),
used for that project's Landau-damping and plasma-echo pre-registration (`K1`–`K3` of
`docs/designs/KINETIC_TDA_PREREG.md`).

## The model

```text
df/dt + v df/dx - E df/dv = 0,     dE/dx = 1 - rho,     rho = int f dv.
```

Strang splitting: the `x`-advection `f(x - v dt, v)` is an **exact** Fourier shift (multiplying
the Fourier coefficient of a periodic-in-`x` field by `exp(-i k v dt)` solves `df/dt + v df/dx = 0`
exactly, for any `dt`). The `v`-advection `f(x, v + E dt)` is a cubic semi-Lagrangian
interpolation, zero outside `[-vmax, vmax]`.

## Spline note (an honest deviation)

The Python solver uses `scipy.interpolate.CubicSpline`'s default not-a-knot boundary condition;
this crate uses a **natural** cubic spline (zero second derivative at both ends) to avoid pulling
in an external linear-algebra dependency for a small tridiagonal system. The two disagree only in
the boundary treatment of the two outermost velocity-grid intervals — where `f` is already many
orders of magnitude below its peak for the near-Maxwellian distributions this solver targets.
`tests/python_cross_check.rs` measures the actual size of the disagreement directly against
Python (`~4e-5` relative in `step`'s output, `0.0` bit-exact at the domain edge itself, `1e-9` or
better everywhere else) rather than asserting it away. The physics tests (Landau damping rate and
frequency, ported from `tests/test_kinetic.py`) already tolerate 1–2% for the reason they were
written that way in the first place — this is Landau-damping physics being checked, not a
bit-exact numerical reproduction — so the spline choice sits comfortably inside a margin the
Python authors already decided was appropriate. The one test needing tight agreement
(`ballistic_echo_matches_closed_form`, `1e-6` relative against a closed form) runs with
`field_on = false`, so it never calls the spline at all.

## What it computes

| Function | Meaning |
|---|---|
| `Grid`, `maxwellian` | phase-space grid, the Maxwellian `exp(-(v-u)^2/2)/sqrt(2 pi)` |
| `density`, `field_from` | `rho = int f dv`; `E` from `i k E_k = -rho_k` (background cancels `k=0`) |
| `advect_x`, `advect_v` | the two split-step half/full steps |
| `step`, `run` | one Strang step; a full time series with an observer callback and scheduled `events` |
| `mode_amplitude`, `pulse` | Fourier-mode readout; a cosine density perturbation |
| `energy`, `mass` | kinetic + field energy; total mass |
| `peak_fit` | damping rate and frequency from the local maxima of `\|a(t)\|` (parabolic refinement) |

`Grid::recurrence_time(mode)` gives the free-streaming recurrence time `T_R = 2 pi / (k dv)` at
which the velocity quadrature aliases the initial perturbation back — not physical, and no
resolution removes it, only postpones it (`recurrence_free_streaming_is_exact_and_field_shifts_it`
pins this alongside the fact that the field-driven case delays the peak past `T_R`).

## Validation

- **3 physics tests ported from `tests/test_kinetic.py`**: the Landau damping rate and frequency
  at `k=0.5` matched to 2%/1% against the certified root `omega_r = 1.415661888604536`, `gamma =
  -0.153359466909605` (Canosa 1973) — that certification is a completely different kind of
  numerics (interval-Newton in Arb ball arithmetic on the analytic dispersion relation, not
  reproduced here; only its published result is used to check this solver); the free-streaming
  recurrence and how the self-consistent field delays it; the plasma echo matching its closed form
  (`0.5 a^2 exp(-((k2-k1)t - k2 tau)^2/2)`) to `1e-6` relative, with the echo peak at `t=15`.
- **`tests/python_cross_check.rs`**: exact-value comparison against the real Python `density`,
  `field_from`, `advect_x`, `advect_v`, `step`, `energy` and `mass` on a fixed 8x12 deterministic
  field, `1e-9` or exact except where the spline note above applies.
