"""Figures for the H7 sigma = 3 CVODE study (PDF + PNG), generated from the JSON outputs only.

Usage: make_figures.py sigma3_study.json well_spectrum.json OUT_DIR
"""
import json
import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

C = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300"]
INK, MUTED = "#0b0b0b", "#52514e"
plt.rcParams.update({"font.size": 9, "axes.edgecolor": MUTED, "axes.labelcolor": INK, "xtick.color": MUTED,
                     "ytick.color": MUTED, "axes.grid": True, "grid.color": "#e4e3df", "grid.linewidth": 0.6,
                     "axes.spines.top": False, "axes.spines.right": False, "lines.linewidth": 2,
                     "legend.frameon": False})


def save(fig: plt.Figure, out: str, name: str) -> None:
    fig.tight_layout()
    for ext in ("pdf", "png"):
        fig.savefig(os.path.join(out, f"{name}.{ext}"), dpi=200)
    plt.close(fig)


def main() -> None:
    st = json.load(open(sys.argv[1]))
    sp = json.load(open(sys.argv[2]))
    out = sys.argv[3]
    os.makedirs(out, exist_ok=True)

    # Fig 1: conservation vs rtol, positive vs perturbed-coupling negative control.
    fig, axes = plt.subplots(1, 2, figsize=(7.0, 2.9), sharey=True)
    for ax, key, title in ((axes[0], "max_rel_dS", "mass S"), (axes[1], "max_rel_dH", "second invariant H$_3$")):
        i = 0
        for sec, lab in (("P1_positive_convergence", "exact couplings"), ("N1_negative_perturbed", "coupling k$_1$×1.01")):
            for n in (4, 6):
                rows = [r for r in st[sec] if r["n_max"] == n and "error" not in r]
                if not rows:
                    continue
                ax.loglog([r["rtol"] for r in rows], [max(r[key], 1e-17) for r in rows], marker="o", ms=5,
                          color=C[i], label=f"{lab}, n$_{{max}}$={n}", ls="-" if sec.startswith("P") else "--")
                i += 1
        ax.set_xlabel("CVODE rtol (atol = 1e-3 rtol)")
        ax.set_title(f"max relative drift of {title}", fontsize=9, color=INK)
        ax.invert_xaxis()
    axes[0].set_ylabel("max over internal steps")
    axes[1].legend(fontsize=7, loc="center left", bbox_to_anchor=(1.0, 0.5))
    save(fig, out, "fig_conservation_controls")

    # Fig 2: sup ratio vs truncation; sigma = 3 decaying data, and sigma = 2 vs 3 on top-shell data.
    fig, axes = plt.subplots(1, 2, figsize=(7.0, 2.9))
    ax = axes[0]
    rows = [r for r in st["S3_sigma3_sweep"] if "error" not in r]
    ns = sorted({r["n_max"] for r in rows})
    for j, seed_lab in enumerate(("phase seed 1", "phase seed 2")):
        sub = [r for r in rows if r["label"] == "S3_sweep"][j::2]
        ax.plot([r["n_max"] for r in sub], [r["sup_ratio"] for r in sub], marker="o", ms=5, color=C[j], label=seed_lab)
    ax.axhline(1.0, color=MUTED, lw=1, ls=":")
    ax.set_xticks(ns)
    ax.set_xlabel("truncation n$_{max}$ (shells 0..n$_{max}$)")
    ax.set_ylabel(r"$\sup_t\,2D\Omega(t)\,/\,(H_3(0)+S(0)^{3/2})$")
    ax.set_title("σ = 3, decaying data: bound ratio", fontsize=9, color=INK)
    ax.legend(fontsize=7)
    ax = axes[1]
    c2 = [r for r in st["C2_sigma2_vs_sigma3"] if "error" not in r]
    for j, (lab, sig) in enumerate((("σ = 2 (H$_2$ in place of H$_3$)", 2.0), ("σ = 3", 3.0))):
        sub = [r for r in c2 if r["sigma"] == sig]
        x = np.array([r["n_max"] for r in sub])
        y = np.array([r["sup_ratio"] for r in sub])
        ax.semilogy(x, y, marker="o", ms=5, color=C[j], label=lab)
    ax.axhline(1.0, color=MUTED, lw=1, ls=":")
    ax.set_xlabel("truncation n$_{max}$")
    ax.set_ylabel(r"$\sup_t\,2D\Omega(t)\,/\,(H_\sigma(0)+S(0)^{3/2})$")
    ax.set_title("top-shell data: σ = 2 control", fontsize=9, color=INK)
    ax.legend(fontsize=7)
    save(fig, out, "fig_truncation_sweep")

    # Fig 3: The Well shell spectra + controls; and one data-driven trajectory against its bound.
    fig, axes = plt.subplots(1, 2, figsize=(7.0, 2.9))
    ax = axes[0]
    nsh = np.arange(len(sp["snapshots"][0]["E_n"]))
    for j, s in enumerate(sp["snapshots"]):
        en = np.array(s["E_n"])
        ax.semilogy(nsh, en / en.sum(), marker="o", ms=4, color=C[j], label=f"t index {s['time_index']}")
    wn = np.array(sp["control_white_noise"]["E_n"])
    ax.semilogy(nsh, wn / wn.sum(), marker="s", ms=4, color=MUTED, ls="--", lw=1.2, label="white-noise control")
    ax.set_xlabel("dyadic shell n  (k$_x$ ∈ [2$^n$, 2$^{n+1}$))")
    ax.set_ylabel("E$_n$ / Σ E$_n$")
    ax.set_title("The Well, turbulent_radiative_layer_2D", fontsize=9, color=INK)
    ax.legend(fontsize=7)
    ax = axes[1]
    w = [r for r in st["W_well_data"] if "error" not in r and "trace_t_S_H_Omega" in r]
    pick = [r for r in w if r["n_max"] == 7][:1] or w[:1]
    if pick:
        r = pick[0]
        tr = np.array(r["trace_t_S_H_Omega"])
        ax.plot(tr[:, 0], 2 * r["D"] * tr[:, 3], color=C[0], lw=1.2, label=r"$2D\Omega(t)$")
        ax.axhline(r["bound_rhs"], color=C[1], lw=2, label=r"$H_3(0)+S(0)^{3/2}$")
        ax.set_xlabel("t")
        ax.set_title(f"data-driven run {r['label']}, n$_{{max}}$={r['n_max']}", fontsize=9, color=INK)
        ax.legend(fontsize=7)
    save(fig, out, "fig_well_data")
    print("figures written to", out)


if __name__ == "__main__":
    main()
