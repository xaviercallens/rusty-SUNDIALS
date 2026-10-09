"""Regenerates thermal_N32.{raw,json} and thermal_N32_run20_occ.raw (Python reference values for tests/thermal_crosscheck.rs).
Run with the QuantumFluids venv; the paths below assume the author layout."""
import sys, json
sys.path.insert(0, "/home/xavkal/xdev/SocrateAI-Scientific-QuantumFluids/exploration/pgpe")
import numpy as np
from pgpe import PGPE
from observables import thermometer, condensate_fraction, vortices, current_correlators
sys.path.insert(0, "/home/xavkal/xdev/SocrateAI-Scientific-QuantumFluids/exploration/pgpe")
from make_friction_bases import resample
OUT = "/home/xavkal/xdev/rusty-sundials-wt/thermal/crates/qf-pgpe/tests/fixtures/"
s = PGPE(N=32, L=16.0)
rng = np.random.default_rng(1)
c = s.random_state(1.0, 1.5, rng)
c = s.run(c, 100.0)
c.astype(np.complex128).tofile(OUT + "thermal_N32.raw")
occ = np.abs(c) ** 2 * s.dx ** 2 / s.N ** 2
T1, o1 = thermometer(s, occ, 0.6, 1.0); T2, o2 = thermometer(s, occ, 0.4, 0.6)
cr = current_correlators(s, c)
pos, q = vortices(s, c)
psi = s.psi(c)
r64 = resample(psi, 64)
back = resample(r64, 32)
js = {"n": 32, "l": 16.0, "g": 1.0, "dt": 0.01, "norm": s.norm(c), "energy": s.energy(c),
      "T_hi": T1, "off_hi": o1, "T_lo": T2, "off_lo": o2, "cond": condensate_fraction(s, c),
      "shells": [{"shell": k, "jl": v[0], "jt": v[1]} for k, v in cr.items()], "n_v": int(len(q)),
      "resample64": {"sum2": float(np.sum(np.abs(r64) ** 2)), "sum4": float(np.sum(np.abs(r64) ** 4)),
                     "pts": [[i, j, r64[i, j].real, r64[i, j].imag] for (i, j) in [(0, 0), (5, 7), (33, 60), (63, 1)]],
                     "roundtrip_maxdiff": float(np.max(np.abs(back - psi)))},
      }
c2 = s.run(c, 20.0)
(np.abs(c2) ** 2 * s.dx ** 2 / s.N ** 2).astype(np.float64).tofile(OUT + "thermal_N32_run20_occ.raw")
js["run20"] = {"norm": s.norm(c2), "energy": s.energy(c2), "t": 20.0}
json.dump(js, open(OUT + "thermal_N32.json", "w"), indent=1)
print(json.dumps(js, indent=1))
