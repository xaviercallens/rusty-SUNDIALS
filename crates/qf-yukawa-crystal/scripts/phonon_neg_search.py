"""Search for a counterexample of QU-1 among non-completely-monotone potentials (negative control).

usage: python -I phonon_neg_search.py BIN TAUFILE OUT.json
Family: V = -ring(r1, w) - c ring(r2, w) with r1 = 1 (square nearest-neighbour distance), r2 = sqrt 2 (square
next-nearest distance), i.e. two attractive Gaussian wells placed where the unit-covolume square lattice has its first
two neighbour shells; the triangular lattice has 6 neighbours at 1.0746 and 6 at 1.861. For each (w, c) the shape
domain is scanned at n = 48 and the triangular lattice is compared with every other lattice: (i) static minimiser,
(ii) dynamically stable lattices, (iii) the lowest E_zp among stable lattices. The parameter selection is the grid below;
nothing is tuned beyond it.
"""

import csv
import json
import math
import subprocess
import sys

binp, taufile, outp = sys.argv[1:4]
RHO = (0.5, math.sqrt(3.0) / 2.0)
res = []
for w in (0.08, 0.12, 0.2, 0.3):
    for c in (0.0, 0.5, 1.0):
        spec = f"wells:1.0,{math.sqrt(2.0):.12f},{w},{c}"
        dst = f"/tmp/neg_{w}_{c}.csv"
        subprocess.run(["taskset", "-c", "6-7", binp, "scan", spec, "48", "400", taufile, dst], check=True)
        rows = []
        with open(dst) as f:
            for r in csv.DictReader(f):
                rows.append({k: (float(v) if k not in ("pot",) else v) for k, v in r.items()})
        tri = min(rows, key=lambda r: math.hypot(r["x"] - RHO[0], r["y"] - RHO[1]))
        smin = min(rows, key=lambda r: r["e_static"])
        stable = [r for r in rows if r["n_unstable"] == 0]
        item = {
            "w": w,
            "c": c,
            "tri": {"es": tri["e_static"], "ez": tri["e_zp"], "unstable_pts": tri["n_unstable"]},
            "static_min": {"x": smin["x"], "y": smin["y"], "es": smin["e_static"], "unstable_pts": smin["n_unstable"]},
            "n_stable": len(stable),
            "n_points": len(rows),
        }
        if stable:
            zmin = min(stable, key=lambda r: r["e_zp"])
            item["zp_min_stable"] = {"x": zmin["x"], "y": zmin["y"], "ez": zmin["e_zp"], "es": zmin["e_static"]}
        res.append(item)
        print(json.dumps(item), flush=True)
json.dump(res, open(outp, "w"), indent=1)
