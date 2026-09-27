# sundials-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server (stdio, JSON-RPC 2.0) that lets AI
agents and scientists run rusty-SUNDIALS solvers and read the results with their evidentiary status
made explicit. Pure `std` + `serde_json`; `#![forbid(unsafe_code)]`.

## Tools

| Tool | What it does | Checked against |
|---|---|---|
| `about` | scope, conventions, what is not exposed yet | — |
| `list_problems` | the named CVODE problems, parameters, bounds | — |
| `solve` | `exponential`, `robertson`, `vanderpol`, `lorenz` via CVODE (BDF) | exponential: closed form `exp(-t)`; robertson: mass conservation and the LLNL SUNDIALS `cvRoberts_dns` published output at `t = 0.4`; vanderpol, lorenz: no closed form, trajectory and statistics only |
| `pgpe_run` | `crates/qf-pgpe` projected Gross–Pitaevskii solver on a 16/32/64 grid, ≤ 4000 steps | plane wave: exact solution `exp(i(kx − ωt))`, `ω = k²/2 + g`; norm and momentum conservation |

Not exposed yet: the CMB dark-energy-transition bound (`crates/qf-cmb-cascade`) is in open PR #57 and
is not on `main`; a tool for it will follow once #57 merges. It is deliberately **not** stubbed.

## Honesty conventions

- Every result carries **`ran`**. `ran: false` means the computation did not execute (bad arguments,
  timeout, worker crash) — never read it as a pass.
- A **solver failure is an MCP tool error** (`isError: true`) carrying the solver's own message, with
  **no partial trajectory**.
- Each known-answer check **names its reference**, which is fixed independently of this code.

## Why solvers run in a worker subprocess

`crates/cvode/src/solver.rs` prints diagnostics to **stdout** on some failure paths
(`println!("ERROR FAIL 3: local error test failed > max times")`). On a stdio protocol that corrupts the
stream. Redirecting file descriptors would need `unsafe`, which this repository does not use. Instead the
protocol process never calls solver code: every `solve`/`pgpe_run` call runs in
`sundials-mcp --worker …`, whose stdout is a duplicate of the parent's *stderr* handle
(`AsFd::try_clone_to_owned`, safe std) and whose result returns through a temporary file. A bonus: the
parent enforces a wall-clock timeout (`SUNDIALS_MCP_TIMEOUT_SECS`, default 120) by killing the worker.

`tests/stdio.rs` reproduces the pitfall end-to-end: with isolation the protocol stream stays pure
JSON-RPC while the solver's `ERROR FAIL 3` appears on stderr; the control (`SUNDIALS_MCP_NO_ISOLATION=1`,
test-only) shows the corruption, so the test can fail.

## Observations recorded while building (upstream, not changed here)

1. `exponential` (`y' = -y`) at `rtol = 1e-9, atol = 1e-12` fails with "too many error test failures at
   one step" and prints `ERROR FAIL 3`; the default `1e-6 / 1e-10` works.
2. `robertson` with the LLNL configuration fails the same way at `rtol = 1e-9, atol = 1e-12`, and exhausts
   500 000 steps by `t ≈ 0.014` at `atol = 1e-2` (inappropriate for `y2 ~ 1e-5`). Both are reported as
   errors, never partial results.

## Build, test, register

```bash
cargo build --release -p sundials-mcp
cargo test --release -p sundials-mcp     # 10 unit + 4 stdio end-to-end tests
claude mcp add sundials -- /path/to/rusty-SUNDIALS/target/release/sundials-mcp
```

Protocol handshake versions implemented: 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25. Interoperability
with the official Python MCP SDK (2.2.0) client is tested from the SocrateAI-Scientific-SectorCausalityTheory
repository (`mcp-server/tests/test_interop_rusty_sundials.py`), which also documents co-deployment of this
server with that repository's theory server.
