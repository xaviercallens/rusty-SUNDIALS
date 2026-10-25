"""Extract the MoS2, MoSe2, WS2, WSe2 rows (all prototypes) from colabfit/JARVIS_C2DB and write them as JSON.

Usage: hf_tmd_rows.py DATA_DIR OUT_JSON
"""

import json
import math
import sys

import pyarrow.parquet as pq

TARGETS = {"MoS2", "MoSe2", "S2W", "Se2W"}


def chalcogen_stack(r: dict) -> bool | None:
    """True if the two chalcogen atoms of a 3-site MX2 cell sit at the same in-plane point (2H), False if not (1T)."""
    if r["nsites"] != 3:
        return None
    z = r["atomic_numbers"]
    ch = [p for p, zz in zip(r["positions"], z) if zz in (16, 34)]
    if len(ch) != 2:
        return None
    a1, a2 = r["cell"][0], r["cell"][1]
    det = a1[0] * a2[1] - a1[1] * a2[0]
    dx, dy = ch[0][0] - ch[1][0], ch[0][1] - ch[1][1]
    f1 = (dx * a2[1] - dy * a2[0]) / det
    f2 = (a1[0] * dy - a1[1] * dx) / det
    return abs(f1 - round(f1)) < 1e-3 and abs(f2 - round(f2)) < 1e-3


def main() -> None:
    d, out = sys.argv[1], sys.argv[2]
    co = pq.read_table(d + "/co/co_0.parquet").to_pylist()
    keep = []
    for r in co:
        if r["chemical_formula_reduced"] in TARGETS:
            cell = r["cell"]
            a1, a2 = cell[0], cell[1]
            la1 = math.hypot(a1[0], a1[1])
            la2 = math.hypot(a2[0], a2[1])
            cosang = (a1[0] * a2[0] + a1[1] * a2[1]) / (la1 * la2)
            keep.append({
                "formula_reduced": r["chemical_formula_reduced"],
                "names": r["names"],
                "configuration_id": r["configuration_id"],
                "method": r["method"],
                "software": r["software"],
                "electronic_band_gap_eV": r["electronic_band_gap"],
                "energy_eV": r["energy"],
                "nsites": r["nsites"],
                "a1_len_A": la1,
                "a2_len_A": la2,
                "cos_a1_a2": cosang,
                "positions": r["positions"],
                "atomic_numbers": r["atomic_numbers"],
                "chalcogens_share_inplane_site": chalcogen_stack(r),
                "configuration_metadata": json.loads(r["configuration_metadata"]) if r["configuration_metadata"] else None,
            })
    json.dump(keep, open(out, "w"), indent=1, default=str)
    for k in keep:
        print(k["formula_reduced"], k["names"], k["nsites"], round(k["a1_len_A"], 4), round(k["cos_a1_a2"], 4), k["electronic_band_gap_eV"], k["chalcogens_share_inplane_site"], sorted((k["configuration_metadata"] or {}).keys()))


if __name__ == "__main__":
    main()
