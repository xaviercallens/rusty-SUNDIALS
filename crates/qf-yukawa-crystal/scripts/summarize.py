"""Summarise results/*.jsonl against the preregistered predictions P1-P3 (results/preregistration.json).

Usage: python summarize.py <crate_dir>   -> writes results/summary.json and prints a table.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

REL_TOL_UNDERCUT = 1e-9   # P1 tolerance (relative), as preregistered
REL_TOL_LATTICE = 1e-8    # P2: |e - e_tri| < 1e-8 |e_tri| and 0 defects


def load(p: Path) -> list[dict]:
    return [json.loads(l) for l in p.read_text().splitlines() if l.strip()] if p.exists() else []


def main() -> None:
    res = Path(sys.argv[1]) / "results"
    out: dict = {"references": load(res / "references.jsonl"), "controls": load(res / "controls.jsonl")}
    for kappa_file in sorted(res.glob("main_kappa*.jsonl")):
        rows = load(kappa_file)
        fam: dict[str, dict] = {}
        undercuts = []
        for r in rows:
            for stage in ("quench", "anneal_quench"):
                s = r.get(stage)
                if not s or "error" in s:
                    continue
                f = fam.setdefault(f'{r["family"]}/{stage}', {"runs": 0, "perfect_lattice": 0, "defected": 0, "min_rel_gap": None,
                                                               "max_rel_gap": None, "not_converged": 0, "hess_negative_runs": 0})
                f["runs"] += 1
                g = s["rel_gap"]
                f["min_rel_gap"] = g if f["min_rel_gap"] is None else min(f["min_rel_gap"], g)
                f["max_rel_gap"] = g if f["max_rel_gap"] is None else max(f["max_rel_gap"], g)
                if s["defects"] == 0 and abs(g) < REL_TOL_LATTICE:
                    f["perfect_lattice"] += 1
                if s["defects"] > 0:
                    f["defected"] += 1
                if not s["converged"]:
                    f["not_converged"] += 1
                if s.get("hess_negative", 0) > 0:
                    f["hess_negative_runs"] += 1
                if g < -REL_TOL_UNDERCUT and r["family"] != "ctl_neg_incommensurate_box":
                    undercuts.append({"label": r["label"], "stage": stage, "rel_gap": g})
                if r["family"] == "ctl_neg_incommensurate_box" and g < -REL_TOL_UNDERCUT:
                    undercuts.append({"label": r["label"], "stage": stage, "rel_gap": g, "note": "incommensurate box"})
        errors = [(r["label"], st) for r in rows for st in ("quench", "anneal_quench") if r.get(st, {}).get("error")]
        p2 = {k.split("/")[0]: v["perfect_lattice"] for k, v in fam.items() if k.endswith("anneal_quench") and not k.startswith("ctl_")}
        p3 = {k.split("/")[0]: f'{v["defected"]}/{v["runs"]}' for k, v in fam.items() if k.endswith("/quench") and not k.startswith("ctl_")}
        incomm = fam.get("ctl_neg_incommensurate_box/anneal_quench", {})
        out[kappa_file.stem] = {
            "families": fam,
            "errors": errors,
            "P1_no_undercut": {"pass": not undercuts, "undercuts": undercuts},
            "P2_annealed_reaches_lattice": {"per_family_perfect": p2, "pass": all(v >= 1 for v in p2.values()) and bool(p2)},
            "P3_quench_glassy": {"defected_over_runs": p3},
            "neg_control_incommensurate": {"min_rel_gap": incomm.get("min_rel_gap"), "defected": incomm.get("defected"),
                                           "runs": incomm.get("runs"), "pass": bool(incomm) and incomm.get("min_rel_gap", -1) > 0},
        }
    disk = load(res / "disk.jsonl")
    if disk:
        by: dict[str, list] = {}
        for r in disk:
            by.setdefault(r["geometry"], []).append(r)
        out["disk"] = {g: {"runs": len(v),
                           "interior_defect_fraction_mean": sum(r["interior_defects"] / max(r["interior_counted"], 1) for r in v if "interior_defects" in r) / len(v),
                           "edge_defect_fraction_mean": sum(r["edge_defects"] / max(r["edge_counted"], 1) for r in v if "edge_defects" in r) / len(v),
                           "interior_psi6_local_mean": sum(r["interior_psi6_local"] for r in v if "interior_psi6_local" in r) / len(v),
                           "cluster_radius_mean": sum(r["cluster_radius"] for r in v if "cluster_radius" in r) / len(v)}
                       for g, v in by.items()}
    (res / "summary.json").write_text(json.dumps(out, indent=1))
    print(json.dumps({k: v for k, v in out.items() if k not in ("references", "controls")}, indent=1))


if __name__ == "__main__":
    main()
