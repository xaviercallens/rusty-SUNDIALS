"""Long-wavelength (elastic) stability of the headline competitors, independent of any k-grid.

usage: python -I phonon_longwave_check.py OUT.json
For Gaussian a = 1 and a = 0.5 and Yukawa kappa = 1, 2 at tau = 0.5 + 0.966025 i, 0.5 + 1.016 i and 0.5 + 1.036 i, evaluate
D(k) = sum_R (1 - cos k.R) Hess V(R) (explicit sum, no folding) for k = eps (cos t, sin t), 721 directions t in [0, 2 pi],
eps = 1e-3, 1e-2, and report min over t of lambda_min(D)/eps^2 (the sound-speed-squared of the softer branch). A negative
value would mean an elastic instability that a finite k-grid can miss.
"""

import json
import math
import sys

import numpy as np

out = sys.argv[1]


def basis(x, y):
    sy = math.sqrt(y)
    a1 = np.array([1.0 / sy, 0.0])
    a2 = np.array([x / sy, sy])
    return a1, a2


def pot(kind, par):
    if kind == "gauss":

        def f(r):
            v = np.exp(-math.pi * par * r * r)
            return v, -2 * math.pi * par * r * v, (-2 * math.pi * par + 4 * math.pi**2 * par**2 * r * r) * v

        return f, math.sqrt(41 / (math.pi * par))

    def f(r):
        e = np.exp(-par * r)
        return e / r, -e * (par * r + 1) / r**2, e * (par**2 * r * r + 2 * par * r + 2) / r**3

    return f, 42.0 / par


def vectors(x, y, f, rc):
    a1, a2 = basis(x, y)
    M = int(rc * math.sqrt(y) * 1.5 + abs(x) * rc + 6)
    L = int(rc / math.sqrt(y) + 3)
    m, l = np.meshgrid(np.arange(-M, M + 1), np.arange(-L, L + 1), indexing="ij")
    R = m[..., None] * a1 + l[..., None] * a2
    r = np.hypot(R[..., 0], R[..., 1])
    sel = (r > 0) & (r < rc)
    R, r = R[sel], r[sel]
    _, v1, v2 = f(r)
    nx, ny = R[:, 0] / r, R[:, 1] / r
    a = v1 / r
    b = v2 - a
    return R, np.stack([a + b * nx * nx, b * nx * ny, a + b * ny * ny], axis=1)


res = []
for kind, par in (("gauss", 1.0), ("gauss", 0.5), ("yukawa", 1.0), ("yukawa", 2.0)):
    f, rc = pot(kind, par)
    for x, y in ((0.5, 0.966025403784), (0.5, 1.016025403784), (0.5, 1.036025403784), (0.5, 0.8660254037844386)):
        R, H = vectors(x, y, f, rc)
        item = {"pot": f"{kind}:{par}", "x": x, "y": y}
        for eps in (1e-3, 1e-2):
            t = np.linspace(0, 2 * math.pi, 721)
            k = eps * np.stack([np.cos(t), np.sin(t)], axis=1)
            c = 1.0 - np.cos(k @ R.T)
            D = c @ H
            mid = 0.5 * (D[:, 0] + D[:, 2])
            rad = np.sqrt(0.25 * (D[:, 0] - D[:, 2]) ** 2 + D[:, 1] ** 2)
            item[f"min_lambda_over_eps2_{eps}"] = float((mid - rad).min() / eps**2)
        res.append(item)
        print(json.dumps(item), flush=True)
json.dump(res, open(out, "w"), indent=1)
