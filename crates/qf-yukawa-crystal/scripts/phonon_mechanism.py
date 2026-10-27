"""Mechanism of the QU-1 counterexample for Gaussian potentials (P2 gate G2, Tier X numerics, exact sum rules).

usage: python -I phonon_mechanism.py OUT.json

Uses the reciprocal-space dynamical matrix and the Duffy-Gauss Brillouin-zone rule of phonon_poisson_check.py.

1. Curvature at the triangular point rho. For any function F(tau) invariant under PSL2(Z) and smooth at rho, the
   gradient vanishes and the Hessian is isotropic (stabiliser of order 3 acting by a 120 degree rotation of the tangent
   plane), so F(tau) - F(rho) = (Lap F / 4) |tau - rho|^2 / y^2 + O(|tau-rho|^3) with Lap = y^2 (d_xx + d_yy) the hyperbolic
   Laplacian. c = Lap F / 4 is the quantity tabulated by G1. Lap is computed by central differences (h = 2e-3, 1e-3,
   Richardson) on E_zp (Duffy N=96, converged to ~1e-15).
2. Positive control of the stencil: for a pair sum F = sum_{R != 0} f(|R|^2) the exact identity (Q = |m + n tau|^2/y,
   Lap Q = 2 Q, |grad Q|^2_hyp = Q^2) gives Lap F = sum_{R != 0} h(|R|^2), h(u) = d/du (u^2 f'(u)). Checked for the static
   energy f = e^{-pi a u}/2 and for the second moment (sum rule) f = Lap V = (-4 pi a + 4 pi^2 a^2 u) e^{-pi a u}.
3. Decomposition Lap E_zp = Lin + Conc with Phi(D) = tr D^{1/2}: Lin = (1/2) < y^2 sum_i tr(Phi'(D) d_i^2 D) >,
   Conc = (1/2) < y^2 sum_i Phi''(D)[d_i D, d_i D] > <= 0 (operator concavity of the square root; Daleckii-Krein form),
   split into the diagonal (per-branch) and the cross (branch-mixing) part and into radial Duffy shells s in
   [0,1/3], [1/3,2/3], [2/3,1] (s = 1 is the zone edge in reduced coordinates).
4. Einstein bound E_zp <= E_E = sqrt(<tr D>/2) (Jensen), whose curvature has the sign of Lap <tr D> (exact sum).
5. Scan of c(a) over a and bisection of its sign changes.
6. Branch / shell split of the finite difference E_zp(competitor) - E_zp(rho) for the gauss:1 headline competitor.
"""

import os

for _v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "2"

import json  # noqa: E402
import math  # noqa: E402
import sys  # noqa: E402
import time  # noqa: E402

import numpy as np  # noqa: E402
import sympy as sp  # noqa: E402
from scipy.optimize import brentq  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from phonon_poisson_check import PoissonGauss, bases, invariants, lattice_points, omega_sum, stability  # noqa: E402

PI = math.pi
S3 = math.sqrt(3.0)
RHO = (0.5, S3 / 2)


def duffy_shell_nodes(N: int, edges: tuple = (0.0, 1 / 3, 2 / 3, 1.0)) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    xg, wg = np.polynomial.legendre.leggauss(N)
    us, vs, ww, tag = [], [], [], []
    tg = 0.5 * (xg + 1)
    wt = 0.5 * wg
    for b in range(len(edges) - 1):
        lo, hi = edges[b], edges[b + 1]
        s = lo + (hi - lo) * 0.5 * (xg + 1)
        ws = (hi - lo) * 0.5 * wg
        S, Tt = np.meshgrid(s, tg, indexing="ij")
        W = np.outer(ws, wt)
        S, Tt, W = S.ravel(), Tt.ravel(), W.ravel()
        for sg in (1.0, -1.0):
            for horiz in (True, False):
                if horiz:
                    us.append(sg * S / 2)
                    vs.append(S * (Tt - 0.5))
                else:
                    us.append(S * (Tt - 0.5))
                    vs.append(sg * S / 2)
                ww.append(W * S / 2)
                tag.append(np.full(S.shape, b))
    return np.concatenate(us), np.concatenate(vs), np.concatenate(ww), np.concatenate(tag)


NODES = duffy_shell_nodes(48)


def ezp_at(a: float, x: float, y: float) -> float:
    u, v, w, _ = NODES
    pg = PoissonGauss(a, x, y)
    return 0.5 * float((w * omega_sum(pg.D(pg.kappa(u, v)))).sum())


def lap_stencil(F, x0: float, y0: float) -> dict:
    out = {}
    for h in (2e-3, 1e-3):
        f0 = F(x0, y0)
        fxx = (F(x0 + h, y0) - 2 * f0 + F(x0 - h, y0)) / h**2
        fyy = (F(x0, y0 + h) - 2 * f0 + F(x0, y0 - h)) / h**2
        fxy = (F(x0 + h, y0 + h) - F(x0 + h, y0 - h) - F(x0 - h, y0 + h) + F(x0 - h, y0 - h)) / (4 * h * h)
        fx = (F(x0 + h, y0) - F(x0 - h, y0)) / (2 * h)
        fy = (F(x0, y0 + h) - F(x0, y0 - h)) / (2 * h)
        out[h] = (y0 * y0 * (fxx + fyy), fxx, fyy, fxy, fx, fy)
    lap = (4 * out[1e-3][0] - out[2e-3][0]) / 3
    o = out[1e-3]
    return {"lap": lap, "lap_h2e-3": out[2e-3][0], "lap_h1e-3": o[0], "fxx": o[1], "fyy": o[2], "fxy": o[3], "grad": [o[4], o[5]]}


U = sp.symbols("u", positive=True)
A = sp.symbols("a", positive=True)


def h_of(fexpr: sp.Expr):
    h = sp.diff(U**2 * sp.diff(fexpr, U), U)
    return sp.lambdify((U, A), sp.simplify(h), "numpy"), sp.lambdify((U, A), fexpr, "numpy"), sp.simplify(h)


F_STATIC = sp.exp(-sp.pi * A * U) / 2
F_M2 = (-4 * sp.pi * A + 4 * sp.pi**2 * A**2 * U) * sp.exp(-sp.pi * A * U)


def pair_sum(fn, a: float, x: float, y: float) -> float:
    Bm, _ = bases(x, y)
    R = lattice_points(Bm, math.sqrt(45.0 / (PI * a)) + 1.0)
    u = (R**2).sum(1)
    u = u[u > 0]
    return float(fn(u, a).sum())


def decomposition(a: float, h: float = 1e-3) -> dict:
    """Lin + Conc split of Lap E_zp at rho, per Duffy shell, with d_i D by central differences at fixed reduced k."""
    u, v, w, tag = NODES
    x0, y0 = RHO

    def Dat(x: float, y: float) -> np.ndarray:
        pg = PoissonGauss(a, x, y)
        return pg.D(pg.kappa(u, v))

    D0 = Dat(x0, y0)
    M = np.stack([np.stack([D0[:, 0], D0[:, 1]], 1), np.stack([D0[:, 1], D0[:, 2]], 1)], 1)
    lam, Ev = np.linalg.eigh(M)
    lam = np.maximum(lam, 1e-300)
    sq = np.sqrt(lam)
    lin = np.zeros(len(u))
    cdiag = np.zeros(len(u))
    ccross = np.zeros(len(u))
    for dx, dy in ((h, 0.0), (0.0, h)):
        Dp = Dat(x0 + dx, y0 + dy)
        Dm = Dat(x0 - dx, y0 - dy)
        d1 = (Dp - Dm) / (2 * h)
        d2 = (Dp - 2 * D0 + Dm) / (h * h)
        for X, kind in ((d2, "lin"), (d1, "conc")):
            Xm = np.stack([np.stack([X[:, 0], X[:, 1]], 1), np.stack([X[:, 1], X[:, 2]], 1)], 1)
            Xe = np.einsum("nji,njk,nkl->nil", Ev, Xm, Ev)  # eigenbasis
            if kind == "lin":
                lin += y0 * y0 * (Xe[:, 0, 0] / (2 * sq[:, 0]) + Xe[:, 1, 1] / (2 * sq[:, 1]))
            else:
                cdiag += y0 * y0 * (-(Xe[:, 0, 0] ** 2) / (4 * lam[:, 0] * sq[:, 0]) - Xe[:, 1, 1] ** 2 / (4 * lam[:, 1] * sq[:, 1]))
                ccross += y0 * y0 * 2 * (-(Xe[:, 0, 1] ** 2) / (2 * sq[:, 0] * sq[:, 1] * (sq[:, 0] + sq[:, 1])))
    out = {}
    for name, arr in (("lin", lin), ("conc_diag", cdiag), ("conc_cross", ccross)):
        out[name] = 0.5 * float((w * arr).sum())
        out[name + "_by_shell"] = [0.5 * float((w * arr)[tag == b].sum()) for b in range(3)]
    out["conc"] = out["conc_diag"] + out["conc_cross"]
    out["lin_plus_conc"] = out["lin"] + out["conc"]
    # per-branch split of Lin and diagonal Conc (branch 0 = lower)
    return out


def curvature(a: float) -> float:
    return lap_stencil(lambda x, y: ezp_at(a, x, y), *RHO)["lap"] / 4.0


def branch_shell_split(a: float, comp: tuple[float, float]) -> dict:
    u, v, w, tag = NODES
    res = {}
    vals = {}
    for name, (x, y) in (("tri", RHO), ("comp", comp)):
        pg = PoissonGauss(a, x, y)
        D = pg.D(pg.kappa(u, v))
        T, P, lo = invariants(D)
        hi = T - lo
        vals[name] = (np.sqrt(np.maximum(lo, 0)), np.sqrt(np.maximum(hi, 0)), T)
    for bi, bname in ((0, "lower_branch"), (1, "upper_branch")):
        d = vals["comp"][bi] - vals["tri"][bi]
        res[bname] = 0.5 * float((w * d).sum())
        res[bname + "_by_shell"] = [0.5 * float((w * d)[tag == b].sum()) for b in range(3)]
    res["total"] = res["lower_branch"] + res["upper_branch"]
    dT = vals["comp"][2] - vals["tri"][2]
    res["d_trD_avg"] = float((w * dT).sum())
    res["einstein_tri"] = math.sqrt(float((w * vals["tri"][2]).sum()) / 2)
    res["einstein_comp"] = math.sqrt(float((w * vals["comp"][2]).sum()) / 2)
    res["einstein_diff"] = res["einstein_comp"] - res["einstein_tri"]
    return res


def main() -> None:
    out = sys.argv[1]
    t0 = time.time()
    hs, fs, hs_expr = h_of(F_STATIC)
    hm, fm, hm_expr = h_of(F_M2)
    result: dict = {"h_static": str(hs_expr), "h_m2": str(hm_expr)}
    # control: Duffy N=48 x 3 shells vs N=128 single rule at rho (gauss:1)
    pg = PoissonGauss(1.0, *RHO)
    result["ctl_nodes_ezp_gauss1_rho"] = ezp_at(1.0, *RHO)
    # 2. stencil positive controls, and 1. curvature + 3. decomposition + 4. Einstein, at the G1 potentials
    rows = []
    for a in (0.25, 0.5, 1.0, 2.0, 4.0):
        r: dict = {"a": a}
        st = lap_stencil(lambda x, y: pair_sum(fs, a, x, y), *RHO)
        r["static_lap_stencil"] = st["lap"]
        r["static_lap_exact"] = pair_sum(hs, a, *RHO)
        sm = lap_stencil(lambda x, y: pair_sum(fm, a, x, y), *RHO)
        r["m2"] = pair_sum(fm, a, *RHO)
        r["m2_lap_stencil"] = sm["lap"]
        r["m2_lap_exact"] = pair_sum(hm, a, *RHO)
        ez = lap_stencil(lambda x, y: ezp_at(a, x, y), *RHO)
        r["ezp_rho"] = ezp_at(a, *RHO)
        r["ezp_lap"] = ez["lap"]
        r["ezp_lap_h2e-3"] = ez["lap_h2e-3"]
        r["ezp_fxx_fyy_fxy"] = [ez["fxx"], ez["fyy"], ez["fxy"]]
        r["ezp_grad"] = ez["grad"]
        r["c_ezp"] = ez["lap"] / 4
        M2 = r["m2"]
        r["einstein_E"] = math.sqrt(M2 / 2)
        r["einstein_lap"] = r["m2_lap_exact"] / (4 * math.sqrt(M2 / 2))
        r["decomposition"] = decomposition(a)
        r["tri_stability"] = stability(PoissonGauss(a, *RHO))
        rows.append(r)
        print(json.dumps(r), flush=True)
        result["at_G1_potentials"] = rows
        json.dump(result, open(out, "w"), indent=1)
    # 6. branch/shell split for the headline competitors
    result["split_gauss1_c1"] = branch_shell_split(1.0, (0.5, 0.966025403784))
    result["split_gauss1_c2"] = branch_shell_split(1.0, (0.295, 0.9555))
    result["split_gauss2_c1"] = branch_shell_split(2.0, (0.5, 0.966025403784))
    print(json.dumps({k: result[k] for k in ("split_gauss1_c1", "split_gauss1_c2", "split_gauss2_c1")}), flush=True)
    json.dump(result, open(out, "w"), indent=1)
    # 5. scan c(a) and sign changes
    grid = [0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.6, 0.7, 0.8, 1.0, 1.2, 1.4, 1.6, 1.8, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0]
    scan = []
    for a in grid:
        c = curvature(a)
        m2lap = pair_sum(hm, a, *RHO)
        stab = stability(PoissonGauss(a, *RHO))
        scan.append({"a": a, "c_ezp": c, "lap_m2_exact": m2lap, "tri_min_ratio": stab["min_ratio_lam_over_k2"], "tri_stable": stab["stable"]})
        print(json.dumps(scan[-1]), flush=True)
        result["scan"] = scan
        json.dump(result, open(out, "w"), indent=1)
    roots = []
    for i in range(len(scan) - 1):
        if scan[i]["c_ezp"] * scan[i + 1]["c_ezp"] < 0:
            rt = brentq(curvature, scan[i]["a"], scan[i + 1]["a"], xtol=1e-6)
            roots.append({"bracket": [scan[i]["a"], scan[i + 1]["a"]], "root": rt})
            print(json.dumps(roots[-1]), flush=True)
    m2roots = []
    for i in range(len(scan) - 1):
        if scan[i]["lap_m2_exact"] * scan[i + 1]["lap_m2_exact"] < 0:
            rt = brentq(lambda aa: pair_sum(hm, aa, *RHO), scan[i]["a"], scan[i + 1]["a"], xtol=1e-10)
            m2roots.append({"bracket": [scan[i]["a"], scan[i + 1]["a"]], "root": rt})
    result["c_sign_changes"] = roots
    result["lap_m2_sign_changes"] = m2roots
    result["seconds"] = round(time.time() - t0, 1)
    json.dump(result, open(out, "w"), indent=1)
    print(json.dumps({"roots": roots, "m2roots": m2roots, "seconds": result["seconds"]}), flush=True)


if __name__ == "__main__":
    main()
