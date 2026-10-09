# qf-pgpe-py — Python extension for the quantum-fluids engine

`import qf_pgpe` gives Python programmes the Rust PGPE engine (`qf-pgpe`) on numpy arrays, with no per-step Python round trip (`Pgpe.run` evolves a field for `t_end`
time units inside one call, GIL released), the periodic vortex imprint, sub-grid vortex detection and the registered transport estimators.

```python
import numpy as np, qf_pgpe
s = qf_pgpe.Pgpe(128, 64.0)                                  # n, L, g=1, dt=0.01, kcut_frac=0.5
c = s.imprint_v2(s.uniform(), np.array([[35., 20.], [25., 20.]]), np.array([1, -1]))
c = s.run(c, 100.0)
pos, q = s.detect(c)                                         # sub-grid positions and windings
est = qf_pgpe.analyse_tracks([(t, R, q)], 64.0)              # alpha (energy, regression), 1-alpha', eta with jackknife errors
```

Fields are C-contiguous `complex128` arrays `(n, n)` holding the projected Fourier amplitudes of `pgpe.PGPE` (numpy `fft2` convention), so existing scripts keep their data layout:
the SocrateAI-Scientific-QuantumFluids programme swaps the engine with one line (`from pgpe_rust import PGPERust as PGPE`).

## Build

```
cargo build --release -p qf-pgpe-py
mkdir -p /tmp/ext && cp target/release/libqf_pgpe.so /tmp/ext/qf_pgpe.so      # .dylib / .pyd on macOS / Windows
PYTHONPATH=/tmp/ext python3 crates/qf-pgpe-py/tests/smoke.py
```
(`maturin develop` also works; the crate is a plain pyo3 `cdylib`.) Not built on `wasm32`.

## Validated against the numpy engine (same inputs)

| quantity | difference |
|---|---|
| `uniform`, mode count (797 at n = 64, L = 32) | identical |
| `imprint_v2` | 1.2e-11 (max over modes) |
| `detect` positions | identical |
| `run` 20 time units, n = 64 | 1.0e-10 (energy 2e-14 relative, norm 6e-16) |
| `analyse_tracks` on two real tracks | bit-identical for four estimators, ≤ 2e-15 for the other two |
| `PGPERust` vs `PGPE` in `round2.run_blocks` (60 time units, measurement included) | same T, `n_s/n`, vortex count, energy to the printed digits; **5.3x faster end to end** (109.8 s -> 20.8 s on a loaded machine) |
