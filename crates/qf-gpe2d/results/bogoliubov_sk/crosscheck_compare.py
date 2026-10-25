"""Two-language cross-check: Rust lr_series (Strang, dt_safety 0.25) vs numpy_crosscheck.py.

Usage: crosscheck_compare.py LR_SERIES_BINARY OUT_JSON
"""
from __future__ import annotations

import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from numpy_crosscheck import run  # noqa: E402


def rust(binary: str, x: float) -> dict[str, float]:
    out = subprocess.run([binary, str(x), "1", "200", "phase", "0.25", "strang"],
                         capture_output=True, text=True, check=True).stdout
    head = out.splitlines()[0].lstrip("# ").split()
    vals = dict(kv.split("=") for kv in head)
    return {k: float(v) for k, v in vals.items()}


def main(binary: str, out: str) -> None:
    rows = []
    for x in (0.05, 1.0, 4.0):
        r = rust(binary, x)
        p = run(x, 1.0, 0.25, 1e-3)
        rows.append({"x": x, "rust_omega": r["omega_zc"], "numpy_omega": float(p["omega"]),
                     "rel_diff_omega": abs(r["omega_zc"] - p["omega"]) / p["omega"],
                     "rust_amp": r["amp"], "numpy_amp": p["amp"],
                     "rel_diff_amp": abs(r["amp"] - p["amp"]) / p["amp"],
                     "omega_B": float(p["omega_B"])})
        print(rows[-1])
    json.dump(rows, open(out, "w"), indent=1)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
