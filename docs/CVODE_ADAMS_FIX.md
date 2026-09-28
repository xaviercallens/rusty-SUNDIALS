# CVODE Adams never exceeded order 1, and diagnostics went to stdout (2026-09-28)

Two defects in `crates/cvode`, both found by earlier audits (docs/CVODE_TIGHT_TOLERANCE_FIX.md
follow-ups, crates/qf-bao-distances/README.md finding 1).

## A. `Method::Adams` was implicit Euler

### Symptom
On the Einstein-de Sitter distance integrand `dD/dz = (1+z)^{-3/2}` the Adams error scaled like
`sqrt(rtol)`: 2.1e-4 relative at rtol 1e-7 (~7,000 steps), 6.9e-5 at 1e-8 (~21,000 steps). cvode's
own Adams unit test only asserted `|y - 5| < 3.5` on `y' = 1`.

### Root cause
`compute_l` in `crates/cvode/src/solver.rs` was a placeholder for Adams: every coefficient set to 1
(`l = [1, 1, ..., 1]`). The error constant was the generic `1/(q+1)`, and order selection
(`try_order_change`) ran for BDF only. So an Adams run stayed at q = 1 forever with l = [1, 1] and
error coefficient 1/2, which is implicit Euler with a Milne error estimate. Under per-step local error
control a first-order method takes h ~ sqrt(rtol) steps, so the number of steps grows like
1/sqrt(rtol) and the global error (steps x rtol) like sqrt(rtol). The unfixed sweep below shows exactly
that: 3.16x less error per decade of rtol, 3.1x more steps.

### Fix
The Adams path now follows LLNL CVODE (cvode.c), in this crate's normalisation l[1] = 1 (LLNL uses
l[0] = 1; gamma = h * l[0] here equals LLNL's h / l[1]):

- `set_adams` = `cvSetAdams` / `cvAdamsStart` / `cvAdamsFinish` / `cvAltSum`: the l polynomial from the
  product `prod_j (1 + x / xi_j)` over the real past step sizes `tau[j]`, and the test quantities tq[1]
  (order q-1 error), tq[2] (local error test), tq[3] (order q+1 error), tq[4], tq[5].
- The local error test uses `dsm = tq[2] * ||Delta_n||`, with `Delta_n = l_0 * acor` (the correction in
  LLNL's normalisation).
- `adams_complete_step` = the `tau` shift and `zn[qmax]` save of `cvCompleteStep`, then
  `cvPrepareNextStep` (etaq with BIAS2 = 6, ADDON = 1e-6; when `qwait` reaches 0, `cvComputeEtaqm1`
  with tq[1] and BIAS1 = 6, `cvComputeEtaqp1` with tq[3], tq[5]/saved_tq5 and BIAS3 = 10;
  `cvChooseEta` with THRESH = 1.5), `cvSetEta` (etamax 10^4 on the first step, 10 after, 1 right after a
  failure; hmax), `cvAdjustParams` / `cvAdjustAdams` (zero the new column on an increase, the
  `x q INT{u (u+xi_1)...(u+xi_{q-2})}` correction on a decrease) and `cvRescale`.
- `adams_error_test_failure` = `cvDoErrorTest`: eta from dsm, ETAMIN 0.1, ETAMXF 0.2 from the second
  failure, order reduction after MXNEF1 = 3 failures, and at order 1 a restart with `zn[1] = h f(t, y)`
  and `qwait = LONG_WAIT`.
- A read-only accessor `Cvode::max_order_reached()` reports the highest order an accepted step used.

The BDF path is untouched: its l table, error constants, `try_order_change` and step controller are
the same code, and `crates/cvode/tests/adams_order.rs::bdf_decay_unchanged_reference` pins its
previously documented numbers (614 steps, 1.01e-8 at rtol 1e-10 on y' = -y).

### Tests: `crates/cvode/tests/adams_order.rs` (9 tests; 8 fail on the unfixed code)
Written before the fix and run against origin/main first; every reference is a closed form or a
conserved quantity.

| Test | Unfixed (origin/main) | Fixed |
|---|---|---|
| (1) y'=-y to t=10, err(1e-8)/err(1e-6) < 0.05 and err(1e-8) < 1e-6 | ratio 0.100, err 6.4e-4, q=1 | ratio 0.015, err 1.6e-8, 100 steps, q up to 9 |
| (1-control) same with `max_order(1)` (= the placeholder's l=[1,1], C=1/2) must FAIL the ratio test | ratio 0.100; the test itself failed on the unfixed code because its step-count clause compares against the full-order run, which was also q = 1 | ratio 0.104, 183,041 steps: fails the criterion, test passes |
| (2) y'=cos t, same criterion | ratio 0.100, err 1.7e-3 | ratio 0.015, err 1.6e-7, 93 steps, q up to 12 |
| (3) max order reached >= 3 | 1 | 9 |
| (4a) logistic y'=y(1-y), rtol 1e-8, rel err < 1e-6 | 5.5e-5 (19,778 steps) | 3.7e-8 (101 steps, q 7) |
| (4b) Lotka-Volterra, invariant drift < 1e-6, vs BDF rtol 1e-10 < 1e-5 | 4.0e-4 / 1.5e-3 (127,279 steps) | 1.5e-8 / 5.0e-8 (378 steps, q 10) |
| (6) EdS dD/dz=(1+z)^{-3/2}, rtol 1e-7, z in {0.1..4}, worst rel err < 2.1e-6 | 2.15e-4 (29,728 steps in all) | 1.27e-8 (235 steps in all, q 8) |
| work-precision sweep monotone, >= 2x per decade, <= 250 rtol | 651 rtol at 1e-4 | passes (table below) |
| BDF reference unchanged | 614 steps, 1.0149e-8 | 614 steps, 1.0149e-8 |

The unit test `solver::tests::test_linear_growth` (y' = 1, Adams) was tightened from `|y-5| < 3.5` to
`< 1e-9`.

### Work-precision: y' = -y, atol 1e-14, max rel err over t = 1..=10 (real runs of `work_precision_adams_decay`)

| rtol | unfixed: nst | unfixed: err | unfixed: err/rtol | fixed: nst | fixed: q max | fixed: err | fixed: err/rtol |
|---|---|---|---|---|---|---|---|
| 1e-4 | 802 | 6.51e-2 | 651 | 53 | 5 | 2.71e-4 | 2.7 |
| 1e-5 | 2,496 | 2.03e-2 | 2,030 | 58 | 8 | 5.90e-6 | 0.6 |
| 1e-6 | 7,878 | 6.38e-3 | 6,380 | 68 | 9 | 1.08e-6 | 1.1 |
| 1e-7 | 24,870 | 2.01e-3 | 20,143 | 77 | 9 | 2.82e-7 | 2.8 |
| 1e-8 | 78,508 | 6.37e-4 | 63,726 | 100 | 9 | 1.58e-8 | 1.6 |
| 1e-9 | 245,948 | 2.03e-4 | 203,422 | 113 | 10 | 3.37e-9 | 3.4 |
| 1e-10 | 733,424 | 6.95e-5 | 694,553 | 132 | 9 | 9.76e-10 | 9.8 |

Unfixed: error x0.316 and steps x3.16 per decade, the sqrt(rtol) signature. Fixed: error within 10x
of rtol over the whole range with 53-132 steps. For comparison, BDF on the same problem
(docs/CVODE_TIGHT_TOLERANCE_FIX.md) needs 131-614 steps and reaches err/rtol 2.3-101.

### EdS with `qf-bao-distances`
`chi_cvode` still uses BDF (not changed here). `cargo test -p qf-bao-distances -- --ignored` (the
spec-1e-7 test, BDF, rtol 1e-7, 40 redshifts) measured 4.262e-7 after this change (6,591 steps,
16,827 RHS evaluations), the same as before it, since the BDF path is unchanged; that test stays
ignored. The Adams number on the same integrand is in the table above (1.27e-8 at z <= 4, fresh solve
per redshift), so switching `chi_cvode` to Adams would meet the 1e-7 spec.

## B. Diagnostics on stdout

`solver.rs` printed `ERROR FAIL 1/2/3` with `println!`. A library must not write to stdout; on the
stdio MCP server it corrupted the JSON-RPC stream, which is why `crates/sundials-mcp` runs solvers in
a worker subprocess. The three calls are now `eprintln!`.

Proof: `crates/cvode/tests/diagnostics_stderr.rs` re-runs the test binary as a child on a fixture that
deterministically reaches the ERROR FAIL 3 path for both methods (y' = step(t-0.5), `min_step 0.1`,
rtol 1e-10, atol 1e-14: the straddling step's error estimate does not shrink with h and the minimum
step keeps h there, so MAX_ERR_TEST_FAILS = 7 failures occur in one step) and asserts the diagnostic is
on the child's stderr and absent from its stdout. Reverting to `println!` fails it.

`crates/sundials-mcp/tests/stdio.rs`: the two `#[ignore]`d tests
(`solver_stdout_never_reaches_the_protocol_stream` and its control
`control_without_isolation_the_stream_is_corrupted`) needed a built-in problem that printed to stdout;
none did after the tout-rescale fix, and with the solver silent on stdout the control can never show
corruption again. They are replaced by `worker_binary_writes_nothing_to_stdout_on_a_failing_solve`
(runs `sundials-mcp --worker` directly with stdout captured on the failing `domain_exit` solve: stdout
must be empty, result `status: err, ran: true`) and
`without_isolation_a_failing_solve_leaves_the_protocol_stream_pure` (`SUNDIALS_MCP_NO_ISOLATION=1`
session stays pure JSON-RPC). The isolation layer stays as defense in depth and for the timeout; the
`about` text, module docs and README say so. The README's stale observations 1 and 2 (exponential and
robertson failing at rtol 1e-9) were re-measured through the worker binary: both succeed now (432 and
837 steps).

## Test counts (`nice cargo test`, default features)
- `-p cvode`: before 8 unit + 5 tight_tolerance; after 8 unit + 5 tight_tolerance + 9 adams_order +
  2 diagnostics_stderr, all passing. With `--features experimental-nls-v2`: the same minus the exact
  BDF step-count pin, which is gated to the default Newton path (8 + 5 + 8 + 2).
- `-p sundials-mcp`: before 13 unit + stdio 3 passed / 2 ignored; after 13 unit + 5 stdio, 0 ignored.
