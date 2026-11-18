# Python tests, coverage and benchmarks: measured on 2026-11-18

Commit `d94055a22c0f` (rusty-SUNDIALS `main`, shallow clone). Host: GCP VM, Linux x86-64, 8 vCPU. This replaces three
"not verified" rows of the README's Verification status section. An AI agent ran it under the repository owner's
direction; it is not a human audit and not an independent reproduction. Nothing here is run in CI.

## 1. Python tests

- Command (repository root): `python -m pytest -q -p no:cacheprovider` with `pytest.ini` as shipped
  (`testpaths = autoresearch_agent/tests`). Python 3.11.16 in a virtual environment that was **not** built from
  `autoresearch_agent/requirements.txt` (for example `problog` is absent, so the "DeepProbLog" gate runs its pure-Python
  fallback; the tests still pass).
- Result: **230 collected, 230 passed, 0 failed, 4 warnings, 228 s**, across 7 test files. The README's earlier figure
  of 134 (80 + 54) does not match.
- Negative control: in a scratch copy, `autoresearch_agent/neuro_symbolic_v10.py` had the divergence check
  (`if not div_b:`) disabled. `pytest autoresearch_agent/tests/test_v10_suite.py -k NeuroSymbolic` then gave
  **1 failed, 7 passed** (`TestNeuroSymbolic::test_div_b_violation_caught`); with the file restored, **8 passed**. So
  at least that test can fail when the code it guards is broken. No other test was mutation-checked.
- Defect found: `test_v10_suite.py` does `sys.path.insert(0, "..")` (relative to the current directory), so it **fails
  to collect when run alone** (`ModuleNotFoundError: physics_validator_v10`) and passes only in the full run, where
  other test files add the package directory first. It needs `PYTHONPATH=autoresearch_agent` to run alone.

## 2. Coverage

- Command: `python -m pytest -q -p no:cacheprovider --cov=autoresearch_agent --cov-report=json:cov.json`
  (coverage.py 7.16.1, pytest-cov, line coverage, 550 s with the tool attached).
- Result: **49.0%** (3,210 of 6,548 statements) for the whole `autoresearch_agent` package **including the 7 test files**,
  which cover themselves at 99.8% (1,208 of 1,210).
- **Source files only: 37.5%** (2,002 of 5,338 statements, 39 files). 24 files with 2,709 statements are at 0%
  (for example `tearing_mode_agent.py`, 299 statements, and `tearing_mode_1d.py`, 172).
- The README's 98.4% has no command behind it that was found. The only files near 100% are the test files themselves;
  a figure computed over test files alone would be close to 99.8%, which is probably how such a number arises, but
  that is an inference and was not shown.
- Scope: Python only. The Rust coverage job in CI measures something else and was not examined here.

## 3. The ten benchmarks

- The README table lists ten examples. They were built and run once each: `cargo build --release -p examples` for
  `lorenz`, `hodgkin_huxley`, `sir_epidemic`, `lotka_volterra`, `hires`, `double_pendulum`, `rigid_body`, `rossler`,
  `fitzhugh_nagumo` and `three_body` (rustc 1.96.1, a separate target directory, processes pinned to cores 0-5).
  The build succeeded (31 s) and **all ten exit 0**.
- The shipped `examples/run_benchmarks.sh` only works on macOS (`sysctl`, `/Volumes/ramdisk`, `perl alarm`); it was not
  used.
- **The table does not reproduce.** Steps and RHS evaluations, which do not depend on the machine, are lower in all ten
  cases:

| Example | README steps / RHS evals | Measured steps / RHS evals |
|---|---|---|
| Lorenz | 3,819 / 14,498 | 686 / 2,111 |
| Hodgkin-Huxley | 307 / 1,060 | 234 / 653 |
| SIR | 10,949 / 59,704 | 67 / 227 |
| Lotka-Volterra | 9,023 / 48,307 | 175 / 643 |
| HIRES | 10,711 / 40,682 | 241 / 708 |
| Double pendulum | 4,892 / 18,667 | 315 / 876 |
| Rigid body | 15,364 / 84,152 | 70 / 222 |
| Rossler | 6,567 / 24,556 | 553 / 1,478 |
| FitzHugh-Nagumo | 22,270 / 120,093 | 210 / 837 |
| Three-body | 3,139 / 13,478 | 1,053 / 3,138 |

- Wall time per process was about 6 to 8 ms here against about 410 to 460 ms in the table. The two include different
  things (the table's script timed `python3` and `perl` wrappers around each run), so this is not a speed comparison.
- Why the counts differ is **not established**. The example sources were last changed on 2026-05-12, one day after the
  first release; the table may come from an earlier version of the examples or the solver, or from different
  tolerances. This was not investigated by building old commits.
- Not checked: whether any of the ten results is numerically correct (these are runs to completion, not accuracy tests).

## Limits

One host, one run each; no repetition for timing; Python environment not the declared one; no human review.
