# `vortex_transport` example — cross-check against the Python instrument

Source of truth: `SocrateAI-Scientific-QuantumFluids/exploration/pgpe/vortex_transport.py` (`imprint_v2`, `detect`,
`momentum`), the instrument of the vortex-transport campaign (paper: *Vortex transport and screening in a closed
two-dimensional Bose field*, DOI 10.5281/zenodo.23202455; pre-registration `docs/designs/PGPE_FRICTION_PREREG.md`
amendments A1/A1.1).

## Known answer (`cargo run --release -p qf-pgpe --example vortex_transport -- validate`)

Gate G1 of the campaign at T = 0 (uniform condensate, single pair `d0 = 10`, L = 64, N = 128):

| quantity | Rust | Python campaign value |
|---|---|---|
| vortices detected after the imprint | 2 | 2 |
| field momentum / (2π n d) after the imprint | 0.9579 | 0.958 (static sweep at d = 10), 0.96–0.99 (G1 run) |
| separation, t = 10 → 100 | 9.837 → 9.867 | 9.815 → 9.821 over 400 time units |
| field momentum, t = 0 → 100 | constant to < 1e-6 (relative) | constant to 3e-10 |

PASS criteria coded in the example: 2 vortices; momentum ratio in [0.95, 1.0]; separation change < 0.1; relative
momentum change < 1e-6; run reaches `t_max`.

## Direct numeric comparison (same imprint positions, 2026-10-07)

Rust run `T0 --geom dipole --d0 10 --t-max 30 --seed 1`, positions written to `OUT.pos0`; Python `imprint_v2` at
those positions on the same grid, then `PGPE.run(c, 30)`:

| | t = 0 | t = 30 |
|---|---|---|
| detected vortices (Python / Rust) | 2 / 2 | 2 / 2 |
| max difference of sub-grid positions | 2.8e-7 | 5.3e-7 |
| difference of total momentum (Px, Py) | 1.4e-7, 3.6e-7 | 1.4e-7, 3.6e-7 |
| difference of band momentum (|k| > 1) | 8.5e-8 | 6.2e-8 |

The residual is the CSV's 6-decimal rounding; the two implementations are the same formulas (theta-function
phase with boundary-jump removal, Bernoulli amplitude, plaquette winding with least-squares plane refinement).

## Not ported

The estimators (`transport_estimators.py`: energy estimator of α, two-coefficient regression for 1 − α′, residual
MSD for η) stay in Python and read the CSV; porting them was not asked and would duplicate ~200 lines of
validated analysis code. The placement generator is a small LCG, documented as not numpy-identical (the
campaign's claims are ensemble statements).
