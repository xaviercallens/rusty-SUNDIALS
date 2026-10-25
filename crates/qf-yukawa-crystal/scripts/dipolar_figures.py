"""Figures and a JSON summary for the dipolar-exciton study (all numbers read from results/dipolar/*.json[l]).

Usage: python -I dipolar_figures.py <results/dipolar dir> <out dir>
Writes fig_lattice_gaps.{pdf,png}, fig_relaxed_gaps.{pdf,png}, fig_truncation_residual.{pdf,png}, summary.json.
"""
from __future__ import annotations

import glob
import json
import math
import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

BLUE, ORANGE, AQUA, VIOLET = "#2a78d6", "#eb6834", "#1baf7a", "#6250d6"
INK, INK2, GRID = "#0b0b0b", "#52514e", "#e4e3df"


def style(ax) -> None:
    ax.grid(True, color=GRID, lw=0.6)
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)
    for s in ("left", "bottom"):
        ax.spines[s].set_color(INK2)
    ax.tick_params(colors=INK2, labelsize=8)


def jl(path: str) -> list[dict]:
    return [json.loads(x) for x in open(path) if x.strip()]


def save(fig, out: str, name: str) -> None:
    for ext in ("pdf", "png"):
        fig.savefig(os.path.join(out, f"{name}.{ext}"), dpi=200, bbox_inches="tight")
    plt.close(fig)


def main(res: str, out: str) -> None:
    os.makedirs(out, exist_ok=True)
    gaps = jl(os.path.join(res, "lattice_gaps.jsonl"))
    phys = json.load(open(os.path.join(res, "physical_d.json")))
    refs = jl(os.path.join(res, "references.jsonl"))
    ctl = jl(os.path.join(res, "controls.jsonl")) if os.path.exists(os.path.join(res, "controls.jsonl")) else []
    mains = []
    for p in sorted(glob.glob(os.path.join(res, "main_d*.jsonl"))):
        mains += jl(p)

    # --- figure 1: lattice gaps vs d
    fig, ax = plt.subplots(figsize=(5.2, 3.4))
    ds = [g["d"] for g in gaps]
    ax.plot(ds, [g["rel_gap_square"] for g in gaps], color=BLUE, lw=2, label="square / triangular - 1")
    ax.plot(ds, [g["rel_gap_honeycomb"] for g in gaps], color=ORANGE, lw=2, label="honeycomb / triangular - 1")
    dt = phys["d_tilde"]
    lo, hi = dt["1e+11"], dt["3e+12"]
    ax.axvspan(lo, hi, color=AQUA, alpha=0.15, lw=0)
    for key, lab in (("1e+11", "$10^{11}$"), ("1e+12", "$10^{12}$"), ("3e+12", "$3{\\times}10^{12}$")):
        ax.axvline(dt[key], color=AQUA, lw=1, ls="--")
        ax.text(dt[key], 0.6, lab, rotation=90, fontsize=7, color=INK2, ha="right", va="bottom",
                transform=ax.get_xaxis_transform())
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel(r"$\tilde d = d\,\sqrt{n}$ (interlayer separation / $n^{-1/2}$)", fontsize=9, color=INK)
    ax.set_ylabel("relative energy gap (exact lattice sums)", fontsize=9, color=INK)
    ax.set_title("Lattice energy gaps above the triangular lattice, $V=2(1/r-1/\\sqrt{r^2+d^2})$\n"
                 f"band: MoSe$_2$/WSe$_2$ proxy d = {phys['MoSe2_WSe2_proxy_d_A']:.3f} Å (JARVIS-DFT), "
                 "n = $10^{11}$–$3{\\times}10^{12}$ cm$^{-2}$", fontsize=8, color=INK)
    style(ax)
    ax.legend(fontsize=8, frameon=False)
    save(fig, out, "fig_lattice_gaps")

    # --- figure 2: relaxed configurations, exact gaps
    fig, ax = plt.subplots(figsize=(5.2, 3.4))
    fams = {"uniform": (BLUE, "o", "T=0 quench, uniform start"),
            "uniform_annealed": (AQUA, "s", "annealed + quench"),
            "ctl_neg_incommensurate_box": (ORANGE, "^", "incommensurate box (neg. control)")}
    rows = []
    for m in mains:
        rec = m.get("quench") or m.get("anneal_quench")
        if rec is None or "error" in rec:
            rows.append({"label": m["label"], "d": m["d"], "error": (rec or {}).get("error", "missing")})
            continue
        rows.append({"label": m["label"], "family": m["family"], "d": m["d"], "gap_exact": rec["gap_exact"],
                     "rel_gap_exact": rec["rel_gap_exact"], "gap_sf": rec["gap_sf"], "defects": rec["defects"],
                     "converged": rec["converged"], "max_force": rec["max_force"],
                     "hess_negative": rec.get("hess_negative"), "wall_s": rec["wall_s"]})
    for fam, (c, mk, lab) in fams.items():
        xs = [r["d"] * (1 + 0.04 * (hash(r["label"]) % 5 - 2)) for r in rows if r.get("family") == fam]
        ys = [r["rel_gap_exact"] for r in rows if r.get("family") == fam]
        if xs:
            ax.scatter(xs, ys, s=22, color=c, marker=mk, label=lab, edgecolors="white", linewidths=0.8, zorder=3)
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel(r"$\tilde d = d\,\sqrt{n}$", fontsize=9, color=INK)
    ax.set_ylabel(r"$(e - e_{\rm tri})/e_{\rm tri}$, exact potential", fontsize=9, color=INK)
    ax.set_title("CVODE-relaxed periodic configurations (N = 288/289): all above the triangular lattice?",
                 fontsize=8, color=INK)
    style(ax)
    ax.legend(fontsize=7, frameon=False)
    save(fig, out, "fig_relaxed_gaps")

    # --- figure 3: truncation residual vs Ewald
    fig, ax = plt.subplots(figsize=(5.2, 3.2))
    for r, c in zip(refs, [BLUE, ORANGE, AQUA, VIOLET, INK2]):
        Rs = [x["R"] for x in r["convergence"]]
        res_ = [abs(x["e_tri_full"] - r["e_tri_exact"]) for x in r["convergence"]]
        ax.plot(Rs, res_, color=c, lw=2, marker="o", ms=4, label=f"d = {r['d']}")
    ax.set_xscale("log")
    ax.set_yscale("log")
    ax.set_xlabel("radius R of the direct lattice sum (plus exact uniform tail)", fontsize=9, color=INK)
    ax.set_ylabel(r"$|e_{\rm tri}(R) - e_{\rm tri}^{\rm Ewald}|$ (units of $d^2$)", fontsize=9, color=INK)
    ax.set_title("Lattice-minus-continuum residual of a spherical cutoff (why Ewald is used)", fontsize=8,
                 color=INK)
    style(ax)
    ax.legend(fontsize=7, frameon=False)
    save(fig, out, "fig_truncation_residual")

    ok = [r for r in rows if "error" not in r]
    summ = {
        "n_runs": len(rows), "n_errors": len(rows) - len(ok),
        "min_rel_gap_exact_by_d": {str(d): min((r["rel_gap_exact"] for r in ok if r["d"] == d), default=None)
                                   for d in sorted({r["d"] for r in ok})},
        "P1_full_undercuts": [r for r in ok if r["gap_exact"] < -1e-9 * 4.0],
        "P1b_sf_undercuts": [r for r in ok if r["gap_sf"] < -1e-9 * 4.0],
        "not_converged": [r["label"] + "@" + str(r["d"]) for r in ok if not r["converged"]],
        "perfect_lattice_reached": [r["label"] + "@" + str(r["d"]) for r in ok
                                    if r["defects"] == 0 and abs(r["rel_gap_exact"]) < 1e-8],
        "quench_defected": sum(1 for r in ok if r.get("family") == "uniform" and r["defects"] > 0),
        "quench_total": sum(1 for r in ok if r.get("family") == "uniform"),
        "controls": ctl, "runs": rows,
        "lattice_gap_at_physical_d": {k: min(gaps, key=lambda g: abs(math.log(g["d"] / v))) for k, v in dt.items()},
    }
    json.dump(summ, open(os.path.join(out, "summary.json"), "w"), indent=1)
    print(json.dumps({k: v for k, v in summ.items() if k not in ("runs", "controls", "lattice_gap_at_physical_d")},
                     indent=1))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
