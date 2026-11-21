# qf-popgen

Absorbing Markov chains on `{0, ..., N}` (Moran with selection, neutral Wright-Fisher) as the linear stiff ODE
`d pi / dt = pi (P - I)` integrated with this repository's CVODE (BDF, exact analytic Jacobian), and a
**fail-closed absorption certificate**: a run is accepted only if the kernel is a valid absorbing kernel, total mass
is conserved, every component of the state stays non-negative (`NegativeMass` otherwise: the signed mass sums alone
cannot see cancelling entries), the interior mass has decayed below a tolerance, and the absorption mass at `N` matches
the target. Every other outcome is an `Err` (`InvalidKernel`, `MassDrift`, `NegativeMass`, `NotAbsorbed`,
`ValueMismatch`, `Solver`, `NonFinite`).

Scope: Tier X (floating point). It cross-checks closed forms that are kernel-checked in Lean in the AutoevolveAI
repository (`formal_cert/Programmes/PopGen/`: `P^n(i,N) -> i/N` for Wright-Fisher and for Moran with `r = 1`,
`-> S_i/S_N` with `S_n = sum_{k<n} r^{-k}` for Moran with fitness `r`). The Poisson-mixture step from the
uniformised ODE to the n-step kernel is not formalised. It never supports a claim.

```
cargo run --release -p qf-popgen --bin fixation_ode -- crates/qf-popgen/results
cargo test -p qf-popgen
```

`results/results.jsonl` holds 115 rows (106 accepted runs, 9 refused negative controls); `results/summary.json` the
counts. The negative controls are: exponent-flipped formula, neutral `i/N` under selection, the identity kernel
(valid but never absorbing), a horizon that is too short, and a corrupted row sum. All nine are refused for the
stated reason; a negative control that is accepted, or a run that is refused, makes the binary exit with status 1.
