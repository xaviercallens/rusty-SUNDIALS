"""Summarise linear_response.json (from the bogoliubov_dispersion example) and evaluate gates.

Writes linear_response_summary.json with every gate, its threshold and its measured value.
"""
from __future__ import annotations

import json
import math
import sys


def max_abs(rows: list[dict[str, float]], key: str) -> float:
    return max(abs(r[key]) for r in rows)


def main(inp: str, out: str) -> None:
    d = json.load(open(inp))
    g1 = d["sweep_phase_g1"]
    dn = d["sweep_density_g1"]
    g0 = d["sweep_phase_g0"]
    gates: dict[str, dict[str, object]] = {}

    def gate(name: str, value: float, op: str, thr: float, note: str) -> None:
        ok = value < thr if op == "<" else value > thr
        gates[name] = {"value": value, "op": op, "threshold": thr, "pass": ok, "note": note}

    gate("pos_phase_g1_max_rel_err_vs_epsB", max_abs(g1, "rel_err_vs_eps_B_g1"), "<", 1e-4,
         "phase kick, g=1, 200 steps/period: omega vs sqrt(eps0(eps0+2gn))")
    gate("pos_density_g1_max_rel_err_vs_epsB", max_abs(dn, "rel_err_vs_eps_B_g1"), "<", 1e-4,
         "density kick, g=1: independent frequency readout")
    gate("pos_S_amp_vs_S_freq_max_rel", max(abs(r["S_amp"] - r["S_freq"]) / r["S_freq"] for r in g1),
         "<", 5e-3, "f-sum-rule amplitude estimator vs period estimator (eps=1e-3)")
    gate("neg_g0_max_rel_err_vs_eps0", max_abs(g0, "rel_err_vs_eps0"), "<", 1e-9,
         "g=0 control: omega = k^2/2 (split step exact for g=0)")
    small = [r for r in g0 if r["x"] < 1.0]
    gate("neg_g0_min_rel_distance_from_epsB_for_x_lt_1", min(abs(r["rel_err_vs_eps_B_g1"]) for r in small),
         ">", 0.1, "g=0 response is far from the g=1 Bogoliubov curve at small k")
    # Bound level (dual-scale / diffraction test) on the measured S.
    gate("bound_g1_max_S_over_bound_minus_1", max(r["S_freq"] / r["bound"] for r in g1) - 1.0, "<", 1e-4,
         "measured S (g=1) never exceeds sqrt(k/2ks) beyond discretisation error")
    fires_g0 = [r for r in g0 if r["S_freq"] > r["bound"]]
    expect_fire = [r for r in g0 if r["x"] < 2.0]
    gate("bound_g0_fires_on_all_x_lt_2", float(len([r for r in expect_fire if r["S_freq"] > r["bound"]]) == len(expect_fire)),
         ">", 0.5, "free S=1 exceeds sqrt(k/2ks) for every k < 2 ks (all such sweep points)")
    gate("bound_g0_silent_on_all_x_gt_2",
         float(all(r["S_freq"] <= r["bound"] for r in g0 if r["x"] > 2.0)), ">", 0.5,
         "for k > 2 ks the free value 1 is below the threshold (the test is one-sided)")
    # tightness near x=1
    near = min(g1, key=lambda r: abs(r["x"] - 1.0))
    gate("bound_g1_ratio_at_x_near_1_minus_1_abs", abs(near["S_freq"] / near["bound"] - 1.0), "<", 2e-3,
         f"S/bound at the sweep point nearest x=1 (x={near['x']:.4f}); exact Bogoliubov ratio "
         f"{near['S_B_g1'] / near['bound']:.6f}")

    ref = d["sweep_phase_g1_reference"]
    gate("neg_reference_splitting_S_amp_vs_S_freq_max_rel",
         max(abs(r["S_amp"] - r["S_freq"]) / r["S_freq"] for r in ref), ">", 5e-3,
         "recorded failure mode: the Python-reference (first-order) splitting gives a visibly wrong amplitude")
    unc = d["sweep_phase_g1_uncapped"]
    informational = {
        "uncapped_strang_max_rel_err_vs_epsB": max_abs(unc, "rel_err_vs_eps_B_g1"),
        "uncapped_strang_S_amp_vs_S_freq_max_rel": max(abs(r["S_amp"] - r["S_freq"]) / r["S_freq"] for r in unc),
        "reference_max_rel_err_vs_epsB": max_abs(ref, "rel_err_vs_eps_B_g1"),
    }

    def conv_table(rows: list[dict[str, float]]) -> list[dict[str, object]]:
        out_rows = []
        for x in sorted({round(r["x"], 6) for r in rows}):
            rs = sorted([r for r in rows if round(r["x"], 6) == x], key=lambda r: -r["safety"])
            errs = [abs(r["rel_err_vs_eps_B_g1"]) for r in rs]
            amp_errs = [abs(r["S_amp"] - r["S_B_g1"]) / r["S_B_g1"] for r in rs]
            ratios = [errs[i] / errs[i + 1] for i in range(len(errs) - 1)]
            out_rows.append({"x": x, "safety": [r["safety"] for r in rs], "dt": [r["dt"] for r in rs],
                             "rel_err": errs, "observed_order": [math.log2(q) for q in ratios],
                             "S_amp_rel_err": amp_errs,
                             "S_amp_observed_order": [math.log2(amp_errs[i] / amp_errs[i + 1])
                                                        for i in range(len(amp_errs) - 1)]})
        return out_rows

    conv_out = conv_table(d["dt_convergence"])
    conv_ref = conv_table(d["dt_convergence_reference"])
    first_orders = [o for c in conv_out for o in c["observed_order"][:3]]
    gate("pos_dt_observed_order_min_first3", min(first_orders), ">", 1.8,
         "Strang splitting: frequency error is second order in dt (first three halvings, eps=1e-4)")
    ref_amp_orders = conv_ref[0]["S_amp_observed_order"]
    gate("neg_reference_amp_order_max", max(ref_amp_orders), "<", 1.5,
         "reference splitting: amplitude error converges at first order only")

    scan = d["eps_scan"]
    se = sorted(scan, key=lambda r: r["eps"])
    eps_err = [{"eps": r["eps"], "rel_err": r["rel_err_vs_eps_B_g1"],
                "S_amp_minus_S_freq": r["S_amp"] - r["S_freq"]} for r in se]
    diag = [{"mode": r["mode"], "rel_err": r["rel_err_vs_eps_B_g1"]} for r in d["diagonal_mode"]]
    gate("pos_isotropy_max_rel_err", max(abs(r["rel_err"]) for r in diag), "<", 1e-4,
         "same |k| on modes (2,0),(1,1),(2,2)")
    gate("gs_residual_max", max(r["gs_residual"] for r in g1 + g0 + dn), "<", 1e-10,
         "imaginary-time relaxed ground state is uniform")

    res = {"gates": gates, "all_pass": all(g["pass"] for g in gates.values()),
           "dt_convergence": conv_out, "dt_convergence_reference": conv_ref, "informational": informational, "eps_scan": eps_err, "diagonal_mode": diag}
    json.dump(res, open(out, "w"), indent=1)
    for k, g in gates.items():
        print(f"{'PASS' if g['pass'] else 'FAIL'} {k}: {g['value']:.3e} {g['op']} {g['threshold']:.1e}")
    for c in conv_out + conv_ref:
        print("conv x=", c["x"], ["%.2e" % e for e in c["rel_err"]], ["%.2f" % o for o in c["observed_order"]],
              "S_amp", ["%.2e" % e for e in c["S_amp_rel_err"]])
    print("informational", informational)
    for e in eps_err:
        print("eps", e)
    print("all_pass", res["all_pass"])


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
