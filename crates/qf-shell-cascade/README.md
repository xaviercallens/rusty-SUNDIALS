# qf-shell-cascade

The complexified dyadic shell cascade (Katz–Pavlovic / Desnyansky–Novikov with a dispersive
quantum-pressure regulator). Pure Rust; the only dependency is `num-complex`.

This is a port of `src/quantumfluids/w4_shell_model/{shell_dynamics,integrate}.py` in
[SocrateAI-Scientific-QuantumFluids](https://github.com/xaviercallens/SocrateAI-Scientific-QuantumFluids).
That module was itself designed to be cross-checked against
[SocrateAI-Scientific-MechanicaFluidorum](https://github.com/xaviercallens/SocrateAI-Scientific-MechanicaFluidorum)'s
independently written `exploration/dyadic_cascade.py` — two repositories already maintaining
parallel implementations of the same model on purpose, for validation. This crate exists so both
of them, and any future stream that needs this cascade, can depend on one implementation instead.

## The model

Shells `n = 0..=N`, wavenumbers `k_n = 2^n`, complex amplitudes `a_n`, boundary convention
`a_{-1} = a_{N+1} = 0`:

```text
da_n/dt = B_n(a) - nu k_n^2 a_n - i D k_n^2 a_n
```

with the conjugated-complexification nonlinearity `B_n(a) = k_{n-1} a_{n-1}^2 - k_n conj(a_n)
a_{n+1}`. It conserves `E = 1/2 sum |a_n|^2` exactly (to round-off) and reduces *exactly* to the
real dyadic model on real input — the reals are an invariant subspace at `D = 0`. `-nu k^2 a`
(real `nu`) is dissipative; `-i D k^2 a` (real `D`) is dispersive and energy-neutral, at the cost
of breaking that reality invariance (by design — the dispersive term's whole point).

Integration is classical RK4 with a fixed step `dt = 0.1 / ((nu+D) k_N^2 + k_N)`, a divergence
guard (`|a_n| > 1e12` or non-finite), and a `max_steps` refusal (raising rather than silently
reporting a supremum measured over a fraction of the intended horizon).

## What it computes

| Function | Meaning |
|---|---|
| `k_shells(n_max)` | `k_n = 2^n` |
| `nonlinear_real` / `nonlinear_conj` | the real and complexified nonlinearity |
| `viscous` / `quantum_pressure` | the two regulator terms |
| `rhs` | full right-hand side |
| `energy` | `1/2 sum |a_n|^2` |
| `enstrophy_sum` / `enstrophy_max` | **both** conventions — see below |
| `energy_rate` | `sum Re(conj(a_n) da_n)`, the conservation diagnostic |
| `step_size`, `make_profile` (`P1`/`P2`/`P3`), `integrate` | the runner |

`enstrophy_max` exists because MechanicaFluidorum's reference script computes `sup_Omega` as
`max_n 1/2 k_n^2 |a_n|^2`, not the sum its own docstring names — a pre-existing discrepancy the
upstream Python port documented rather than silently picking a side. Both are provided so a caller
can compare them instead of assuming they agree (`enstrophy_sum_and_max_reproduce_the_mf_discrepancy`
pins the exact numbers: on profile `P2` at `N=8`, `4.5` vs `0.5`).

## The positive control

`tests/positive_control.rs` reruns the same nine `(nu, profile)` configurations at `N = 8`, `D =
0` that MechanicaFluidorum's `data/dyadic_omega_sup.csv` publishes, and checks `sup_Omega` (max
convention) and `E_final` against those reference values to `1e-9` relative — transcribed
verbatim from that CSV, not regenerated, so the comparison doesn't depend on both repositories
being checked out. The agreement is tight rather than merely close because `k_n = 2^n` are exact
powers of two: multiplying by them only shifts the IEEE754 exponent, leaving the mantissa
untouched, so a Numba index-loop implementation and a Rust one associating products differently
still agree far more closely than a generic shell spacing would allow. `N >= 12` would need
10M+ RK4 steps to reproduce (a Numba-scale run); `N = 8` already exercises every code path.

## Performance

Release build: the full nine-configuration positive control (about 1.6M total RK4 steps,
dominated by `N=8, nu=0.1`'s 680,960) runs in ~7.5s, roughly 200k steps/s — in the same order as
MechanicaFluidorum's own measured Numba throughput (~5.5e5 steps/s) without any SIMD or
buffer-reuse optimization. `N >= 16` sweeps (10^10+ steps) remain intractable regardless of
language; this crate does not attempt them.
