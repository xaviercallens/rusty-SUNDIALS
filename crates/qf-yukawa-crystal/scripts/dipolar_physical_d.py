"""Physical interlayer separation d for TMD bilayers from the JARVIS-DFT rows extracted by dipolar_tmd_extract.py.

Selection (stated before looking at the numbers it produces): semiconducting rows (electronic_band_gap > 0) of
MoS2 / MoSe2 / WS2 / WSe2 whose structure is a stack of >= 2 MX2 layers (>= 2 metal planes with a spacing between
4.5 and 8 Angstrom, i.e. a van der Waals gap, not a polytype with metal planes < 4.5 A apart). For each such row the
adjacent metal-plane spacings below 8 A are taken (vacuum gaps excluded). Rows are deduplicated by their name list.
Monolayer (d2-*) rows give a and t_XX only, never d. The heterobilayer MoSe2/WSe2 value is the mean of the two
homo-stack values (proxy; JARVIS has no heterobilayer row on the Hub).

Then d_tilde = d sqrt(n) for exciton densities n in cm^-2 (spacing convention: lengths in units of n^-1/2; the
triangular-lattice spacing is a = sqrt(2/sqrt3) n^-1/2 = 1.0746 n^-1/2).

Usage: python -I dipolar_physical_d.py <jarvis_tmd_structures.json> <out.json>
"""
from __future__ import annotations

import json
import math
import statistics
import sys

DENSITIES = [1e10, 1e11, 3e11, 1e12, 3e12]


def main(src: str, out: str) -> None:
    rows = json.load(open(src))
    seen: set[str] = set()
    stacks: dict[str, list[dict]] = {}
    mono: dict[str, list[dict]] = {}
    for r in rows:
        key = json.dumps(r.get("names"), sort_keys=True)
        if key in seen:
            continue
        seen.add(key)
        gap = r.get("electronic_band_gap") or 0.0
        sp = r.get("metal_plane_spacings", [])
        names = r.get("names") or []
        is_mono = any(str(n).startswith("d2-") for n in names)
        if gap <= 0:
            continue
        if is_mono:
            mono.setdefault(r["formula"], []).append({"names": names, "a": r["a_in_plane"], "t_XX": r["t_XX"],
                                                      "gap_eV": gap})
            continue
        vdw = [s for s in sp if 4.5 < s < 8.0]
        if len(r.get("metal_planes_z", [])) < 2 or not vdw or any(s < 4.5 for s in sp):
            continue
        stacks.setdefault(r["formula"], []).append({"names": names, "spacings_A": vdw, "gap_eV": gap,
                                                    "natoms": r["natoms"], "method": r.get("method"),
                                                    "stacking_period_A": r["stacking_period"]})
    summary = {}
    for f, lst in stacks.items():
        allsp = [s for x in lst for s in x["spacings_A"]]
        summary[f] = {"rows": lst, "d_mean_A": statistics.mean(allsp), "d_min_A": min(allsp), "d_max_A": max(allsp),
                      "n_spacings": len(allsp)}
    res = {"homo_stacks": summary, "monolayers": mono}
    if "MoSe2" in summary and "WSe2" in summary:
        d = 0.5 * (summary["MoSe2"]["d_mean_A"] + summary["WSe2"]["d_mean_A"])
        res["MoSe2_WSe2_proxy_d_A"] = d
        res["d_tilde"] = {f"{n:.0e}": d * 1e-8 * math.sqrt(n) for n in DENSITIES}
        res["d_tilde_convention"] = "d * sqrt(n), lengths in units of n^-1/2"
    if "MoS2" in summary:
        res["MoS2_stack_d_A"] = summary["MoS2"]["d_mean_A"]
        res["d_tilde_MoS2_stack"] = {f"{n:.0e}": summary["MoS2"]["d_mean_A"] * 1e-8 * math.sqrt(n) for n in DENSITIES}
    json.dump(res, open(out, "w"), indent=1, default=str)
    for f, s in summary.items():
        print(f, "d_mean", round(s["d_mean_A"], 4), "range", round(s["d_min_A"], 4), round(s["d_max_A"], 4),
              "rows", [x["names"][-1] for x in s["rows"]], "methods", {str(x["method"]) for x in s["rows"]})
    for f, lst in mono.items():
        print("monolayer", f, [(round(x["a"][0], 4), [round(t, 4) for t in x["t_XX"]]) for x in lst])
    print("proxy MoSe2/WSe2 d (A):", res.get("MoSe2_WSe2_proxy_d_A"))
    print("d_tilde:", res.get("d_tilde"))
    print("MoS2 stack d_tilde:", res.get("d_tilde_MoS2_stack"))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
