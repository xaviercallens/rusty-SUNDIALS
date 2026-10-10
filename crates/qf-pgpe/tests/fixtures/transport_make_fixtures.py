#!/usr/bin/env python3
"""Regenerates the fixtures of tests/transport_crosscheck.rs from the Python original.

    OMP_NUM_THREADS=1 QF=/path/to/SocrateAI-Scientific-QuantumFluids $QF/.venv/bin/python transport_make_fixtures.py

Four real tracks of the vortex-transport campaign are trimmed to 800 samples and written as the CSV of
`qf_pgpe::vortex::csv` (positions with 6 decimals); the reference estimates of `transport_estimators.analyse_tracks`
are computed FROM THE ROUNDED CSV (so the Rust comparison is exact up to floating-point rounding) and stored in
transport_reference.json (NaN written as null). The same script prints the point values hard-coded in the unit tests.
"""
import csv, json, os, sys
from pathlib import Path
import numpy as np

QF = Path(os.environ.get("QF", "/home/xavkal/xdev/SocrateAI-Scientific-QuantumFluids"))
sys.path.insert(0, str(QF / "exploration/pgpe"))
from analyze_transport import load
from transport_estimators import analyse_tracks, grad_h, h_pair, pv_velocity, pv_energy

HERE = Path(__file__).resolve().parent
TR = QF / "data/generated/pgpe/transport"
L = 64.0
# (source file, first sample, last sample (exclusive), fixture name)
SRC = [
    ("fl/FL_third_base_k2.094_N128_e0.60_antiparallel_d12_s2.npz", 0, 800, "transport_fl_e060_d12_s2.csv"),        # t_settle drops 0..99
    ("fl/FL_third_base_k2.094_N128_e0.55_antiparallel_d12_s1.npz", 1000, 1800, "transport_fl_e055_d12_s1.csv"),
    ("prod_e0.60_d12_s1.npz", 0, 800, "transport_prod_e060_d12_s1.csv"),
    ("prod_e0.60_d8_s2.npz", 1000, 1800, "transport_prod_e060_d8_s2.csv"),                                       # validity cut at t = 1367
]


def write_csv(path, t, R, n_det, P, Phi):
    nv = R.shape[1]
    with open(path, "w") as f:
        f.write("t" + "".join(f",x{i},y{i}" for i in range(nv)) + ",n_det,Px,Py,Px_hi,Py_hi\n")
        for k in range(len(t)):
            f.write(repr(float(t[k])) + "".join(f",{R[k, i, 0]:.6f},{R[k, i, 1]:.6f}" for i in range(nv)))
            f.write(f",{int(n_det[k])},{P[k,0]:.6f},{P[k,1]:.6f},{Phi[k,0]:.6f},{Phi[k,1]:.6f}\n")


def read_csv(path, nv):
    rows = list(csv.reader(open(path)))[1:]
    t = np.array([float(r[0]) for r in rows])
    R = np.array([[[float(r[1 + 2 * i]), float(r[2 + 2 * i])] for i in range(nv)] for r in rows])
    return t, R


def clean(o):
    if isinstance(o, dict):
        return {k: clean(v) for k, v in o.items()}
    if isinstance(o, (list, tuple)):
        return [clean(v) for v in o]
    if isinstance(o, float) and not np.isfinite(o):
        return None
    return o


tracks, names = [], []
for src, a, b, name in SRC:
    t, R, q, z, meta = load(TR / src)
    write_csv(HERE / name, t[a:b], R[a:b], z["n_det"][a:b], z["P"][a:b], z["P_hi"][a:b])
    t2, R2 = read_csv(HERE / name, R.shape[1])
    tracks.append((t2, R2, q)); names.append(name)

ref = {"l": L, "q": [int(v) for v in tracks[0][2]], "files": names, "single": {}, "pairs": {}}
for nm, tr in zip(names, tracks):
    ref["single"][nm] = clean(analyse_tracks([tr], L))
ref["pairs"]["0+3"] = clean(analyse_tracks([tracks[0], tracks[3]], L))
ref["all"] = clean(analyse_tracks(tracks, L))
ref["all_lag5_settle200_dvalid6"] = clean(analyse_tracks(tracks, L, lag=5, t_settle=200.0, d_valid=6.0, lags_eta=(4, 8, 16, 24, 32, 64, 128, 256)))
(HERE / "transport_reference.json").write_text(json.dumps(ref, indent=1))
print("all:", {k: v for k, v in ref["all"].items() if k != "msd"})

# point values for the unit tests (12 significant digits)
print("grad_h / h_pair")
for x, y in [(0.5, 0.3), (-1.2, 2.0), (3.0, -2.5), (0.05, 0.02), (7.0, -9.0), (0.6, 0.0)]:
    gx, gy = grad_h(np.array(x), np.array(y)); print(f"({x!r}, {y!r}, {float(gx):.12e}, {float(gy):.12e}, {float(h_pair(np.array(x), np.array(y))):.12e}),")
pos = np.array([[10.3, 20.1], [22.4, 21.7], [40.9, 33.3], [55.2, 8.4]]); qq = np.array([1, -1, 1, -1])
v = pv_velocity(pos, qq, 64.0); print("pv_velocity 4 vortices"); print([(f"{a:.12e}", f"{b:.12e}") for a, b in v])
print("pv_energy 4", f"{pv_energy(pos, qq, 64.0):.12e}")
pos2 = np.array([[30.0, 31.0], [38.0, 31.5]]); q2 = np.array([1, -1]); v2 = pv_velocity(pos2, q2, 64.0)
print("pv_velocity 2 vortices"); print([(f"{a:.12e}", f"{b:.12e}") for a, b in v2]); print("pv_energy 2", f"{pv_energy(pos2, q2, 64.0):.12e}")
