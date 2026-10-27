"""Quadrature / stencil robustness of the E_zp curvature c(a) at rho near its sign changes (P2 gate G2, Tier X).

usage: python -I phonon_smalla_check.py OUT.json
c(a) = Lap E_zp(rho)/4 with Duffy-Gauss rules of 3 x 48 and 3 x 80 radial nodes and stencil steps h = 2e-3, 1e-3, plus
the exact long-wave (elastic) eigenvalue of the triangular lattice, for a at and around the three sign changes.
"""

import os

for _v in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[_v] = "2"

import json  # noqa: E402
import sys  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import phonon_mechanism as pm  # noqa: E402
from phonon_elastic_check import elastic_min  # noqa: E402


def main() -> None:
    out = sys.argv[1]
    rows = []
    for a in (0.1, 0.11, 0.1227, 0.135, 0.15, 0.36, 0.3767, 0.39, 1.55, 1.5815, 1.61):
        row = {"a": a}
        for N in (48, 80):
            pm.NODES = pm.duffy_shell_nodes(N)
            st = pm.lap_stencil(lambda x, y: pm.ezp_at(a, x, y), *pm.RHO)
            row[f"N{N}"] = {"c": st["lap"] / 4, "c_h2e-3": st["lap_h2e-3"] / 4, "c_h1e-3": st["lap_h1e-3"] / 4, "ezp_rho": pm.ezp_at(a, *pm.RHO)}
        row["tri_elastic_min_lam"] = elastic_min(f"gauss:{a}", 0.5, pm.S3 / 2, 60.0)["min_lam"]
        rows.append(row)
        print(json.dumps(row), flush=True)
        json.dump(rows, open(out, "w"), indent=1)


if __name__ == "__main__":
    main()
