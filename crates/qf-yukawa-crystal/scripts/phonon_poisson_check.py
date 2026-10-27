"""Independent reciprocal-space (Poisson) check of the QU-1 counterexample for Gaussian potentials (P2 gate G2, Tier X).

usage: python -I phonon_poisson_check.py OUT.json

Nothing here shares code with phonon.rs or phonon_independent_check.py. For V(r) = exp(-pi a r^2) on a unit-covolume
lattice L (basis a1 = (1/sqrt y, 0), a2 = (x/sqrt y, sqrt y)) Poisson summation gives, with kappa = k/(2 pi) and L* the
dual lattice (xi . R integer),

    D(k) = sum_{R != 0} (1 - cos k.R) Hess V(R)
         = (4 pi^2 / a) sum_{xi in L*} [ (xi+kappa)(xi+kappa)^T e^{-pi |xi+kappa|^2/a} - xi xi^T e^{-pi |xi|^2/a} ]

(the R = 0 term may be included since 1 - cos 0 = 0). The zero-point energy per particle is
E_zp = (1/2) <sqrt(lam1) + sqrt(lam2)>_BZ = (1/2) <sqrt(tr D + 2 sqrt(det D))>_BZ (smooth across branch crossings).

Two Brillouin-zone quadratures, both different from the Rust code:
 * uniform grids n = 256, 512, 1024 (Gamma-centred and half-shifted) with n^-3 Richardson extrapolation;
 * a Duffy-transformed Gauss-Legendre rule: the reduced cell [-1/2, 1/2]^2 is split into four triangles with apex at
   Gamma, so the conical singularity sqrt(D) ~ |k| is removed by the Jacobian and the rule converges spectrally.
Stability: lam_min(D(k)) / |k|^2 is minimised over the zone (dense seeds + Nelder-Mead), so soft long-wave and
zone-boundary modes are both covered.
Controls: (positive) Poisson D vs an explicit real-space sum at random k; exact sum rule <tr D>_BZ = 4 pi a -
(4 pi^2/a) sum_{xi in L*} |xi|^2 e^{-pi|xi|^2/a} = sum_{R != 0} Lap V(R); (negative) planted bugs (dropped -xi xi^T term,
a <-> 1/a in the transform) must break the real-space match; the square lattice must be flagged unstable; gauss:2,
which survived G1, must not produce a stable lower-E_zp lattice at the gauss:1 competitor shapes.
"""

import os

for _v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "2"

import json  # noqa: E402
import math  # noqa: E402
import sys  # noqa: E402
import time  # noqa: E402

import numpy as np  # noqa: E402
from scipy.optimize import minimize  # noqa: E402

PI = math.pi
S3 = math.sqrt(3.0)
TOL_EXP = 41.0  # e^{-41} ~ 1.6e-18
KMIN_OPT = 3e-3  # smallest |k| the stability optimiser may visit


def bases(x: float, y: float) -> tuple[np.ndarray, np.ndarray]:
    sy = math.sqrt(y)
    A = np.array([[1.0 / sy, 0.0], [x / sy, sy]])  # rows a1, a2
    Xi = np.linalg.inv(A).T  # rows xi1, xi2 with xi_i . a_j = delta_ij
    return A, Xi


def lattice_points(B: np.ndarray, rmax: float) -> np.ndarray:
    """All m b1 + l b2 with |.| < rmax (rows of B are the basis)."""
    det = abs(np.linalg.det(B))
    mm = int(rmax * np.linalg.norm(B[1]) / det) + 2
    ll = int(rmax * np.linalg.norm(B[0]) / det) + 2
    m, l = np.meshgrid(np.arange(-mm, mm + 1), np.arange(-ll, ll + 1), indexing="ij")
    P = m.reshape(-1, 1) * B[0] + l.reshape(-1, 1) * B[1]
    r = np.hypot(P[:, 0], P[:, 1])
    return P[r < rmax]


class PoissonGauss:
    def __init__(self, a: float, x: float, y: float, bug: str = "") -> None:
        self.a, self.x, self.y = a, x, y
        _, self.Xi = bases(x, y)
        self.kmax = 0.5 * (np.linalg.norm(self.Xi[0]) + np.linalg.norm(self.Xi[1]))
        # transform width: correct is exp(-pi |xi|^2 / a); planted bug 'swap' uses exp(-pi a |xi|^2)
        self.w = a if bug != "swap" else 1.0 / a
        self.G = lattice_points(self.Xi, math.sqrt(TOL_EXP * self.w / PI) + self.kmax + 1e-9)
        g2 = (self.G**2).sum(1)
        e = np.exp(-PI * g2 / self.w)
        self.C0 = np.array([(e * self.G[:, 0] ** 2).sum(), (e * self.G[:, 0] * self.G[:, 1]).sum(), (e * self.G[:, 1] ** 2).sum()])
        if bug == "drop_c0":
            self.C0 = np.zeros(3)
        self.pref = 4 * PI * PI / self.w

    def D(self, K: np.ndarray) -> np.ndarray:
        """K: (N,2) kappa vectors (k/2pi) with reduced coordinates in [-1/2,1/2]. Returns (N,3) [Dxx, Dxy, Dyy]."""
        out = np.empty((len(K), 3))
        for s in range(0, len(K), 16384):
            k = K[s : s + 16384]
            Zx = k[:, 0:1] + self.G[None, :, 0]
            Zy = k[:, 1:2] + self.G[None, :, 1]
            e = np.exp(-PI * (Zx * Zx + Zy * Zy) / self.w)
            out[s : s + len(k), 0] = (e * Zx * Zx).sum(1)
            out[s : s + len(k), 1] = (e * Zx * Zy).sum(1)
            out[s : s + len(k), 2] = (e * Zy * Zy).sum(1)
        return self.pref * (out - self.C0[None, :])

    def kappa(self, u: np.ndarray, v: np.ndarray) -> np.ndarray:
        return u.reshape(-1, 1) * self.Xi[0] + v.reshape(-1, 1) * self.Xi[1]

    def sum_rule_exact(self) -> float:
        g2 = (self.G**2).sum(1)
        return 4 * PI * self.a - (4 * PI * PI / self.a) * (g2 * np.exp(-PI * g2 / self.a)).sum()


def realspace_D(a: float, x: float, y: float, K: np.ndarray) -> np.ndarray:
    """Explicit sum over R != 0 of (1 - cos 2 pi kappa.R) Hess V(R), Hess V = (-2 pi a I + 4 pi^2 a^2 R R^T) e^{-pi a R^2}."""
    A, _ = bases(x, y)
    R = lattice_points(A, math.sqrt(TOL_EXP / (PI * a)) + 1.0)
    r2 = (R**2).sum(1)
    R = R[r2 > 0]
    r2 = r2[r2 > 0]
    e = np.exp(-PI * a * r2)
    H = np.stack([(-2 * PI * a + 4 * PI * PI * a * a * R[:, 0] ** 2) * e, 4 * PI * PI * a * a * R[:, 0] * R[:, 1] * e, (-2 * PI * a + 4 * PI * PI * a * a * R[:, 1] ** 2) * e], 1)
    c = 1.0 - np.cos(2 * PI * K @ R.T)
    return c @ H


def realspace_laplacian_sum(a: float, x: float, y: float) -> float:
    A, _ = bases(x, y)
    R = lattice_points(A, math.sqrt(TOL_EXP / (PI * a)) + 1.0)
    r2 = (R**2).sum(1)
    r2 = r2[r2 > 0]
    return float(((-4 * PI * a + 4 * PI * PI * a * a * r2) * np.exp(-PI * a * r2)).sum())


def invariants(D: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    T = D[:, 0] + D[:, 2]
    P = D[:, 0] * D[:, 2] - D[:, 1] ** 2
    lam_lo = 0.5 * T - np.sqrt(0.25 * (D[:, 0] - D[:, 2]) ** 2 + D[:, 1] ** 2)
    return T, P, lam_lo


def omega_sum(D: np.ndarray) -> np.ndarray:
    T, P, _ = invariants(D)
    return np.sqrt(np.maximum(T + 2 * np.sqrt(np.maximum(P, 0.0)), 0.0))


def ezp_uniform(pg: PoissonGauss, n: int, shift: float) -> tuple[float, float, float]:
    idx = ((np.arange(n) + shift) / n + 0.5) % 1.0 - 0.5
    uu, vv = np.meshgrid(idx, idx, indexing="ij")
    K = pg.kappa(uu, vv)
    D = pg.D(K)
    T, _, lo = invariants(D)
    k2 = (2 * PI) ** 2 * (K**2).sum(1)
    nz = k2 > 1e-20
    return 0.5 * float(omega_sum(D).mean()), float(T.mean()), float((lo[nz] / k2[nz]).min())


def duffy_nodes(N: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    xg, wg = np.polynomial.legendre.leggauss(N)
    s = 0.5 * (xg + 1)
    ws = 0.5 * wg
    S, Tt = np.meshgrid(s, s, indexing="ij")
    W = np.outer(ws, ws)
    S, Tt, W = S.ravel(), Tt.ravel(), W.ravel()
    us, vs, ww = [], [], []
    for sg in (1.0, -1.0):
        us.append(sg * S / 2)
        vs.append(S * (Tt - 0.5))
        ww.append(W * S / 2)
        us.append(S * (Tt - 0.5))
        vs.append(sg * S / 2)
        ww.append(W * S / 2)
    return np.concatenate(us), np.concatenate(vs), np.concatenate(ww)


def ezp_duffy(pg: PoissonGauss, N: int) -> tuple[float, float]:
    u, v, w = duffy_nodes(N)
    D = pg.D(pg.kappa(u, v))
    T, _, _ = invariants(D)
    return 0.5 * float((w * omega_sum(D)).sum()), float((w * T).sum())


def stability(pg: PoissonGauss) -> dict:
    """min over the zone of lam_min(D(k))/|k|^2 (k = 2 pi kappa): seeds on a shifted n=192 grid, Nelder-Mead refine."""
    n = 192
    idx = ((np.arange(n) + 0.5) / n + 0.5) % 1.0 - 0.5
    uu, vv = np.meshgrid(idx, idx, indexing="ij")
    u, v = uu.ravel(), vv.ravel()
    K = pg.kappa(u, v)
    _, _, lo = invariants(pg.D(K))
    k2 = (2 * PI) ** 2 * (K**2).sum(1)
    ratio = lo / k2
    order = np.argsort(ratio)[:12]

    def f(p: np.ndarray) -> float:
        uu_ = (p[0] + 0.5) % 1.0 - 0.5
        vv_ = (p[1] + 0.5) % 1.0 - 0.5
        Kp = pg.kappa(np.array([uu_]), np.array([vv_]))
        kk = (2 * PI) ** 2 * float((Kp**2).sum())
        if kk < KMIN_OPT**2:
            # below |k| = KMIN_OPT the Poisson D is a difference of O(1) sums and lam/|k|^2 is round-off dominated
            # (|k| ~ 1e-6 gives spurious negative ratios ~ 1e-5); that region is covered by the long-wave rows below
            # and by the exact elastic tensor of phonon_elastic_check.py
            return 1e3
        _, _, l = invariants(pg.D(Kp))
        return float(l[0]) / kk

    best = (float(ratio[order[0]]), float(u[order[0]]), float(v[order[0]]))
    for o in order:
        r = minimize(f, np.array([u[o], v[o]]), method="Nelder-Mead", options={"xatol": 1e-7, "fatol": 1e-12, "maxiter": 2000})
        if r.fun < best[0]:
            best = (float(r.fun), float((r.x[0] + 0.5) % 1 - 0.5), float((r.x[1] + 0.5) % 1 - 0.5))
    # long-wave limit, |k| = 1e-2 and 3e-3, 720 directions
    lw = {}
    for keps in (1e-2, 3e-3):
        th = np.linspace(0, 2 * PI, 720, endpoint=False)
        K = (keps / (2 * PI)) * np.stack([np.cos(th), np.sin(th)], 1)
        _, _, l = invariants(pg.D(K))
        lw[f"{keps:g}"] = float(l.min() / keps**2)
    Kb = pg.kappa(np.array([best[1]]), np.array([best[2]]))
    return {"min_ratio_lam_over_k2": best[0], "at_reduced_uv": [best[1], best[2]], "at_abs_k": float(2 * PI * np.linalg.norm(Kb)), "longwave_min_ratio": lw, "stable": bool(best[0] > 0 and min(lw.values()) > 0)}


def analyse(a: float, x: float, y: float, full: bool) -> dict:
    t0 = time.time()
    pg = PoissonGauss(a, x, y)
    rng = np.random.default_rng(12345)
    uv = rng.uniform(-0.5, 0.5, size=(200, 2))
    K = pg.kappa(uv[:, 0], uv[:, 1])
    Dp = pg.D(K)
    Dr = realspace_D(a, x, y, K)
    scale = float(np.abs(Dr).max())
    row: dict = {"pot": f"gauss:{a:g}", "x": x, "y": y, "nG": int(len(pg.G))}
    row["ctl_poisson_vs_realspace_maxrel"] = float(np.abs(Dp - Dr).max() / scale)
    for bug in ("drop_c0", "swap"):
        Db = PoissonGauss(a, x, y, bug=bug).D(K)
        row[f"ctl_planted_bug_{bug}_maxrel"] = float(np.abs(Db - Dr).max() / scale)
    row["sum_rule_exact_poisson"] = pg.sum_rule_exact()
    row["sum_rule_exact_realspace"] = realspace_laplacian_sum(a, x, y)
    st = stability(pg)
    row["stability"] = st
    duf = {}
    for N in (16, 32, 64, 96, 128):
        e, t = ezp_duffy(pg, N)
        duf[str(N)] = {"ezp": e, "trD_avg": t}
    row["duffy"] = duf
    if full:
        uni = {}
        for n in (256, 512, 1024):
            for sh in (0.0, 0.5):
                e, t, lr = ezp_uniform(pg, n, sh)
                uni[f"n{n}_s{sh}"] = {"ezp": e, "trD_avg": t, "min_ratio_on_grid": lr}
        for sh in (0.0, 0.5):
            e1, e2 = uni[f"n512_s{sh}"]["ezp"], uni[f"n1024_s{sh}"]["ezp"]
            uni[f"richardson_n512_1024_s{sh}"] = (1024**3 * e2 - 512**3 * e1) / (1024**3 - 512**3)
        row["uniform"] = uni
    row["seconds"] = round(time.time() - t0, 1)
    return row


def main() -> None:
    out = sys.argv[1]
    tri = (0.5, S3 / 2)
    c1 = (0.5, 0.966025403784)
    c2 = (0.295, 0.9555)
    c3 = (0.3, 0.953939201417)
    cases = [
        (1.0, tri, True),
        (1.0, c1, True),
        (1.0, c2, True),
        (1.0, c3, False),
        (1.0, (0.0, 1.0), False),  # square lattice: must be flagged unstable
        (0.5, tri, True),
        (0.5, c1, True),
        (0.5, (0.5, 1.016), False),
        (2.0, tri, False),  # gauss:2 survived G1: negative control for the comparison
        (2.0, c1, False),
        (2.0, c2, False),
        (2.0, c3, False),
    ]
    res = []
    for a, (x, y), full in cases:
        row = analyse(a, x, y, full)
        res.append(row)
        print(json.dumps(row), flush=True)
        json.dump(res, open(out, "w"), indent=1)
    # comparisons against the triangular lattice, Duffy N=128
    ref = {r["pot"]: r["duffy"]["128"]["ezp"] for r in res if abs(r["x"] - 0.5) < 1e-12 and abs(r["y"] - S3 / 2) < 1e-12}
    summary = []
    for r in res:
        e = r["duffy"]["128"]["ezp"]
        summary.append({"pot": r["pot"], "x": r["x"], "y": r["y"], "stable": r["stability"]["stable"], "min_ratio": r["stability"]["min_ratio_lam_over_k2"], "ezp_duffy128": e, "dEzp_vs_tri": e - ref[r["pot"]], "duffy_conv_96_128": abs(r["duffy"]["128"]["ezp"] - r["duffy"]["96"]["ezp"])})
    json.dump({"cases": res, "summary": summary}, open(out, "w"), indent=1)
    for s in summary:
        print(json.dumps(s), flush=True)


if __name__ == "__main__":
    main()
