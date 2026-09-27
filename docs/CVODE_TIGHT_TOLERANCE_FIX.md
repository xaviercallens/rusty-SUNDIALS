# CVODE tight-tolerance failure: root cause and fix (2026-09-27)

## Symptom
The fusion audit (docs/audit/fusion-2026-09-27/B_numerics.md, controls C1/C3/C4) found that the `cvode` crate stopped with `ErrTestFailure` ("ERROR FAIL 3") on trivially smooth problems at tight tolerances:
- a state-independent quadrature at rtol <= 1e-8;
- the L/R decay y' = -y/tau at rtol 1e-10;
- a 2x2 linear circuit at rtol 1e-8.

Accuracy also stalled between rtol 1e-6 and 1e-7.

## Root cause
Line numbers in this section refer to the code before the fix; after it, the call sits two lines lower.

The bug is at `crates/cvode/src/solver.rs:291-296` (`solve_normal`). When the next step would overshoot `tout`, the solver cuts the step to h' = tout - t, which gives eta = h'/h. It then called `NordsieckArray::rescale_with_interpolation(eta, q)` (`crates/cvode/src/nordsieck.rs:73-105`, binomial loop at 88-99). That routine computes

    z_i <- eta^i * sum_{j>=i} C(j,i) z_j

This is a Taylor shift of the history polynomial forward by the old h, followed by a rescale. z_0 is not shifted, so z_1 ends up holding eta*h*y'(t_n + h) where eta*h*y'(t_n) belongs, and the higher columns are wrong in the same way. Changing only the step size keeps the expansion point t_n, so the correct operation is LLNL's `cvRescale`: z_i *= eta^i. That is `NordsieckArray::rescale`, which the solver already uses everywhere else. The shift came in with commit 444d7f0 (routine from d848f71). Its doc comment says it replicates SUNDIALS' step-size adjustment. It does not: LLNL applies a pure eta^i scaling on a step-size change (`cvRescale`), and a binomial (Pascal) shift only appears in the predictor and its undo.

**How it was found (instrumented, not guessed).** A temporary per-attempt trace was added (t, h, q, err_norm, fails). On y' = -y at rtol 1e-10 it showed:
- Steady steps up to t = 0.982 with err_norm ≈ 0.53.
- The step cut for tout = 1 then gave err_norm = 6.6e5.
- The next 7 retries fell exactly in proportion to h: 6.6e5, 6.6e4, 9.4e3, 1.8e3, 3.7e2, 74, 15.

An error proportional to h is the signature of an O(h) inconsistency z_1 ≠ h f(y_n). Shrinking the step cannot clear it within MXNEF = 7 when the tolerance is tight. At loose rtol the same corruption only costs extra steps, which is why the defect appeared only at tight tolerances.

**Suspected causes that were ruled out.** The audit suspected two other mechanisms:
- no drop to order 1 after repeated failures;
- the zeroed z[q] on an order increase (solver.rs:723-728).

After the fix, the same trace over all five regression tests showed 16 rejected attempts out of 6936. Every one of them passed on its first retry, so neither mechanism is exercised on these problems. They are still deviations from LLNL but were not the cause here. The error constant is also not the cause. From a constant-step derivation (not a run): `BDF_ERR_COEFF` = 1/(q+1) applied to the z_1 correction overestimates LLNL's tq[2] estimate by a factor of (sum_{j<=q} 1/j)^2, which is conservative.

## Fix
This is a one-line change in `solver.rs` (`solve_normal`): `self.zn.rescale_with_interpolation(eta, self.q)` became `self.zn.rescale(eta, self.q)`. `rescale_with_interpolation` is left defined, but nothing calls it any more. Its commit (d848f71) cites the Lean item `rescale_interpolation_exact` in `proofs/lean4/roadmap/v2_upgrades.lean:55`. That item is an `axiom`, not a proof, and it only asserts that *some* rescaled array represents the same curve. Plain eta^i scaling satisfies it, so it does not certify the binomial shift.

## Regression tests: `crates/cvode/tests/tight_tolerance.rs`
Every reference is a closed form. The bounds are set from VODE BDF (scipy 1.13.1), CVODE's direct ancestor, run on the same problems and output times, with a margin of about 2-3x. VODE itself does not meet the task brief's literal targets of "<1e-8" and "within 10x of rtol": it gives 1.43e-8, and its error/rtol ratio runs from 36 to 143 over the sweep.

| Test | Before fix | After fix | VODE BDF | Bound |
|---|---|---|---|---|
| y'=-y, rtol 1e-10, atol 1e-14, max rel err over t=1..10 | ErrTestFailure at tout=1 | 1.01e-8 (614 steps) | 1.43e-8 | < 3e-8 |
| L/R (C3), rtol 1e-10, R×1 and R×2, t = τ, 2τ, 5τ | ErrTestFailure at t=τ | ≤ 1.87e-8 (R×2 at 5τ) | ≤ 1.86e-8 | < 4e-8 |
| 2x2 circuit (C4), rtol 1e-8, 500 outputs, max \|err\|/I0 | 1.42e-6 | 1.41e-9 | 2.79e-8 | < 1e-7 |
| y'=cos t, rtol 1e-10, 20 outputs, max abs err | ErrTestFailure at t=0.5 | 1.61e-9 | 5.39e-9 | < 1e-8 |

## Work-precision: y' = -y, atol 1e-14, max rel err over t = 1..10 (real runs)

| rtol | before: nst | before: err | after: nst | after: err | after: err/rtol | VODE BDF: err |
|---|---|---|---|---|---|---|
| 1e-4 | 612 | 9.91e-4 | 131 | 2.33e-4 | 2.3 | 5.28e-3 |
| 1e-5 | 1039 | 2.01e-4 | 176 | 3.41e-5 | 3.4 | 1.29e-3 |
| 1e-6 | 1530 | 2.49e-5 | 209 | 1.18e-5 | 11.8 | 3.59e-5 |
| 1e-7 | 1838 | 3.10e-6 | 288 | 1.85e-6 | 18.5 | 3.64e-6 |
| 1e-8 | FAILED (tout=3) | – | 413 | 3.11e-7 | 31.1 | 4.36e-7 |
| 1e-9 | FAILED (tout=1) | – | 461 | 5.73e-8 | 57.3 | 7.06e-8 |
| 1e-10 | FAILED (tout=1) | – | 614 | 1.01e-8 | 101.5 | 1.43e-8 |

The test asserts three things: the error decreases strictly, it drops by at least 2x per decade of rtol (the audited stall was 1.4x; the weakest decade after the fix is 2.9x), and err <= 250·rtol.

## Audit controls re-run against the fixed crate
The unmodified `B_runs/controls` was copied to /tmp, built in release mode and run with `ulimit -v 4000000; timeout 600`. It completed with rc=0 and no ERROR FAIL lines.

| Control | Before (controls_run4.txt) | After |
|---|---|---|
| C1 quadrature, rtol 1e-6 / 1e-7 / 1e-8 / 1e-10 | Te err 3.96e-6 / 2.80e-6 / FAILED / FAILED | 4.34e-6 / 8.12e-7 / 2.25e-7 / 6.70e-9 (nst 162 / 195 / 253 / 270) |
| C1 nst at rtol 1e-4 | 576 | 154 |
| C3 L/R, rtol 1e-10 | ErrTestFailure | rel err 7.2e-10 (t=τ), 4.6e-9 (t=5τ) |
| C4 circuit, nst at rtol 1e-4 / 1e-6 | 27583 / 29933 | 5785 / 4011 |
| C4 circuit, rtol 1e-8 | ErrTestFailure | energy balance -4.7e-9, 5871 steps |

## Test counts
Each of these was run with `nice cargo test`:
- `-p cvode -p sundials-core`:
  - before: cvode 8/8 unit tests, sundials-core 95/95.
  - after: cvode 8/8 unit tests plus 5/5 new tight_tolerance tests, sundials-core 95/95. The new tests fail 5/5 on the unfixed code.
- `-p cvode --features experimental-nls-v2`: after, 8/8 plus 5/5.
- `-p sundials-mcp`: `tests/stdio.rs` goes from 4/4 before to 2/4 after. Its "failing solve" fixture (exponential at rtol 1e-9) relied on this defect to trigger "ERROR FAIL" on stdout, and that solve now succeeds. The fixture needs a new genuinely failing input. That crate is outside this fix's scope.
  - Follow-up done on this branch: a documented, genuinely failing problem `domain_exit` (y' = -sqrt(y), leaves the RHS's real domain past t = 2) was added to `sundials-mcp` and now drives the fixture. It fails via a Newton convergence failure, which prints nothing, and no built-in problem reaches cvode's stdout `println!` paths any more. So the two stdout-isolation tests are `#[ignore]`d with that reason, and a new live test covers the failure path (isError, ran=true, no trajectory, pure JSON-RPC stream, server recovers). Result: `-p sundials-mcp` 10/10 unit, stdio 3 passed / 2 ignored.

## Follow-ups (found, not fixed here)
- cvode prints diagnostics with `println!` to stdout (`solver.rs` "ERROR FAIL 1/2/3"). A library should use stderr or a logger; that change would make the MCP isolation layer defense-in-depth and let the ignored tests be rewritten.
- Adams accuracy: on `domain_exit` (y' = -sqrt(y), smooth for t <= 1.5), Adams at rtol 1e-6 / atol 1e-10 gave max abs error 2.25e-4 at y ~ 0.06 (0.4 % relative), while BDF met < 1e-4. Not investigated; the tight-tolerance tests above cover BDF only.
