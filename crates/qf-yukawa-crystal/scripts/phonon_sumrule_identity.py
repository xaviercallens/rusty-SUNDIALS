"""Sum rule <tr D>_BZ for the Gaussian versus theta functions (P2 gate G2; exact identities checked in floating point).

usage: python -I phonon_sumrule_identity.py OUT.json
<tr D> = sum_{R != 0} Lap V(R) = 4 pi a - (4 pi^2 / a) S(1/a; L), S(al; L) = sum_{P in L} |P|^2 e^{-pi al |P|^2}
(Poisson; the dual of a unit-covolume planar lattice is a rotation of it). At a = 1, Luo-Wei (arXiv:2302.05042,
Lemma 3.5) give S(1; L) = theta(1; L)/(2 pi), hence <tr D> = 4 pi - 2 pi theta(1; L). Checked here at several shapes.
"""

import json
import math
import sys

import numpy as np

PI = math.pi


def pts(x: float, y: float, rmax: float = 9.0) -> np.ndarray:
    sy = math.sqrt(y)
    L = int(rmax / sy) + 2
    M = int(rmax * sy + abs(x) * L / sy) + 3
    m, l = np.meshgrid(np.arange(-M, M + 1), np.arange(-L, L + 1), indexing="ij")
    X = (m + l * x) / sy
    Y = l * sy
    u = X * X + Y * Y
    return u[u < rmax * rmax].ravel()


def main() -> None:
    rows = []
    for x, y in ((0.5, math.sqrt(3) / 2), (0.5, 0.966025403784), (0.295, 0.9555), (0.0, 1.0), (0.1, 1.3)):
        u = pts(x, y)
        nz = u[u > 0]
        theta1 = float(np.exp(-PI * u).sum())
        S1 = float((u * np.exp(-PI * u)).sum())
        m2 = float(((-4 * PI + 4 * PI * PI * nz) * np.exp(-PI * nz)).sum())
        rows.append({"x": x, "y": y, "theta_1": theta1, "S_1": S1, "theta_1_over_2pi": theta1 / (2 * PI), "m2_pair_sum": m2, "m2_from_theta": 4 * PI - 2 * PI * theta1})
        print(json.dumps(rows[-1]), flush=True)
    json.dump(rows, open(sys.argv[1], "w"), indent=1)


if __name__ == "__main__":
    main()
