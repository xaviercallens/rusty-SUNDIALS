# Results: atol-dominance monitor (issue 17), 2026-11-19

Preregistration: `docs/verification/PREREG_ATOL_MONITOR_2026-11-19.md` (commit 0b4939f, committed before any monitor code). Monitor and harness commit: 95ca3a9b5b5e822a36de98db80eaac98976b448a (branch `feat/atol-dominance-monitor`, base 34d8669). Toolchain rustc/cargo 1.96.1, Linux, pinned to cores 0-5.

## Commands

```
cargo test -p cvode                                   # all pass (12 unit + 5 monitor integration tests + the older suites)
cargo clippy -p cvode --all-targets -- -D warnings    # clean
cargo build --release -p examples --example atol_monitor_experiment
taskset -c 0-5 target/release/examples/atol_monitor_experiment
```

## Outcome against the preregistered criteria: SUCCESS (with one false alarm that the criteria allowed)

| Criterion | Result |
|---|---|
| (1) y2 flagged in all four published Robertson settings | yes, 4 of 4: (1e-4,1e-7) 367/392 hit steps, (1e-6,1e-9) 440/491, (1e-8,1e-11) 750/801, (1e-10,1e-10) 838/889 |
| secondary: y1 and y3 not flagged in the positive runs | yes, 0 hit steps for y1 and y3 in every run |
| (2) y2 not flagged for Robertson (1e-4, 1e-13) | yes: 0 hit steps of 420 (y2 atol-dominated in only 13 steps) |
| (3) at most 1 of 10 well-posed runs flagged | 1 of 10: **hires, component index 7 (y8), 134 hit steps of 241 (55.6%, just over the 50% threshold)** |
| (4) component-step false-alarm rate on the ten at most 5% | 139 hit component-steps of 21,658 = 0.64%; atol-dominated alone (no Jacobian condition) 520 of 21,658 = 2.4% |
| (5) bit-identical with monitor on | yes, all 16 runs: every output value (raw f64 bits), step count and RHS count equal |

False-alarm metrics. Runs: 1 of the 10 well-posed runs flagged (10%); 1 of 11 if the small-atol Robertson is added (9.1%). Component-steps: 0.64% on the ten. The hires flag is counted as a false alarm by the preregistered design (a well-posed control); this experiment did not measure whether the true hires error in y8 exceeds rtol, so it is a flag on a control, not a demonstrated error.

Limits of the negative controls. Four of the ten (sir_epidemic, lotka_volterra, rigid_body, fitzhugh_nagumo) use the Adams method, which holds no Jacobian, so the rule cannot flag them by construction; only the six BDF runs genuinely test the Jacobian condition, and one of those six flagged. In the BDF runs, the Jacobian is finite-difference (cached), not analytic, except Robertson.

## Deviations from the preregistration

- None to the rule, constants, controls or criteria.
- The preregistered extra attempt Robertson (1e-10, 1e-13) did NOT fail here: it completed (1238 steps) and y2 was flagged (1187/1238). The published paper reports ErrTestFailure for that setting; this harness may differ from the published run in settings (it uses no `init_step` and `max_steps` 50,000,000; the published run's exact options were not re-checked), and the cause of the difference was not investigated. It is reported as an extra, outside the four counted runs.

## What this does NOT do

- It does not estimate the size of the global error. The issue's exit criterion asks to reproduce the 169x / 486x table from diagnostics alone; a flag cannot do that. That needs the global-error estimator (a later roadmap item). Not claimed.
- Positive evidence is one problem (Robertson, this port). The negative evidence is ten problems, four of which cannot flag by construction (Adams). Tier C.
- The flag does not say the user's answer is wrong; it says the error test is atol-dominated on a component with a stiff Jacobian column, the situation in which the published error excess occurred.

## Raw experiment output

Columns after the flag list: per component `hits/atol_dominated/jac_stiff/monitored` (accepted steps).

```
RUN | group | status | steps | nfe | bit_identical | flagged_components | per-component hit/atol_dom/jac_stiff/monitored
robertson rtol=1e-4 atol=1e-7 | positive | ok | 392 | 1043 | true | [1] | 0/0/0/392 367/392/367/392 0/37/0/392
robertson rtol=1e-6 atol=1e-9 | positive | ok | 491 | 1367 | true | [1] | 0/0/51/491 440/491/440/491 0/72/0/491
robertson rtol=1e-8 atol=1e-11 | positive | ok | 801 | 2243 | true | [1] | 0/0/51/801 750/801/750/801 0/106/0/801
robertson rtol=1e-10 atol=1e-10 | positive | ok | 889 | 2560 | true | [1] | 0/90/51/889 838/889/838/889 0/253/0/889
robertson rtol=1e-10 atol=1e-13 | positive | ok | 1238 | 3569 | true | [1] | 0/0/51/1238 1187/1238/1187/1238 0/181/0/1238
robertson rtol=1e-4 atol=1e-13 (small atol) | negative_robertson | ok | 420 | 1140 | true | [] | 0/0/51/420 0/13/369/420 0/42/0/420
lorenz | negative | ok | 686 | 2111 | true | [] | 0/0/0/686 0/0/0/686 0/0/0/686
hodgkin_huxley | negative | ok | 234 | 653 | true | [] | 0/0/0/234 0/0/0/234 0/0/0/234 0/0/51/234
sir_epidemic | negative | ok | 67 | 227 | true | [] | 0/0/0/67 0/0/47/67 0/1/0/67
lotka_volterra | negative | ok | 175 | 643 | true | [] | 0/0/0/175 0/0/0/175
hires | negative | ok | 241 | 708 | true | [7] | 0/0/0/241 0/14/0/241 0/42/0/241 0/27/0/241 0/51/0/241 0/40/0/241 0/48/0/241 134/134/179/241
double_pendulum | negative | ok | 315 | 876 | true | [] | 0/2/0/315 0/14/0/315 0/0/0/315 0/23/0/315
rigid_body | negative | ok | 70 | 222 | true | [] | 0/0/0/70 0/1/0/70 0/0/0/70
rossler | negative | ok | 553 | 1478 | true | [] | 0/0/0/553 0/0/0/553 0/14/0/553
fitzhugh_nagumo | negative | ok | 210 | 837 | true | [] | 0/0/0/210 0/0/0/210
three_body | negative | ok | 1053 | 3138 | true | [] | 0/1/458/1053 1/19/458/1053 0/0/400/1053 2/20/400/1053 1/19/566/1053 1/2/566/1053 0/15/0/1053 0/0/0/1053 0/16/0/1053 0/2/0/1053 0/0/0/1053 0/15/0/1053
SUMMARY well-posed runs: 10, flagged runs: 1
SUMMARY component-steps: monitored 21658, hit 139 (rate 0.006418), atol-dominated only 520 (rate 0.024010)
SUMMARY all runs bit-identical with monitor on: true
```
