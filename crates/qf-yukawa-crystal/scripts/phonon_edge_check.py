"""Stability-edge verification and dense edge scans for the QU-1 gate.

usage: python -I phonon_edge_check.py BIN WORKDIR
1. For every CM potential, the two candidate competitors (0.3 + 0.953939 i, the largest sampled margin and the lowest-x
   stable sample, and 0.5 + 0.966025 i) are recomputed on k-grids n = 96, 192, 384 at rc = 400 and 800; stability
   (no eigenvalue below -1e-9 x scale) and dE_zp = E_zp(tau) - E_zp(rho) at the same n are recorded.
2. Dense 1-D scans (step 0.005, n = 96, rc = 400) along the arc |tau| = 1 from x = 0.5 down to x = 0.2 and along
   Re tau = 0.5 from y = sqrt(3)/2 up to 1.1. The lowest stable dE_zp on each path is recomputed at n = 192, 384.
Writes WORKDIR/edge_check.json and the tau files in WORKDIR/taus_edge.
"""

import csv
import json
import math
import os
import subprocess
import sys

binp, wd = sys.argv[1:3]
cores = os.environ.get("LP_CORES", "4-7")
S3 = math.sqrt(3.0) / 2.0
POTS = (
    [f"gauss:{a}" for a in (0.25, 0.5, 1, 2, 4)]
    + [f"yukawa:{k}" for k in (1, 2)]
    + [f"dipolar:{d}" for d in (0.1, 0.5)]
    + [f"riesz:{s}" for s in (3, 4, 6)]
)
os.makedirs(os.path.join(wd, "taus_edge"), exist_ok=True)


def write_taus(name, pts):
    p = os.path.join(wd, "taus_edge", name)
    with open(p, "w") as f:
        for x, y in pts:
            f.write(f"{x:.12f} {y:.12f}\n")
    return p


def scan(spec, ns, rc, taufile):
    dst = os.path.join(wd, "taus_edge", "tmp_scan.csv")
    subprocess.run(["taskset", "-c", cores, binp, "scan", spec, ns, str(rc), taufile, dst], check=True)
    rows = []
    with open(dst) as f:
        for r in csv.DictReader(f):
            rows.append(
                {
                    "x": float(r["x"]),
                    "y": float(r["y"]),
                    "n": int(r["n"]),
                    "ez": float(r["e_zp"]),
                    "nu": int(r["n_unstable"]),
                }
            )
    os.remove(dst)
    return rows


tri_file = write_taus("tri.txt", [(0.5, S3)])
cand = [(0.3, 0.953939201417), (0.5, 0.966025403784)]
cand_file = write_taus("cand.txt", cand)
arc = []
x = 0.5
while x >= 0.2 - 1e-9:
    arc.append((x, math.sqrt(1.0 - x * x)))
    x -= 0.005
line = []
y = S3
while y <= 1.1 + 1e-9:
    line.append((0.5, y))
    y += 0.005
arc_file = write_taus("arc.txt", arc)
line_file = write_taus("line.txt", line)
res = {}
for spec in POTS:
    item = {"candidates": [], "paths": {}}
    for rc in (400, 800):
        tri = {r["n"]: r for r in scan(spec, "96,192,384", rc, tri_file)}
        for r in scan(spec, "96,192,384", rc, cand_file):
            item["candidates"].append(
                {
                    "rc": rc,
                    "x": r["x"],
                    "y": r["y"],
                    "n": r["n"],
                    "stable": r["nu"] == 0,
                    "dEzp": (r["ez"] - tri[r["n"]]["ez"]) if r["nu"] == 0 else None,
                }
            )
    tri96 = scan(spec, "96", 400, tri_file)[0]["ez"]
    for name, pts, tf in (("arc", arc, arc_file), ("line", line, line_file)):
        rows = scan(spec, "96", 400, tf)
        st = [r for r in rows if r["nu"] == 0 and math.hypot(r["x"] - 0.5, r["y"] - S3) > 1e-9]
        entry = {"n_points": len(rows), "n_stable": len(st) + sum(1 for r in rows if math.hypot(r["x"] - 0.5, r["y"] - S3) <= 1e-9)}
        if st:
            best = min(st, key=lambda r: r["ez"])
            entry["min_stable_dEzp_n96"] = best["ez"] - tri96
            entry["argmin"] = [best["x"], best["y"]]
            bf = write_taus("best.txt", [(best["x"], best["y"])])
            tri_n = {r["n"]: r for r in scan(spec, "192,384", 400, tri_file)}
            ref = scan(spec, "192,384", 400, bf)
            entry["refined"] = [
                {"n": r["n"], "stable": r["nu"] == 0, "dEzp": (r["ez"] - tri_n[r["n"]]["ez"]) if r["nu"] == 0 else None} for r in ref
            ]
            entry["stable_x_range"] = [min(r["x"] for r in st), max(r["x"] for r in st)]
            entry["stable_y_range"] = [min(r["y"] for r in st), max(r["y"] for r in st)]
        item["paths"][name] = entry
    res[spec] = item
    print(spec, json.dumps(item["paths"]), flush=True)
json.dump(res, open(os.path.join(wd, "edge_check.json"), "w"), indent=1)
