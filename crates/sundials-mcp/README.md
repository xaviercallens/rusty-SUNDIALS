# sundials-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server (stdio, JSON-RPC 2.0) that lets AI
agents and scientists run rusty-SUNDIALS solvers and read the results with their evidentiary status
made explicit. Pure `std` + `serde_json`; `#![forbid(unsafe_code)]`.

## Tools

| Tool | What it does | Checked against |
|---|---|---|
| `about` | server scope, honesty conventions, what is exposed | — |
| `list_problems` | the named CVODE problems, parameters, bounds | — |
| `solve` | `exponential`, `robertson`, `vanderpol`, `lorenz`, `domain_exit` via CVODE (BDF) | exponential: closed form `exp(-t)`; robertson: mass conservation and the LLNL SUNDIALS `cvRoberts_dns` published output at `t = 0.4`; vanderpol, lorenz: no closed form, trajectory and statistics only; domain_exit (y' = −√y): closed form `(1 − t/2)²` up to t = 2, and a genuine, documented solver failure for any output time past 2 (the RHS leaves its real domain) — used to test error reporting |
| `pgpe_run` | `crates/qf-pgpe` projected Gross–Pitaevskii solver on a 16/32/64 grid, ≤ 4000 steps | plane wave: exact solution `exp(i(kx − ωt))`, `ω = k²/2 + g`; norm and momentum conservation |
| `cmb_bound` | Koren-Tsai-Wang 2σ CMB bound on late-time dark-energy phase transitions via `crates/qf-cmb-cascade`; takes `beta_over_h` (1–1000), `zpt` (0.01–0.9), `mode` (`planck` or `cosmic_variance`); each call ≈10–30 s | Planck 2018 TT power spectrum (`mode=planck`) or cosmic-variance floor (`mode=cosmic_variance`); exact bubble spectrum from Elor et al. (arXiv:2311.16222) |
| `bao_distances` | `D_M/r_d`, `D_H/r_d`, `D_V/r_d` via `crates/qf-bao-distances` for flat `lcdm` / `wcdm` / `w0wacdm` (radiation off); takes `model`, `Om` (0.01–1), `w` or `w0`+`wa`, `h_rd` (50–200 Mpc, default 101.54 with `h_rd_default_used: true`), `z` (1–50 values in (0, 10]), `chi2_dr2` (bool); line-of-sight integral solved by CVODE (BDF, rtol 1e-7) | every call returns `max_rel_diff_cvode_vs_quadrature` against an independent adaptive Gauss–Kronrod evaluation; optional Gaussian χ² on the real DESI DR2 BAO data vector (13 points, arXiv:2503.14738), or `available: false` with the reason if the files are absent. The crate tests also check the Einstein–de Sitter closed form and astropy |

## Honesty conventions

- Every result carries **`ran`**. `ran: false` means the computation did not execute (bad arguments,
  timeout, worker crash) — never read it as a pass.
- A **solver failure is an MCP tool error** (`isError: true`) carrying the solver's own message, with
  **no partial trajectory**.
- Each known-answer check **names its reference**, which is fixed independently of this code.

## Why solvers run in a worker subprocess

`crates/cvode/src/solver.rs` used to print diagnostics to **stdout** on some failure paths
(`println!("ERROR FAIL 3: local error test failed > max times")`), which on a stdio protocol corrupts the
stream. Since docs/CVODE_ADAMS_FIX.md (part B) those diagnostics go to stderr, and
`crates/cvode/tests/diagnostics_stderr.rs` proves it by capturing a child process that reaches that path.
The isolation is kept as defense in depth: the protocol process never calls solver code; every
`solve`/`pgpe_run` call runs in `sundials-mcp --worker …`, whose stdout is a duplicate of the parent's
*stderr* handle (`AsFd::try_clone_to_owned`, safe std; redirecting file descriptors would need `unsafe`,
which this repository does not use) and whose result returns through a temporary file. A bonus: the
parent enforces a wall-clock timeout (`SUNDIALS_MCP_TIMEOUT_SECS`, default 120) by killing the worker.

`tests/stdio.rs` checks the property end-to-end: the worker binary run directly writes nothing to stdout
during a failing solve, and a session with isolation disabled (`SUNDIALS_MCP_NO_ISOLATION=1`, test-only)
keeps the protocol stream pure JSON-RPC. The earlier pair of tests, which needed a solve that printed to
stdout and had a control showing the corruption, was retired when the solver went silent on stdout.

## Observations recorded while building (upstream)

1. `exponential` (`y' = -y`) at `rtol = 1e-9, atol = 1e-12` used to fail with "too many error test
   failures at one step" and print `ERROR FAIL 3`. Fixed upstream by docs/CVODE_TIGHT_TOLERANCE_FIX.md;
   re-measured after that fix: 432 steps, max abs error 1.1e-9.
2. `robertson` with the LLNL configuration used to fail the same way at `rtol = 1e-9, atol = 1e-12`
   (same fix; re-measured: 837 steps, mass conservation 1.8e-15), and exhausts 500 000 steps by
   `t ≈ 0.014` at `atol = 1e-2` (inappropriate for `y2 ~ 1e-5`). Failures are reported as errors, never
   partial results.
3. `Method::Adams` never left order 1 (`compute_l` was a placeholder), and its error on a smooth
   quadrature ODE scaled like `√rtol` (2.1e-4 relative at `rtol = 1e-7`). Fixed by
   docs/CVODE_ADAMS_FIX.md (1.3e-8 on the same integrand). Many `tout`s from one BDF run also degrade
   accuracy (4e-6 vs 6e-7 with a fresh solve per output). `bao_distances` still uses BDF with one solve
   per redshift; measurements are in `crates/qf-bao-distances/README.md`.

## Build, test, register

```bash
cargo build --release -p sundials-mcp
cargo test --release -p sundials-mcp     # 13 unit + 5 stdio end-to-end tests
claude mcp add sundials -- /path/to/rusty-SUNDIALS/target/release/sundials-mcp
```

Protocol handshake versions implemented: 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25. Interoperability
with the official Python MCP SDK (2.2.0) client is tested from the SocrateAI-Scientific-SectorCausalityTheory
repository (`mcp-server/tests/test_interop_rusty_sundials.py`), which also documents co-deployment of this
server with that repository's theory server.
