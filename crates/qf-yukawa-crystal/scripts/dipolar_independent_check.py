"""Independent (Python, no shared code) evaluation of the triangular and square lattice energies of the rescaled
dipolar-exciton potential U(r) = 2 (1/r - 1/sqrt(r^2+d^2)) / d^2 at density 1, as the positive control for the Rust
Ewald sums in crates/qf-yukawa-crystal/src/dipolar_ewald.rs.

Two methods, both written from scratch here:
 (A) direct lattice sum over |v| < R (numpy, chunked) plus the uniform tail 2 pi / (sqrt(R^2+d^2) + R), for several R;
     the remaining discrete-minus-continuum residual oscillates in R and is reported, not hidden.
 (B) Ewald with adaptive scipy.integrate.quad for the reciprocal kernels and a different split parameter (eta = 1.0)
     than the Rust default (0.8).

Usage: python -I dipolar_independent_check.py <references.jsonl> <out.json>
"""
from __future__ import annotations

import json
import math
import sys

import numpy as np
from scipy.integrate import quad
from scipy.special import erfc

A_TRI = math.sqrt(2.0 / math.sqrt(3.0))


def u(d: float, r: np.ndarray) -> np.ndarray:
    s = np.sqrt(r * r + d * d)
    return 2.0 / (r * s * (r + s))


def direct(d: float, b1: tuple[float, float], b2: tuple[float, float], R: float) -> float:
    area = abs(b1[0] * b2[1] - b1[1] * b2[0])
    h = area / max(math.hypot(*b1), math.hypot(*b2))
    m = int(math.ceil(R / h)) + 2
    total = 0.0
    js = np.arange(-m, m + 1, dtype=float)
    for i in range(-m, m + 1):
        vx = i * b1[0] + js * b2[0]
        vy = i * b1[1] + js * b2[1]
        r2 = vx * vx + vy * vy
        mask = (r2 < R * R) & (r2 > 0)
        total += float(np.sum(u(d, np.sqrt(r2[mask]))))
    return 0.5 * total + 2.0 * math.pi * (1.0 / area) / (math.sqrt(R * R + d * d) + R)


def ewald(d: float, b1: tuple[float, float], b2: tuple[float, float], eta: float = 1.0) -> float:
    area = abs(b1[0] * b2[1] - b1[1] * b2[0])
    # real space, V_s / d^2
    rr = 7.0 / eta
    h = area / max(math.hypot(*b1), math.hypot(*b2))
    m = int(math.ceil(rr / h)) + 2
    real = 0.0
    for i in range(-m, m + 1):
        for j in range(-m, m + 1):
            if i == 0 and j == 0:
                continue
            x = i * b1[0] + j * b2[0]
            y = i * b1[1] + j * b2[1]
            r = math.hypot(x, y)
            if r < rr:
                s = math.hypot(r, d)
                real += 2.0 * (erfc(eta * r) / r - erfc(eta * s) / s)
    # reciprocal
    det = b1[0] * b2[1] - b1[1] * b2[0]
    tp = 2.0 * math.pi / det
    g1 = (tp * b2[1], -tp * b2[0])
    g2 = (-tp * b1[1], tp * b1[0])
    gcut = 14.0 * eta
    mg = int(math.ceil(gcut / min(math.hypot(*g1), math.hypot(*g2)) * 1.5)) + 2
    om = lambda t: -math.expm1(-d * d * t * t)
    recip = 4.0 * math.sqrt(math.pi) * quad(lambda t: om(t) / (t * t) if t > 0 else d * d, 0, eta,
                                            epsabs=0, epsrel=1e-13, limit=200)[0]
    for i in range(-mg, mg + 1):
        for j in range(-mg, mg + 1):
            if i == 0 and j == 0:
                continue
            gx = i * g1[0] + j * g2[0]
            gy = i * g1[1] + j * g2[1]
            g2n = gx * gx + gy * gy
            if math.sqrt(g2n) >= gcut:
                continue
            a = 0.25 * g2n
            k = quad(lambda t: math.exp(-a / (t * t)) * om(t) / (t * t) if t > 0 else 0.0, 0, eta,
                     epsabs=0, epsrel=1e-13, limit=200)[0]
            recip += 4.0 * math.sqrt(math.pi) * k
    recip /= area
    vl0 = 4.0 / math.sqrt(math.pi) * quad(om, 0, eta, epsabs=0, epsrel=1e-13)[0]
    return 0.5 * (real + recip - vl0) / (d * d)


def main(refs_path: str, out_path: str) -> None:
    tri = ((A_TRI, 0.0), (0.5 * A_TRI, 0.5 * math.sqrt(3.0) * A_TRI))
    sq = ((1.0, 0.0), (0.0, 1.0))
    out = []
    for line in open(refs_path):
        rec = json.loads(line)
        d = rec["d"]
        row = {"d": d, "rust_e_tri_exact": rec["e_tri_exact"], "rust_e_square_exact": rec["e_square_exact"]}
        row["py_ewald_tri"] = ewald(d, *tri)
        row["py_ewald_square"] = ewald(d, *sq)
        row["py_direct_tri"] = {str(R): direct(d, *tri, R) for R in (100.0, 200.0, 400.0)}
        row["py_direct_square"] = {str(R): direct(d, *sq, R) for R in (100.0, 200.0, 400.0)}
        row["rel_diff_ewald_tri"] = (row["py_ewald_tri"] - rec["e_tri_exact"]) / rec["e_tri_exact"]
        row["rel_diff_ewald_square"] = (row["py_ewald_square"] - rec["e_square_exact"]) / rec["e_square_exact"]
        row["max_abs_diff_direct_tri"] = max(abs(v - rec["e_tri_exact"]) for v in row["py_direct_tri"].values())
        row["max_abs_diff_direct_square"] = max(abs(v - rec["e_square_exact"])
                                                for v in row["py_direct_square"].values())
        # negative control: drop the tail -> the direct sum misses about 2 pi / (2R)
        row["neg_ctl_direct_tri_no_tail_R100"] = row["py_direct_tri"]["100.0"] - 2.0 * math.pi * (
            1.0 / (0.5 * math.sqrt(3.0) * A_TRI * A_TRI)) / (math.sqrt(1e4 + d * d) + 100.0)
        row["neg_ctl_abs_diff"] = abs(row["neg_ctl_direct_tri_no_tail_R100"] - rec["e_tri_exact"])
        print(json.dumps(row))
        out.append(row)
    json.dump(out, open(out_path, "w"), indent=1)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
