"""Exact long-wave (elastic) stability limit for the QU-1 competitors of every CM potential (P2 gate G2, Tier X).

usage: python -I phonon_elastic_check.py OUT.json

lim_{k->0} D(k)/|k|^2 along the direction t is C(t) = (1/2) sum_{R != 0} (t.R)^2 H(R), H = Hess V, an absolutely convergent
sum for every potential here (Riesz s >= 3 and the dipolar 1/r^3 tail give sum r^{-3}). No k-grid is involved and no
difference of large sums is formed, so there is no cancellation. The sum runs over |R| < rc and the continuum tail
(1/2) int_{|r|>rc} (t.r)^2 H(r) d^2r = (1/2) int_rc^inf r^3 [ pi V'/r I + (pi/4)(V'' - V'/r)(I + 2 t t^T) ] dr is added
(quad). V', V'' come from sympy differentiation of V (independent of phonon.rs). Reported: min over 3600 directions of
lambda_min C(t), at rc = 200 and 400. Positive = stable at long wavelength. Controls: gauss values against the Poisson
long-wave rows of phonon_poisson_check.py; the square lattice (negative control) must give a negative value.
"""

import os

for _v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "2"

import json  # noqa: E402
import math  # noqa: E402
import sys  # noqa: E402

import numpy as np  # noqa: E402
import sympy as sp  # noqa: E402
from scipy.integrate import quad  # noqa: E402

PI = math.pi
S3 = math.sqrt(3.0)
r = sp.symbols("r", positive=True)


def potential(spec: str):
    name, par = spec.split(":")
    p = float(par)
    if name == "gauss":
        V = sp.exp(-sp.pi * p * r**2)
    elif name == "yukawa":
        V = sp.exp(-p * r) / r
    elif name == "riesz":
        V = r ** (-int(p))
    elif name == "dipolar":
        s = sp.sqrt(r**2 + p**2)
        V = 2 / (r * s * (r + s))
    else:
        raise ValueError(spec)
    d1 = sp.diff(V, r)
    d2 = sp.diff(V, r, 2)
    return sp.lambdify(r, d1, "numpy"), sp.lambdify(r, d2, "numpy"), name


def lattice(x: float, y: float, rc: float) -> np.ndarray:
    sy = math.sqrt(y)
    L = int(rc / sy) + 2
    M = int(rc * sy + abs(x) * L / sy) + 2
    m, l = np.meshgrid(np.arange(-M, M + 1), np.arange(-L, L + 1), indexing="ij")
    X = (m + l * x) / sy
    Y = l * sy
    rr = np.hypot(X, Y)
    sel = (rr > 0) & (rr < rc)
    return np.stack([X[sel], Y[sel]], 1)


def elastic_min(spec: str, x: float, y: float, rc: float) -> dict:
    d1, d2, name = potential(spec)
    R = lattice(x, y, rc)
    rr = np.hypot(R[:, 0], R[:, 1])
    v1 = d1(rr)
    v2 = d2(rr)
    a = v1 / rr
    b = v2 - a
    nx, ny = R[:, 0] / rr, R[:, 1] / rr
    H = [a + b * nx * nx, b * nx * ny, a + b * ny * ny]
    # S[(p,q)][c] = sum R_p R_q H_c
    S = {}
    for pq, w in (("xx", R[:, 0] * R[:, 0]), ("xy", R[:, 0] * R[:, 1]), ("yy", R[:, 1] * R[:, 1])):
        S[pq] = [float((w * h).sum()) for h in H]
    if name in ("gauss", "yukawa") and abs(v1[rr > rc - 1].max(initial=0.0)) < 1e-300:
        tI = tT = 0.0
    else:
        tI = 0.5 * quad(lambda q: q**3 * (PI * d1(q) / q + (PI / 4) * (d2(q) - d1(q) / q)), rc, np.inf, limit=400)[0]
        tT = 0.5 * quad(lambda q: q**3 * (PI / 2) * (d2(q) - d1(q) / q), rc, np.inf, limit=400)[0]
    th = np.linspace(0, PI, 3600, endpoint=False)
    c, s = np.cos(th), np.sin(th)
    C = [0.5 * (c * c * S["xx"][i] + 2 * c * s * S["xy"][i] + s * s * S["yy"][i]) for i in range(3)]
    C[0] = C[0] + tI + tT * c * c
    C[1] = C[1] + tT * c * s
    C[2] = C[2] + tI + tT * s * s
    lo = 0.5 * (C[0] + C[2]) - np.sqrt(0.25 * (C[0] - C[2]) ** 2 + C[1] ** 2)
    i = int(np.argmin(lo))
    return {"min_lam": float(lo[i]), "at_theta": float(th[i]), "tail_I": tI, "tail_T": tT}


def main() -> None:
    out = sys.argv[1]
    tri = (0.5, S3 / 2)
    c1 = (0.5, 0.966025403784)
    c3 = (0.3, 0.953939201417)
    sq = (0.0, 1.0)
    cases = {
        "gauss:0.25": [tri, (0.5, 0.871), sq],
        "gauss:0.5": [tri, c1, (0.5, 1.016), c3, sq],
        "gauss:1": [tri, c1, (0.295, 0.9555), c3, sq],
        "gauss:2": [tri, (0.5, 0.871), sq],
        "gauss:4": [tri, (0.5, 0.871), sq],
        "riesz:3": [tri, c3, (0.5, 1.036), sq],
        "riesz:4": [tri, c3, sq],
        "riesz:6": [tri, (0.5, 0.871), sq],
        "yukawa:1": [tri, c3, (0.295, 0.9555), sq],
        "yukawa:2": [tri, c3, sq],
        "dipolar:0.1": [tri, c3, sq],
        "dipolar:0.5": [tri, c3, sq],
    }
    res = []
    for spec, pts in cases.items():
        for x, y in pts:
            row = {"pot": spec, "x": x, "y": y}
            for rc in (200.0, 400.0):
                row[f"rc{int(rc)}"] = elastic_min(spec, x, y, rc)
            row["stable_longwave"] = bool(row["rc400"]["min_lam"] > 0)
            res.append(row)
            print(json.dumps(row), flush=True)
            json.dump(res, open(out, "w"), indent=1)


if __name__ == "__main__":
    main()
