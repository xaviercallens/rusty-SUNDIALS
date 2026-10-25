"""Figures for the Bogoliubov-S(k) lane, generated from the JSON results (PDF + PNG).

Usage: make_figures.py RESULTS_DIR OUT_DIR
Inputs: linear_response.json, linear_response_summary.json, helium_tier_l.json.
Palette: validated default categorical slots 1-3 (blue, orange, aqua) of the dataviz skill;
identity is also carried by marker shape / line style (never colour alone).
"""
from __future__ import annotations

import json
import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

BLUE, ORANGE, AQUA = "#2a78d6", "#eb6834", "#1baf7a"
INK, INK2, GRID = "#0b0b0b", "#52514e", "#e4e3df"

plt.rcParams.update({
    "font.size": 9, "axes.edgecolor": INK2, "axes.labelcolor": INK, "xtick.color": INK2,
    "ytick.color": INK2, "axes.grid": True, "grid.color": GRID, "grid.linewidth": 0.6,
    "axes.spines.top": False, "axes.spines.right": False, "lines.linewidth": 2.0,
    "legend.frameon": False, "savefig.dpi": 200,
})


def save(fig: plt.Figure, out: str, name: str) -> None:
    fig.tight_layout()
    for ext in ("pdf", "png"):
        fig.savefig(os.path.join(out, f"{name}.{ext}"))
    plt.close(fig)


def main(rd: str, out: str) -> None:
    os.makedirs(out, exist_ok=True)
    d = json.load(open(os.path.join(rd, "linear_response.json")))
    he = json.load(open(os.path.join(rd, "helium_tier_l.json")))
    g1, dn, g0 = d["sweep_phase_g1"], d["sweep_density_g1"], d["sweep_phase_g0"]
    xs = np.logspace(np.log10(0.04), np.log10(6), 400)
    k = 2 * xs
    e0 = k * k / 2
    eb = np.sqrt(e0 * (e0 + 2))

    # Fig 1: dispersion
    fig, ax = plt.subplots(figsize=(4.6, 3.4))
    ax.loglog(xs, eb, color=BLUE, label=r"$\varepsilon_B=\sqrt{\varepsilon_0(\varepsilon_0+2gn)}$")
    ax.loglog(xs, e0, color=ORANGE, ls="--", label=r"$\varepsilon_0=k^2/2$")
    ax.loglog([r["x"] for r in g1], [r["omega_zc"] for r in g1], "o", ms=5, mfc="none", mec=BLUE,
              mew=1.4, label="GP, g=1, phase kick")
    ax.loglog([r["x"] for r in dn], [r["omega_zc"] for r in dn], "+", ms=7, color=BLUE, mew=1.2,
              label="GP, g=1, density kick")
    ax.loglog([r["x"] for r in g0], [r["omega_zc"] for r in g0], "s", ms=4.5, mfc="none", mec=ORANGE,
              mew=1.2, label="GP, g=0 (control)")
    ax.set_xlabel(r"$k/k_s$")
    ax.set_ylabel(r"$\omega(k)$  [$\hbar=m=1$, $gn=1$]")
    ax.legend(fontsize=7.5, loc="upper left")
    save(fig, out, "fig_dispersion")

    # Fig 2: S / threshold
    fig, ax = plt.subplots(figsize=(4.6, 3.4))
    ax.semilogx(xs, (e0 / eb) / np.sqrt(xs / 2), color=BLUE, label=r"$S_B/\sqrt{k/2k_s}$ (exact)")
    ax.semilogx([r["x"] for r in g1], [r["S_freq"] / r["bound"] for r in g1], "o", ms=5, mfc="none",
                mec=BLUE, mew=1.4, label=r"GP g=1: $\varepsilon_0/\omega$")
    ax.semilogx([r["x"] for r in g1], [r["S_amp"] / r["bound"] for r in g1], "x", ms=5, color=AQUA,
                mew=1.2, label="GP g=1: f-sum amplitude")
    ax.semilogx([r["x"] for r in g0], [r["S_freq"] / r["bound"] for r in g0], "s", ms=4.5, mfc="none",
                mec=ORANGE, mew=1.2, label="GP g=0 (control): S=1")
    ax.axhline(1.0, color=INK2, lw=1.0, ls=":")
    ax.text(0.045, 1.08, "test fires above this line", color=INK2, fontsize=7.5)
    ax.set_yscale("log")
    ax.set_xlabel(r"$k/k_s$")
    ax.set_ylabel(r"$S(k)\,/\,\sqrt{k/2k_s}$")
    ax.legend(fontsize=7.5, loc="upper right")
    save(fig, out, "fig_s_over_bound")

    # Fig 3: convergence
    s = json.load(open(os.path.join(rd, "linear_response_summary.json")))
    fig, ax = plt.subplots(figsize=(4.6, 3.4))
    styles = {0.25: ("o", BLUE), 1.0: ("s", ORANGE), 4.0: ("^", AQUA)}
    for c in s["dt_convergence"]:
        mk, col = styles[c["x"]]
        ax.loglog(c["dt"], c["rel_err"], marker=mk, color=col, ms=5, lw=1.5,
                  label=fr"Strang, $\omega$, $k/k_s={c['x']:g}$")
    cr = s["dt_convergence_reference"][0]
    ax.loglog(cr["dt"], cr["S_amp_rel_err"], marker="D", color=ORANGE, ls="--", ms=4.5, lw=1.5,
              label=r"reference splitting, amplitude, $k/k_s=1$")
    dt0 = np.array(cr["dt"])
    ax.loglog(dt0, 0.5 * cr["S_amp_rel_err"][0] * (dt0 / dt0[0]) ** 2, color=INK2, lw=0.8, ls=":")
    ax.text(dt0[2], 0.5 * cr["S_amp_rel_err"][0] * (dt0[2] / dt0[0]) ** 2 * 0.4, r"slope 2", color=INK2, fontsize=7.5)
    ax.set_xlabel(r"$\Delta t$")
    ax.set_ylabel("relative error")
    ax.legend(fontsize=7, loc="lower right")
    save(fig, out, "fig_convergence")

    # Fig 4: helium
    rows = he["rows"]
    kk = np.array([r["k"] for r in rows])
    fig, axs = plt.subplots(1, 2, figsize=(7.2, 3.2))
    ax = axs[0]
    ax.plot(kk, [r["S_up"] for r in rows], "o", ms=3.5, mfc="none", mec=BLUE, mew=1.1,
            label=r"$^4$He: Feynman upper bound $\varepsilon_0/\varepsilon$")
    ax.plot(kk, [r["threshold"] for r in rows], color=ORANGE, label=r"threshold $\sqrt{k/2k_s}$")
    ax.plot(kk, [r["S_B"] for r in rows], color=AQUA, ls="--", label=r"Bogoliubov $S_B$ (same $c$)")
    ks_silent = he["silent_all_k_up_to"]
    ax.axvspan(0, ks_silent + 0.025, color=GRID, alpha=0.7, lw=0)
    ax.text(0.08, 2.6, "silent\n(proved\ngiven Feynman)", fontsize=7, color=INK2)
    ax.text(2.35, 0.12, "undecided\n(needs measured S)", fontsize=7, color=INK2)
    ax.set_ylim(0, 3.2)
    ax.set_xlabel(r"$k$ [$\AA^{-1}$]")
    ax.set_ylabel("structure factor")
    ax.legend(fontsize=6.8, loc="center right", bbox_to_anchor=(1.02, 0.43))
    ax = axs[1]
    ax.plot(kk, [r["eps"] for r in rows], "o", ms=3.5, mfc="none", mec=BLUE, mew=1.1,
            label=r"$^4$He measured $\varepsilon(k)$")
    ax.plot(kk, [r["eps_B"] for r in rows], color=AQUA, ls="--", label=r"Bogoliubov $\varepsilon_B$ (same $c$)")
    ax.plot(kk, [r["eps0"] for r in rows], color=ORANGE, label=r"free $\varepsilon_0$")
    ax.set_xlabel(r"$k$ [$\AA^{-1}$]")
    ax.set_ylabel("energy [meV]")
    ax.set_ylim(0, 6)
    ax.legend(fontsize=6.8, loc="upper left")
    save(fig, out, "fig_helium")
    print("figures written to", out)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
