"""Independent numpy re-computation of E_zp (no folding, no Rust) for the QU-1 gate.

usage: python -I phonon_independent_check.py OUT.json
For Gaussian a=1 and Yukawa kappa=1 it evaluates, by an explicit real-space sum over R (|R| < rc) of
(1 - cos k.R) Hess V(R) on (i) Gamma-centred and (ii) half-shifted k-grids, E_zp and the minimum eigenvalue of D(k) over
k != 0 for the triangular lattice and for stable competitors found by the Rust scan. Compare with the Rust values.
"""

import json
import math
import sys

import numpy as np

out = sys.argv[1]
S3 = math.sqrt(3.0)


def basis(x, y):
    sy = math.sqrt(y)
    a1 = np.array([1.0 / sy, 0.0])
    a2 = np.array([x / sy, sy])
    # b_i . a_j = 2 pi delta_ij
    A = np.array([a1, a2])
    B = 2 * math.pi * np.linalg.inv(A).T
    return a1, a2, B[0], B[1]


def pot(kind, par):
    if kind == "gauss":

        def f(r):
            v = np.exp(-math.pi * par * r * r)
            return v, -2 * math.pi * par * r * v, (-2 * math.pi * par + 4 * math.pi**2 * par**2 * r * r) * v

        return f, math.sqrt(41 / (math.pi * par))
    if kind == "yukawa":

        def f(r):
            e = np.exp(-par * r)
            return e / r, -e * (par * r + 1) / r**2, e * (par**2 * r * r + 2 * par * r + 2) / r**3

        return f, 42.0 / par
    raise ValueError(kind)


def dyn_vectors(x, y, f, rc):
    a1, a2, b1, b2 = basis(x, y)
    M = int(rc * math.sqrt(y) * 1.5 + abs(x) * rc + 6)
    L = int(rc / math.sqrt(y) + 3)
    m, l = np.meshgrid(np.arange(-M, M + 1), np.arange(-L, L + 1), indexing="ij")
    R = m[..., None] * a1 + l[..., None] * a2
    r = np.hypot(R[..., 0], R[..., 1])
    sel = (r > 0) & (r < rc)
    R = R[sel]
    r = r[sel]
    _, v1, v2 = f(r)
    nx, ny = R[:, 0] / r, R[:, 1] / r
    a = v1 / r
    b = v2 - a
    H = np.stack([a + b * nx * nx, b * nx * ny, a + b * ny * ny], axis=1)
    return R, H, b1, b2


def ezp(x, y, f, rc, n, shift):
    R, H, b1, b2 = dyn_vectors(x, y, f, rc)
    idx = (np.arange(n) + shift) / n
    ii, jj = np.meshgrid(idx, idx, indexing="ij")
    K = ii.reshape(-1, 1) * b1 + jj.reshape(-1, 1) * b2
    tot = 0.0
    lam_min = np.inf
    cnt = 0
    for s in range(0, len(K), 512):
        k = K[s : s + 512]
        c = 1.0 - np.cos(k @ R.T)
        D = c @ H  # (nk, 3): xx, xy, yy
        mid = 0.5 * (D[:, 0] + D[:, 2])
        rad = np.sqrt(0.25 * (D[:, 0] - D[:, 2]) ** 2 + D[:, 1] ** 2)
        lo, hi = mid - rad, mid + rad
        nz = np.hypot(k[:, 0], k[:, 1]) > 1e-12
        if nz.any():
            lam_min = min(lam_min, lo[nz].min())
        tot += np.sqrt(np.maximum(lo, 0)).sum() + np.sqrt(np.maximum(hi, 0)).sum()
        cnt += len(k)
    return 0.5 * tot / cnt, lam_min


res = []
cases = [
    ("gauss", 1.0, [(0.5, S3 / 2), (0.5, 0.966025403784), (0.3, 0.953939201417), (0.55, 0.866025403784), (0.45, 0.8160254)]),
    ("yukawa", 1.0, [(0.5, S3 / 2), (0.5, 0.966025403784), (0.3, 0.953939201417)]),
]
for kind, par, pts in cases:
    f, rc = pot(kind, par)
    for x, y in pts:
        row = {"pot": f"{kind}:{par}", "x": x, "y": y}
        for n, shift in ((64, 0.0), (128, 0.0), (127, 0.5), (254, 0.5)):
            e, lm = ezp(x, y, f, rc, n, shift)
            row[f"n{n}_s{shift}"] = {"ezp": e, "lam_min_nonzero_k": lm}
        res.append(row)
        print(json.dumps(row), flush=True)
json.dump(res, open(out, "w"), indent=1)
