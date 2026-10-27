"""Positive controls for the lattice-phonon code (QU-1 gate).

usage: python -I phonon_controls.py BIN OUT.json
1. frozen-phonon test: the dynamical matrix against the second difference of the pair-sum energy of a commensurate
   5x5 supercell (V only, no derivatives; dipolar: Ewald supercell energy) at several shapes, k and polarisations;
2. Epstein-zeta closed forms (mpmath) against the static Riesz energies at the square and hexagonal lattices;
3. dipolar static energy against the Ewald sum;
4. cutoff convergence of E_zp (rc = 100, 200, 400, 800) at the triangular point and a stable off-triangular point;
5. modular invariance E(tau) = E(tau + 1) = E(-1/tau) of E_zp.
"""

import json
import math
import subprocess
import sys

import mpmath as mp

binp, outp = sys.argv[1:3]
S3 = math.sqrt(3.0)
TRI = (0.5, S3 / 2.0)


def run(*args):
    r = subprocess.run(["taskset", "-c", "6-7", binp, *map(str, args)], check=True, capture_output=True, text=True)
    return r.stdout


def point(spec, x, y, ns, rc):
    rows = run("point", spec, repr(x), repr(y), ns, rc).strip().splitlines()[1:]
    out = []
    for r in rows:
        p = r.split(",")
        out.append(
            {"n": int(p[3]), "rc": float(p[4]), "es": float(p[5]), "ez": float(p[6]), "ec": float(p[7]), "nu": int(p[9])}
        )
    return out


res = {}
# 1. frozen phonon
fr = []
cases = [
    ("gauss:0.6", 0.5, TRI[1], 10),
    ("gauss:0.6", 0.3, 1.2, 10),
    ("yukawa:1.5", 0.5, TRI[1], 14),
    ("yukawa:1.5", 0.2, 1.1, 14),
    ("riesz:4", 0.5, TRI[1], 60),
    ("riesz:3", 0.5, TRI[1], 60),
    ("riesz:6", 0.5, TRI[1], 40),
    ("lj:0.9", 0.5, TRI[1], 40),
    ("dipolar:0.1", 0.5, TRI[1], 400),
    ("dipolar:0.5", 0.5, TRI[1], 400),
    ("dipolar:0.5", 0.1, 1.2, 400),
]
for spec, x, y, rc in cases:
    for ij, e in (((1, 2), (0.6, 0.8)), ((2, 1), (1.0, 0.0)), ((1, 1), (0.0, 1.0))):
        j = json.loads(run("frozen", spec, repr(x), repr(y), 5, ij[0], ij[1], e[0], e[1], rc))
        fr.append(j)
res["frozen_phonon"] = fr
res["frozen_phonon_max_rel_diff"] = max(j["rel_diff"] for j in fr)

# 2. Epstein closed forms. Sum' (m^2+n^2)^(-w) = 4 zeta(w) beta(w); hexagonal 6 zeta(w) L(w, chi_-3)
mp.mp.dps = 30


def beta(w):
    return mp.nsum(lambda k: (-1) ** int(k) / (2 * k + 1) ** w, [0, mp.inf])


def l3(w):
    return mp.nsum(lambda k: 1 / (3 * k + 1) ** w - 1 / (3 * k + 2) ** w, [0, mp.inf])


ep = []
for s in (3, 4, 6):
    w = mp.mpf(s) / 2
    sq = 4 * mp.zeta(w) * beta(w)  # unit square lattice, covolume 1
    hex_unit = 6 * mp.zeta(w) * l3(w)  # hexagonal lattice with |a| = 1, covolume sqrt(3)/2
    hex_cov1 = hex_unit * (S3 / 2.0) ** (mp.mpf(s) / 2)  # rescale lengths by (sqrt3/2)^(-1/2): r^-s -> (sqrt3/2)^(s/2) r^-s
    for name, ref, (x, y) in (("square", sq, (0.0, 1.0)), ("hexagonal", hex_cov1, TRI)):
        row = point(f"riesz:{s}", x, y, "24", 400)[0]
        ep.append({"s": s, "lattice": name, "E_exact": float(ref / 2), "E_code": row["es"], "diff": row["es"] - float(ref / 2)})
res["epstein"] = ep

# 3. dipolar statics against Ewald
dp = []
for d in (0.1, 0.5):
    for x, y in ((0.5, TRI[1]), (0.0, 1.0), (0.2, 1.2)):
        dp.append(json.loads(run("ewald_static", d, repr(x), repr(y))))
res["dipolar_static_vs_ewald"] = dp

# 4. cutoff convergence of E_zp
cc = []
for spec in ("riesz:3", "riesz:6", "dipolar:0.1", "dipolar:0.5", "yukawa:1", "gauss:0.5"):
    for x, y in (TRI, (0.45, 0.93)):
        for rc in (100, 200, 400, 800):
            r = point(spec, x, y, "96", rc)[0]
            cc.append({"pot": spec, "x": x, "y": y, "rc": rc, "ez": r["ez"], "es": r["es"], "nu": r["nu"]})
res["cutoff_convergence"] = cc

# 5. modular invariance at a stable near-triangular point (all potentials)
mi = []
x, y = 0.46, 0.93
nr = x * x + y * y
for spec in ("gauss:1", "yukawa:1", "dipolar:0.5", "riesz:4"):
    a = point(spec, x, y, "96,192", 400)
    b = point(spec, x + 1.0, y, "96,192", 400)
    c = point(spec, -x / nr, y / nr, "96,192", 400)
    mi.append(
        {
            "pot": spec,
            "ez96": [a[0]["ez"], b[0]["ez"], c[0]["ez"]],
            "ez192": [a[1]["ez"], b[1]["ez"], c[1]["ez"]],
            "es": [a[0]["es"], b[0]["es"], c[0]["es"]],
        }
    )
res["modular_invariance"] = mi
json.dump(res, open(outp, "w"), indent=1)
print(json.dumps({k: v for k, v in res.items() if k in ("frozen_phonon_max_rel_diff", "epstein", "dipolar_static_vs_ewald")}, indent=1))
