# qf-sandier-serfaty-table

Tier-X cross-check, with this repository's CVODE (BDF), of the `table` stage of the finite interval certificate in the
openai/math manuscript *Triangular minimality for planar Coulomb renormalized energy* (2026-09-23; AI-generated, not
peer reviewed; `verification/core.hpp`, SHA-256 `c7f76596...`). It belongs to AutoevolveAI's proposal 2, item A1
(the Lean port of that certificate; `results/proposal2/2026-10-26/A1-continuation/`).

## What is checked

`table(24000)` (core.hpp:246-267) propagates a state `(s1, s2, ps_k)`, `k = 6, 12, ..., 228`, from `t0 = small = p - 0.06`
to `l ≈ 0.6964`, using a midpoint rule with an outward remainder in 144-bit fixed-point interval arithmetic. Read from
the step (core.hpp:254-259) and from `solve` in `fh` (core.hpp:230), the state solves

```
s1'   = t b(t)
s2'   = -2 b~(t) + t b~(t)^2
ps_k' = b~(t) - k ps_k / t
```

with `b = basis(t, seg).first`, `b~ = b' + 1/t`, from `ss0(small)`. The basis switches at `small, p, R, R+.001, R+.075`.

`src/lib.rs` evaluates `b` and `b'` in f64 (forward-mode duals). It integrates the 40-dimensional ODE with
`crates/cvode` (BDF, a fresh solve per output, restarted at every breakpoint) and compares the result with the C++
enclosures `pointS[i]` at 437 of the 6,826 table nodes (node 1562, at `t = small`, and every 25th node after it, plus all
segment-boundary nodes and their predecessors). There are 441 evaluations, because the boundary nodes are checked once
per segment. The propagation covers nodes 1563-6825 (5,263 steps). The data are in `data/table_states.txt`: the coefficients are f64 midpoints, and the
enclosures have their exact endpoints rounded outward to f64. They come from AutoevolveAI's
`results/proposal2/2026-10-26/A1-continuation/cpp/dump2_driver.cpp`, which runs the unmodified corpus headers, through
`scripts/export_cvode_data.py`. The source dump sha256 is in the file header.

There are two comparisons:
* **global**: one chained integration from `ss0(small)`;
* **node to node**: from the midpoint of the enclosure at node `i-1` to node `i`.

## Result (`results/cross_check.json`, `cargo run --release --bin ss_table_cvode`)

| rtol (atol = 1e-4 rtol) | global: values outside / 17,640 | node to node: outside / 8,800 |
|---|---|---|
| 1e-8 | 3,008 | 290 |
| 1e-10 | 498 | 62 |
| 1e-12 | 8 (s1 only, at most 9.9e-13 outside) | 3 (s1, at most 4.5e-13) |
| **1e-13** | **0** | **0** |
| **1e-14** | **0** | **0** |

* Solver drift between rtol 1e-13 and 1e-14 is at most 6.1e-14 (s1), 9.9e-13 (s2) and 1.7e-13 (ps). The certificate's
  enclosure half-widths are 1e-14 to 1e-10 (s1), 1e-12 to 8e-9 (s2) and 7e-13 to 5e-8 (ps). Coarser tolerances fail
  only because the solver error is larger than these widths. The failures shrink steadily as the tolerance tightens.
* In the global run the solution sits close to one edge of some enclosures (max |position| 0.988 of the half-width
  for s1 and 0.999 for ps). This fits the one-signed midpoint remainder.
* **Negative controls**, at rtol 1e-13:
  * `b, b~ -> (1 + 1e-6) b, b~`: 17,600 of 17,640 global values fall outside (all except the 40 at the initial
    node), by up to 1.4e-7. That is more than 10^5 times the solver drift.
  * `k -> k (1 + 1e-6)` in the `ps` equation: 3,342 `ps` values fall outside, by up to 1.1e-7, while `s1, s2` stay inside.

`tests/cross_check.rs` asserts the positive control (0 outside, both comparisons, rtol 1e-13), both negative
controls, the data shape, and the dual-number derivative against a central difference.

## Limits

This is evidence, not proof. f64 and CVODE carry no error bound, and the ODE right-hand side uses f64 midpoints of
the certificate's coefficients. What it shows is that the reading of the propagation as the ODE above agrees with the
certificate's enclosures to the solver's resolution. It also shows that the enclosures are tight enough for a 1e-6
change in the right-hand side to be detected. It does not check `fh` (the point and cell values `pts`, `cels`), only
the propagated states.
