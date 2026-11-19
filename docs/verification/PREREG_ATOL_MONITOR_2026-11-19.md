# Preregistration: atol-dominance monitor for the BDF controller (issue 17)

Date: 2026-11-19. Written and committed BEFORE any monitor code exists. Base commit: 34d8669 (main).

## Motivation (evidence-labelled)

Zenodo 10.5281/zenodo.24216837 (Tier C, this port, these settings, Robertson only): the true global error of the `cvode` crate in y2 reached about 169 times `rtol*|y2|` (up to 486 times for the other components) because y2 (about 1e-5) sits under atol, so the local error test is atol-dominated while the y2 column of the Jacobian carries the stiffness. Published tolerances: (rtol, atol) = (1e-4, 1e-7), (1e-6, 1e-9), (1e-8, 1e-11), and the non-preregistered extra run (1e-10, 1e-10); the preregistered (1e-10, 1e-13) run failed with ErrTestFailure.

## What is monitored

An opt-in, read-only monitor inside `cvode::Cvode`. It reads the solver's accepted solution y, the scalar `rtol` and `atol`, and the cached Jacobian `jac_mat` (the matrix the Newton iteration already holds; it is never recomputed for the monitor, so no extra RHS evaluation happens). It never writes solver state.

## The flag rule (fixed here)

Constants: c1 = 10, c2 = 10, hit fraction f = 0.5, minimum monitored steps m = 10. The window is the whole solve since the monitor was enabled (all accepted steps).

At every accepted step, with y the accepted solution at the new time, for each component i:

- A_i (atol dominated) = `atol > c1 * rtol * |y_i|`.
- J_i (stiff column) = a Jacobian is cached AND `colnorm_i >= c2 * median_j(colnorm_j)` AND `colnorm_i > 0`, where `colnorm_j = max_k |J[k][j]|` (infinity norm of column j of the cached Jacobian; "median" for even n is the mean of the two middle values; for n = 1 the rule can never hold).
- hit_i = A_i AND J_i.

Per component the monitor counts `steps_monitored`, `steps_atol_dominated` (A_i), `steps_jac_stiff` (J_i) and `steps_hit`. Component i is FLAGGED after the solve iff `steps_monitored >= 10` and `steps_hit >= 0.5 * steps_monitored`.

When no Jacobian is cached (the Adams method never forms one), J_i is false, so such runs cannot flag. This is a limitation of the rule, not a success: for every run the results report the A_i-only fraction separately so the reader can see what the Jacobian condition contributes.

## Experiment (fixed here)

Harness: one example binary in `examples/` replicating the ten problems with the same method, tolerances, initial values, output times and builder options as the existing example files at base commit 34d8669 (Jacobian by finite differences, as there). Robertson uses the analytic Jacobian and the twelve output times 0.1, 0.4, 1, 4, 10, 40, 100, 400, 1000, 4000, 10000, 40000 of the published runs, `init_step` unset, `max_steps` large.

Positive control (Robertson, y(0) = (1,0,0)): (rtol, atol) = (1e-4, 1e-7), (1e-6, 1e-9), (1e-8, 1e-11), (1e-10, 1e-10). y2 must be flagged in all four. Secondary: y1 and y3 must not be flagged. The (1e-10, 1e-13) run is also attempted; if the solver fails it is recorded as BLOCKED and not counted.

Negative controls (well-posed): lorenz, hodgkin_huxley, sir_epidemic, lotka_volterra, hires, double_pendulum, rigid_body, rossler, fitzhugh_nagumo, three_body, at their own tolerances (listed in the example files). Plus Robertson at rtol = 1e-4 with atol = 1e-13 (y2 is then rtol-dominated for nearly the whole run); y2 must NOT be flagged.

False-alarm metrics, computed on the ten well-posed runs: (a) flagged runs / 10 (a run is flagged if any component is flagged); (b) flagged component-steps = sum of `steps_hit` over all components, divided by sum of `steps_monitored` over all components. Reported also for the ten plus the small-atol Robertson (11 runs).

## Success and failure

SUCCESS iff all of: (1) y2 flagged in all four positive-control runs; (2) y2 not flagged in the small-atol Robertson run; (3) at most 1 of the 10 well-posed runs flagged; (4) component-step false-alarm rate on the ten at most 5%; (5) the solution with the monitor enabled is bit-identical to the solution without it (every output y, step count, RHS count) in every run.

PARTIAL iff (1), (2) and (5) hold but (3) or (4) fails (the monitor detects the known case but is too noisy).

FAILURE iff (1) or (2) or (5) fails. If the positive control (1) fails the work stops and the failure is reported; the rule is not retuned silently. If the rule is changed after seeing data, the results document states it, keeps the preregistered rule's results alongside, and labels the new rule post hoc.

## What this does not claim

The monitor emits a flag. It does not estimate the size of the global error. The issue's exit criterion to reproduce the 169x / 486x table from solver diagnostics alone is NOT addressed; it needs the global-error estimator (a later roadmap item). One problem class of positive evidence (Robertson, this port).
