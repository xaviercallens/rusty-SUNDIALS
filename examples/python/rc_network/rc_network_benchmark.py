#!/usr/bin/env python3
"""RC-network benchmark for the rusty-SUNDIALS CVODE Python binding.

A resistor network with a capacitance C=1 from every node to ground and
imposed boundary voltages V_b obeys the stiff linear system

    dV_i/dt = -(L_ii V_i + L_ib V_b),

where L is the graph Laplacian and i/b are interior/boundary nodes. It has
an exact solution and a static cross-check, so it makes a sharp regression
test for a BDF integrator:

  K1 (known answer)  V_i(t) = expm(-L_ii t)(V_i(0) - V_inf) + V_inf must be
                     reproduced to 1e-5 relative (max norm) at t = 0.25, 1, 4,
                     16 tau, with tau = 1/lambda_min(L_ii).
  K2 (static check)  the integrated steady-state boundary currents under
                     V_b = e_j must equal column j of the Dirichlet-to-Neumann
                     map Lambda = L_bb - L_bi L_ii^-1 L_ib to 1e-6 relative.

Two fixture networks are included: a layer-2 truncation of the hyperbolic
{7,3} tiling (N=112) and a square-lattice disk of radius 6 (N=113). Their
stiffness ratios are ~15 and ~35; on larger networks the hyperbolic one stays
bounded while the flat one grows linearly in N (Callens 2026, "Logarithmic
boundary depth and the conditioning of the discrete inverse conductance
problem on hyperbolic lattices").

    python3 rc_network_benchmark.py            # prints a table, exit 1 on failure
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

import numpy as np
from scipy.linalg import expm

from rusty_sundials import CvodeSolver

HERE = Path(__file__).resolve().parent
FIXTURES = HERE / "fixtures"
RTOL, ATOL = 1e-8, 1e-10
K1_TOL, K2_TOL = 1e-5, 1e-6


def load(name):
    g = json.loads((FIXTURES / name).read_text())
    n = len(g["x"])
    L = np.zeros((n, n))
    for a, b in g["edges"]:
        L[a, a] += 1; L[b, b] += 1; L[a, b] -= 1; L[b, a] -= 1
    bnd = np.array(g["boundary"])
    interior = np.setdiff1d(np.arange(n), bnd)
    return L, bnd, interior


def integrate(Lii, Lib, Vb, V0, t_out):
    forcing = -(Lib @ Vb)

    def rhs(t, y):
        return (-(Lii @ np.asarray(y)) + forcing).tolist()

    solver = CvodeSolver(method="bdf", rtol=RTOL, atol=ATOL, max_steps=200000)
    out, y, t = [], list(V0), 0.0
    for tk in t_out:
        if tk > t:
            t, y = solver.solve(rhs, t, y, float(tk))
        out.append(np.array(y))
    return np.array(out)


def run_case(name):
    L, bnd, interior = load(name)
    Lii, Lib = L[np.ix_(interior, interior)], L[np.ix_(interior, bnd)]
    ev = np.linalg.eigvalsh(Lii)
    tau = 1.0 / ev[0]
    rng = np.random.default_rng(3)
    Vb = rng.uniform(-1, 1, len(bnd))
    V0 = np.zeros(len(interior))
    t_out = np.array([0.25, 1.0, 4.0, 16.0]) * tau
    Vinf = np.linalg.solve(Lii, -(Lib @ Vb))
    exact = np.array([expm(-Lii * t) @ (V0 - Vinf) + Vinf for t in t_out])
    t0 = time.time()
    got = integrate(Lii, Lib, Vb, V0, t_out)
    k1 = float(np.max(np.abs(got - exact)) / np.max(np.abs(exact)))
    lam = L[np.ix_(bnd, bnd)] - L[np.ix_(bnd, interior)] @ np.linalg.solve(Lii, Lib)
    k2 = 0.0
    for j in (0, len(bnd) // 3, 2 * len(bnd) // 3):
        e = np.zeros(len(bnd)); e[j] = 1.0
        Vi = integrate(Lii, Lib, e, np.zeros(len(interior)), np.array([40.0 * tau]))[-1]
        cur = L[np.ix_(bnd, bnd)] @ e + L[np.ix_(bnd, interior)] @ Vi
        k2 = max(k2, float(np.max(np.abs(cur - lam[:, j])) / np.max(np.abs(lam[:, j]))))
    return {"network": name.removesuffix(".json"), "N": L.shape[0], "stiffness": float(ev[-1] / ev[0]),
            "tau": float(tau), "K1_rel_err": k1, "K1_pass": k1 < K1_TOL, "K2_rel_err": k2,
            "K2_pass": k2 < K2_TOL, "seconds": round(time.time() - t0, 2)}


CASES = ("hyperbolic_7_3_L2.json", "square_R6.json")


def main() -> int:
    rows = [run_case(c) for c in CASES]
    for r in rows:
        print(f"{r['network']:20} N={r['N']:4d} stiffness={r['stiffness']:6.1f} "
              f"K1={r['K1_rel_err']:.1e} ({'pass' if r['K1_pass'] else 'FAIL'}) "
              f"K2={r['K2_rel_err']:.1e} ({'pass' if r['K2_pass'] else 'FAIL'}) {r['seconds']}s")
    return 0 if all(r["K1_pass"] and r["K2_pass"] for r in rows) else 1


if __name__ == "__main__":
    sys.exit(main())
