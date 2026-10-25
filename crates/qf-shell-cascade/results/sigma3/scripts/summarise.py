"""Summarise sigma3_study.json into gate checks (summary.json). Usage: summarise.py STUDY_JSON OUT_JSON"""
import json
import sys

import numpy as np


def ok(rows: list) -> list:
    return [r for r in rows if "error" not in r]


def main() -> None:
    st = json.load(open(sys.argv[1]))
    s: dict = {"git_sha": st["git_sha"], "D": st["D"], "total_wall_s": st["total_wall_s"]}
    errors = [r for k, v in st.items() if isinstance(v, list) for r in v if "error" in r]
    s["failed_runs"] = errors

    def ratio0(r: dict) -> float:
        return 2 * r["D"] * r["Omega0"] / r["bound_rhs"]

    # positive convergence + negative control
    conv = {}
    for sec in ("P1_positive_convergence", "N1_negative_perturbed"):
        for n in (4, 6):
            rows = sorted([r for r in ok(st[sec]) if r["n_max"] == n], key=lambda r: -r["rtol"])
            conv[f"{sec}_n{n}"] = [{"rtol": r["rtol"], "max_rel_dS": r["max_rel_dS"], "max_rel_dH": r["max_rel_dH"],
                                    "S_drift_signed": r["S_drift_signed"], "steps": r["steps"]} for r in rows]
    s["convergence"] = conv
    pos_shrinks = all(c[-1]["max_rel_dS"] < c[0]["max_rel_dS"] / 100 and c[-1]["max_rel_dH"] < c[0]["max_rel_dH"] / 100
                      for k, c in conv.items() if k.startswith("P1"))
    neg_flat = all(min(x["max_rel_dS"] for x in c) > 1e-4 and max(x["max_rel_dS"] for x in c) / min(x["max_rel_dS"] for x in c) < 1.5
                   for k, c in conv.items() if k.startswith("N1"))
    # separation at the tightest tolerance
    sep = {}
    for n in (4, 6):
        p = conv[f"P1_positive_convergence_n{n}"][-1]
        q = conv[f"N1_negative_perturbed_n{n}"][-1]
        sep[f"n{n}"] = {"dS_ratio_neg_over_pos": q["max_rel_dS"] / p["max_rel_dS"],
                        "dH_ratio_neg_over_pos": q["max_rel_dH"] / p["max_rel_dH"]}
    s["neg_over_pos_at_rtol_1e-10"] = sep

    # sigma = 3 sweep
    sw = ok(st["S3_sigma3_sweep"])
    s["S3"] = [{"n_max": r["n_max"], "lean_N": r["lean_N"], "label": r["label"], "steps": r["steps"],
                "wall_s": r["wall_s"], "max_rel_dS": r["max_rel_dS"], "max_rel_dH": r["max_rel_dH"],
                "S_drift_signed": r["S_drift_signed"], "Omega0": r["Omega0"], "sup_Omega": r["sup_Omega"],
                "ratio_t0": ratio0(r), "sup_ratio": r["sup_ratio"]} for r in sw]
    s["S3_max_sup_ratio"] = max(r["sup_ratio"] for r in sw)
    s["S3_max_rel_dS"] = max(r["max_rel_dS"] for r in sw)
    s["S3_max_rel_dH"] = max(r["max_rel_dH"] for r in sw)

    # sigma = 2 vs 3
    c2 = ok(st["C2_sigma2_vs_sigma3"])
    tab = []
    for sig in (2.0, 3.0):
        rows = [r for r in c2 if r["sigma"] == sig]
        x = np.array([r["n_max"] for r in rows], dtype=float)
        y = np.array([r["sup_ratio"] for r in rows])
        slope_all = float(np.polyfit(x, np.log2(y), 1)[0])
        slope_last2 = float(np.log2(y[-1] / y[-2]))
        tab.append({"sigma": sig, "rows": [{"n_max": r["n_max"], "sup_ratio": r["sup_ratio"], "ratio_t0": ratio0(r),
                                            "max_rel_dS": r["max_rel_dS"], "S_drift_signed": r["S_drift_signed"],
                                            "steps": r["steps"]} for r in rows],
                    "log2_slope_fit": slope_all, "log2_step_last_pair": slope_last2})
    s["C2"] = tab

    # data-driven
    w = ok(st["W_well_data"])
    s["W"] = [{"label": r["label"], "n_max": r["n_max"], "max_rel_dS": r["max_rel_dS"], "max_rel_dH": r["max_rel_dH"],
               "S_drift_signed": r["S_drift_signed"], "ratio_t0": ratio0(r), "sup_ratio": r["sup_ratio"],
               "sup_Omega": r["sup_Omega"], "Omega0": r["Omega0"], "bound_rhs": r["bound_rhs"]} for r in w]
    s["W_max_sup_ratio"] = max(r["sup_ratio"] for r in w)
    s["W_n_runs"] = len(w)
    s["W_runs_with_sup_above_t0"] = sum(1 for r in w if r["sup_ratio"] > ratio0(r) * (1 + 1e-12))

    gates = {
        "no_failed_runs": not errors,
        "positive_drift_shrinks_100x_from_rtol_1e-6_to_1e-10": pos_shrinks,
        "negative_drift_rtol_independent_and_above_1e-4": neg_flat,
        "sigma3_bound_holds_every_run (S3, C2 sigma3, W)": (s["S3_max_sup_ratio"] <= 1.0 and s["W_max_sup_ratio"] <= 1.0
                                                              and all(r["sup_ratio"] <= 1.0 for r in c2 if r["sigma"] == 3.0)),
        "sigma2_ratio_exceeds_1_and_grows": tab[0]["rows"][-1]["sup_ratio"] > 1 and tab[0]["log2_slope_fit"] > 1,
    }
    s["gates"] = gates
    s["all_gates_pass"] = all(gates.values())
    json.dump(s, open(sys.argv[2], "w"), indent=1)
    print(json.dumps(gates, indent=1))
    print("neg/pos", sep)
    print("S3 max ratio", s["S3_max_sup_ratio"], "dS", s["S3_max_rel_dS"], "dH", s["S3_max_rel_dH"])
    for t in tab:
        print("C2 sigma", t["sigma"], [(r["n_max"], round(r["sup_ratio"], 4), round(r["ratio_t0"], 4), f"{r['S_drift_signed']:.2e}") for r in t["rows"]],
              "slope", t["log2_slope_fit"], "last", t["log2_step_last_pair"])
    print("W max ratio", s["W_max_sup_ratio"], "runs", s["W_n_runs"], "sup above t0", s["W_runs_with_sup_above_t0"])
    print("S3 sup above t0", sum(1 for r in s["S3"] if r["sup_ratio"] > r["ratio_t0"] * (1 + 1e-12)), "of", len(s["S3"]))
    print("S3 signed S drift", [f"{r['S_drift_signed']:.1e}" for r in s["S3"]])
    print("all_gates_pass", s["all_gates_pass"])


if __name__ == "__main__":
    main()
