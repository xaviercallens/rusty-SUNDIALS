"""Figures and a compact summary for the exact-potential erratum fix (results/exact/).

Usage: python -I exact_figures.py CRATE_DIR OUT_DIR
Writes fig_cutoff_sweep.{pdf,png}, fig_relaxed_gaps.{pdf,png} (if relax results exist) and exact_summary.json.
Everything plotted is floating point (exploration tier).
"""

import glob
import json
import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

BLUE, ORANGE, AQUA, INK, MUTED = "#2a78d6", "#eb6834", "#1baf7a", "#0b0b0b", "#52514e"


def style(ax):
    ax.grid(True, color="#e4e3df", lw=0.6)
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)
    ax.tick_params(colors=MUTED, labelsize=8)


def main() -> None:
    crate, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    rows = [json.loads(l) for l in open(f"{crate}/results/exact/exact_refs.jsonl")]
    lat = {r["kappa"]: r for r in rows if r["kind"] == "lattice"}
    summary = {"lattice": {}, "relax": []}
    fig, axes = plt.subplots(1, 2, figsize=(8.0, 3.2), sharey=True)
    for ax, kappa in zip(axes, (1, 2)):
        sw = [r for r in rows if r["kind"] == "sweep" and r["kappa"] == kappa]
        rc = [r["rc"] for r in sw]
        for key, lab, col, mk in (("sf_err", "shifted force (published)", ORANGE, "o"),
                                  ("plain_err", "plain cut", BLUE, "s"),
                                  ("plain_tail_err", "plain cut + continuum tail", AQUA, "^")):
            ys = [max(abs(r["tri"][key]), 1e-17) for r in sw]
            ax.semilogy(rc, ys, color=col, lw=2, marker=mk, ms=5, label=lab)
        gap = lat[kappa]["gap_exact"]
        ax.axhline(gap, color=INK, lw=1, ls="--")
        ax.text(rc[-1], gap * 1.6, "square $-$ triangular gap", ha="right", fontsize=7, color=MUTED)
        ax.axvline(8, color=MUTED, lw=0.8, ls=":")
        ax.set_title(f"$\\kappa={kappa}$, triangular lattice", fontsize=9, color=INK)
        ax.set_xlabel("cutoff $r_c$", fontsize=8, color=MUTED)
        style(ax)
        summary["lattice"][kappa] = {k: lat[kappa][k] for k in ("gap_exact", "gap_sf_rc8", "gap_change")}
        summary["lattice"][kappa]["tri_exact"] = lat[kappa]["tri"]["exact_partial"]
        summary["lattice"][kappa]["square_exact"] = lat[kappa]["square"]["exact_partial"]
        summary["lattice"][kappa]["tri_exact_minus_sf"] = lat[kappa]["tri"]["exact_minus_sf"]
    axes[0].set_ylabel("|error in energy per particle|", fontsize=8, color=MUTED)
    axes[0].legend(fontsize=7, frameon=False, loc="lower left")
    fig.tight_layout()
    for ext in ("pdf", "png"):
        fig.savefig(f"{out}/fig_cutoff_sweep.{ext}", dpi=200)
    plt.close(fig)

    published = {}
    for kappa in (1, 2):
        for l in open(f"{crate}/results/main_kappa{kappa}.jsonl"):
            o = json.loads(l)
            for tag in ("quench", "anneal_quench"):
                if tag in o and "e" in o[tag]:
                    published[(kappa, o["label"], tag)] = o[tag]["e"]
    pts = []
    for p in sorted(glob.glob(f"{crate}/results/exact/exact_relax_kappa*.jsonl")):
        for l in open(p):
            o = json.loads(l)
            for tag in ("quench", "anneal_quench"):
                if tag in o and "exact_gap" in o[tag]:
                    s = o[tag]
                    pub = published.get((o["kappa"], o["label"], tag))
                    pts.append({"label": o["label"], "family": o["family"], "kappa": o["kappa"], "stage": tag,
                                "published_sf_e": pub, "rerun_sf_e": s["sf_e"],
                                "sf_reproduces_published": pub is not None and abs(pub - s["sf_e"]) < 1e-13,
                                "sf_gap": s["sf_gap"], "exact_gap_at_sf_min": s["exact_gap_at_sf_min"],
                                "exact_gap": s["exact_gap"], "gap_change": s["exact_gap"] - s["sf_gap"],
                                "below_exact_floor": s["exact_gap"] < 0, "sf_defects": s["sf_defects"],
                                "exact_defects": s["exact_defects"], "hess_negative": s["hess_negative"],
                                "converged": s["converged"], "max_displacement": s["max_displacement"]})
    summary["relax"] = pts
    if pts:
        fig, axes = plt.subplots(1, 2, figsize=(8.0, 3.4))
        for ax, kappa in zip(axes, (1, 2)):
            sel = [p for p in pts if p["kappa"] == kappa]
            for fam, col, mk in (("hf_oc20", BLUE, "o"), ("uniform", ORANGE, "s"),
                                 ("ctl_neg_incommensurate_box", AQUA, "^")):
                q = [p for p in sel if p["family"] == fam]
                if q:
                    ax.scatter([p["sf_gap"] for p in q], [p["exact_gap"] for p in q], s=36, color=col, marker=mk,
                               edgecolor="white", linewidth=1, label=fam.replace("ctl_neg_", ""), zorder=3)
            if sel:
                lo = min(min(p["sf_gap"], p["exact_gap"]) for p in sel)
                hi = max(max(p["sf_gap"], p["exact_gap"]) for p in sel)
                ax.plot([lo, hi], [lo, hi], color=MUTED, lw=0.8, ls="--")
            ax.set_title(f"$\\kappa={kappa}$", fontsize=9, color=INK)
            ax.set_xlabel("gap to $e_{tri}$, shifted force (published protocol)", fontsize=8, color=MUTED)
            ax.set_ylabel("gap to exact $e_{tri}$, exact potential", fontsize=8, color=MUTED)
            style(ax)
        axes[0].legend(fontsize=7, frameon=False)
        fig.tight_layout()
        for ext in ("pdf", "png"):
            fig.savefig(f"{out}/fig_relaxed_gaps.{ext}", dpi=200)
        plt.close(fig)
    json.dump(summary, open(f"{out}/exact_summary.json", "w"), indent=1)
    print(json.dumps(summary["lattice"], indent=1))
    for p in pts:
        print(p)


if __name__ == "__main__":
    main()
