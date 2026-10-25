"""Keldysh-jellium analysis: cross-language checks, large-r0 log-gas control, material energies, figures (Tier X).

Usage:
  analyze.py prep  RESULTS_DIR HF_ROWS_JSON          -> writes RESULTS_DIR/materials_in.txt (dimensionless r0 sqrt(n))
  analyze.py final RESULTS_DIR HF_ROWS_JSON          -> reads scan.json, large_r0.json, materials_out.json,
                                                       mpmath_refs.json; writes analysis.json and figures (PDF+PNG)

Material input (Tier L): chi_2D (Angstrom, DFT+RPA, freestanding monolayer) from Berkelbach, Hybertsen, Reichman,
arXiv:1305.4972, Table I; r0 = 2 pi chi_2D (their text, after Cudazzo et al.). MoS2 GW value chi_2D = 5.0 A (same paper,
text) gives a range. Vacuum on both sides (eps1 = eps2 = 1) only; no dielectric environment is modelled.
The HF dataset colabfit/JARVIS_C2DB carries no polarizability; it is used only to identify the 2H monolayers and their
PBE lattice constants and band gaps (cross-check against the paper's Table II lattice constants).
"""

import json
import math
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import matplotlib.ticker  # noqa: E402
import mpmath as mp  # noqa: E402
import numpy as np  # noqa: E402

CHI2D_DFT = {"MoS2": 6.60, "MoSe2": 8.23, "WS2": 6.03, "WSe2": 7.18}  # Angstrom, 1305.4972 Table I
CHI2D_GW_MOS2 = 5.0  # Angstrom, 1305.4972 text
A_TABLE2 = {"MoS2": 3.16, "MoSe2": 3.30, "WS2": 3.16, "WSe2": 3.29}  # Angstrom, 1305.4972 Table II (experimental)
HF_FORMULA = {"MoS2": "MoS2", "MoSe2": "MoSe2", "S2W": "WS2", "Se2W": "WSe2"}
COULOMB_EV_A = 14.399645  # e^2/(4 pi eps0) in eV Angstrom (CODATA 2018)
N_CM2 = np.logspace(10, 13, 31)  # illustrative density grid, cm^-2 (no claim about where a Wigner crystal forms)
COLORS = {"MoS2": "#2a78d6", "MoSe2": "#eb6834", "WS2": "#1baf7a", "WSe2": "#eda100"}
MARKERS = {"MoS2": "o", "MoSe2": "s", "WS2": "^", "WSe2": "D"}


def materials() -> dict[str, float]:
    m = {k: 2 * math.pi * v for k, v in CHI2D_DFT.items()}
    m["MoS2 (GW chi)"] = 2 * math.pi * CHI2D_GW_MOS2
    return m


def r0_tilde(r0_A: float) -> np.ndarray:
    n_A2 = N_CM2 * 1e-16
    return r0_A * np.sqrt(n_A2)


def hf_2h_rows(path: str) -> list[dict]:
    """2H rows: 3 sites, hexagonal cell (cos 120 deg), both chalcogens at the same in-plane position."""
    rows = json.load(open(path))
    out = []
    for r in rows:
        if r["nsites"] != 3 or abs(r["cos_a1_a2"] + 0.5) > 1e-3 or r["chalcogens_share_inplane_site"] is not True:
            continue
        out.append(r)
    return out


def prep(res: str) -> None:
    vals = [0.0]
    for r0 in materials().values():
        vals.extend(r0_tilde(r0).tolist())
    with open(res + "/materials_in.txt", "w") as f:
        f.write("\n".join(f"{v:.17e}" for v in vals) + "\n")
    print("wrote", len(vals), "points")


def eta_log(tau: complex) -> mp.mpf:
    q = mp.exp(2j * mp.pi * tau)
    eta = mp.exp(2j * mp.pi * tau / 24) * mp.qp(q)
    return mp.log(mp.sqrt(mp.im(tau)) * abs(eta) ** 2)


def final(res: str, hf_rows_path: str) -> None:
    mp.mp.dps = 30
    scan = json.load(open(res + "/scan.json"))
    refs = json.load(open(res + "/mpmath_refs.json"))
    large = json.load(open(res + "/large_r0.json"))["r0_Etri_Esq"]
    mat = json.load(open(res + "/materials_out.json"))["r0_Etri_Esq"]
    out: dict = {}

    # Cross-language: Rust DLMF quadrature and Rust mixture vs mpmath Struve/Neumann closed form.
    ref = {(round(a, 10), round(b, 10)): v for a, b, v in refs["V"]}
    errs_d, errs_m = [], []
    for r0, r, d, m in scan["potential_r0_r_dlmf_mixture"]:
        v = ref[(round(r0, 10), round(r, 10))]
        errs_d.append(abs(d - v) / v)
        errs_m.append(abs(m - v) / v)
    out["rust_vs_mpmath_V_max_rel_err"] = {"dlmf": max(errs_d), "mixture": max(errs_m), "n_points": len(errs_d)}

    c = scan["coulomb"]
    out["coulomb_positive_control"] = {
        "lattice_tri_minus_jellium_rs_box": c["lattice_r0_0_tri"] - c["jellium_rs_box_tri"],
        "lattice_tri_minus_literature": c["lattice_r0_0_tri"] - c["literature_tri"],
        "lattice_sq_minus_literature": c["lattice_r0_0_sq"] - c["literature_sq"],
        "r0_1e-9_dE_tri": [x for x in scan["coulomb_limit"] if abs(x["r0"] - 1e-9) < 1e-15][0]["dE_tri"],
        "slope_over_pi_at_r0": {f"{x['r0']:.0e}": x["slope_tri_over_pi"] for x in scan["coulomb_limit"]},
    }
    ind = scan["alpha_independence_and_negative_controls"]
    out["alpha_independence_max_rel_spread"] = max(x["spread"] / abs(x["E_alpha_0.8_1.6_3.2"][0]) for x in ind)
    out["neg_wrong_sign_recip_min_rel_spread"] = min(x["neg_wrong_sign_recip_spread"] / abs(x["E_alpha_0.8_1.6_3.2"][0]) for x in ind)
    out["neg_no_background_min_rel_spread"] = min(x["neg_no_background_spread"] / abs(x["E_alpha_0.8_1.6_3.2"][0]) for x in ind)
    out["neg_wrong_sign_kernel"] = scan["neg_wrong_sign_kernel_coulomb_limit"]
    out["scaling_n4"] = scan["scaling_n4"]
    out["kernel_identity_max_rel_err"] = scan["kernel_identity_max_rel_err"]

    # Large-r0 control: r0 (E_sq - E_tri) -> W_sq - W_tri of the 2D log gas = (1/2)[q(tri) - q(sq)],
    # q(tau) = log(sqrt(Im tau) |eta(tau)|^2)  (Sandier-Serfaty, arXiv:1011.4617, Lemma 3.3).
    logw = float((eta_log(mp.exp(1j * mp.pi / 3)) - eta_log(1j)) / 2)
    out["log_gas_limit"] = {
        "eta_formula_W_sq_minus_W_tri": logw,
        "r0_times_gap": [[r0, r0 * (s - t), r0 * (s - t) / logw - 1] for r0, t, s in large],
    }

    # Coulomb gap and the scan.
    sc = scan["scan"]
    r0s = np.array([x["r0"] for x in sc])
    et = np.array([x["E_tri"] for x in sc])
    gap = np.array([x["gap_sq_minus_tri"] for x in sc])
    out["min_gap_over_scan"] = float(gap.min())
    out["all_gaps_positive"] = bool((gap > 0).all())
    gap_c = c["lattice_r0_0_sq"] - c["lattice_r0_0_tri"]

    # Materials.
    vals = {round(r, 12): (t, s) for r, t, s in mat}
    e0 = vals[0.0]
    n_A2 = N_CM2 * 1e-16
    mats = {}
    for name, r0 in materials().items():
        rt = r0_tilde(r0)
        tri = np.array([vals[round(v, 12)][0] for v in rt])
        sq = np.array([vals[round(v, 12)][1] for v in rt])
        pref = COULOMB_EV_A * np.sqrt(n_A2) * 1e3  # meV
        mats[name] = {
            "r0_A": r0,
            "n_cm2": N_CM2.tolist(),
            "r0_sqrt_n": rt.tolist(),
            "E_tri_meV": (pref * tri).tolist(),
            "E_coulomb_tri_meV": (pref * e0[0]).tolist(),
            "ratio_to_coulomb": (tri / e0[0]).tolist(),
            "gap_sq_minus_tri_meV": (pref * (sq - tri)).tolist(),
        }
    out["materials"] = mats

    hf = hf_2h_rows(hf_rows_path)
    out["hf_2h_rows"] = [
        {
            "material": HF_FORMULA[r["formula_reduced"]],
            "c2db_name": r["names"],
            "a_pbe_A": r["a1_len_A"],
            "a_table2_A": A_TABLE2[HF_FORMULA[r["formula_reduced"]]],
            "band_gap_pbe_eV": r["electronic_band_gap_eV"],
            "method": r["method"],
        }
        for r in hf
    ]
    json.dump(out, open(res + "/analysis.json", "w"), indent=1)

    # Figure 1: dimensionless energies vs r0 at density one.
    plt.rcParams.update({"font.size": 9, "axes.spines.top": False, "axes.spines.right": False})
    fig, ax = plt.subplots(1, 2, figsize=(7.2, 2.9))
    ax[0].semilogx(r0s, et / c["lattice_r0_0_tri"], color="#2a78d6", lw=2, label="triangular")
    ax[0].set_xlabel(r"$r_0\sqrt{n}$")
    ax[0].set_ylabel(r"$E_{\rm tri}(r_0)/E_{\rm tri}(0)$")
    ax[0].set_title("Keldysh-jellium Madelung energy", fontsize=9)
    ax[0].grid(color="#e1e0d9", lw=0.5)
    ax[1].loglog(r0s, gap / gap_c, color="#2a78d6", lw=2, label="computed")
    ax[1].loglog(r0s, logw / r0s / gap_c, color="#898781", lw=1, ls="--", label=r"log-gas limit $\Delta W/r_0$")
    ax[1].axhline(1.0, color="#c3c2b7", lw=1, ls=":", label="Coulomb gap")
    ax[1].set_xlabel(r"$r_0\sqrt{n}$")
    ax[1].set_ylabel(r"$(E_{\rm sq}-E_{\rm tri})/(E_{\rm sq}-E_{\rm tri})_{r_0=0}$")
    ax[1].set_title("Square minus triangular (always > 0)", fontsize=9)
    ax[1].legend(frameon=False, fontsize=7)
    ax[1].grid(color="#e1e0d9", lw=0.5)
    fig.tight_layout()
    fig.savefig(res + "/fig_keldysh_scan.pdf")
    fig.savefig(res + "/fig_keldysh_scan.png", dpi=200)

    # Figure 2: material-specific energies (vacuum-suspended monolayers).
    fig, ax = plt.subplots(1, 2, figsize=(7.2, 2.9))
    for name in ["MoS2", "MoSe2", "WS2", "WSe2"]:
        d = mats[name]
        lab = f"{name} ($r_0$={d['r0_A']:.1f} Å)"
        ax[0].semilogx(N_CM2, d["ratio_to_coulomb"], color=COLORS[name], lw=2, marker=MARKERS[name], ms=4, markevery=5, label=lab)
        ax[1].loglog(N_CM2, d["gap_sq_minus_tri_meV"], color=COLORS[name], lw=2, marker=MARKERS[name], ms=4, markevery=5, label=lab)
    g = mats["MoS2 (GW chi)"]
    ax[0].semilogx(N_CM2, g["ratio_to_coulomb"], color=COLORS["MoS2"], lw=1, ls="--", label=r"MoS2, GW $\chi_{2D}$")
    ax[1].loglog(N_CM2, np.array(g["E_coulomb_tri_meV"]) * 0 + (COULOMB_EV_A * np.sqrt(n_A2) * 1e3 * gap_c), color="#898781", lw=1, ls=":", label="Coulomb (r0 = 0)")
    ax[0].set_xlabel(r"density $n$ (cm$^{-2}$)")
    ax[0].set_ylabel(r"$E_{\rm tri}^{\rm Keldysh}/E_{\rm tri}^{\rm Coulomb}$")
    ax[0].set_title("Madelung energy relative to Coulomb", fontsize=9)
    ax[0].legend(frameon=False, fontsize=6.5)
    ax[0].grid(color="#e1e0d9", lw=0.5)
    ax[1].set_xlabel(r"density $n$ (cm$^{-2}$)")
    ax[1].set_ylabel(r"$E_{\rm sq}-E_{\rm tri}$ (meV per electron)")
    ax[1].set_title("Square minus triangular (vacuum)", fontsize=9)
    ax[1].yaxis.set_major_formatter(matplotlib.ticker.ScalarFormatter())
    ax[1].yaxis.set_minor_formatter(matplotlib.ticker.NullFormatter())
    ax[1].set_yticks([0.2, 0.5, 1.0, 2.0, 4.0])
    ax[1].legend(frameon=False, fontsize=6.5)
    ax[1].grid(color="#e1e0d9", lw=0.5)
    fig.tight_layout()
    fig.savefig(res + "/fig_keldysh_materials.pdf")
    fig.savefig(res + "/fig_keldysh_materials.png", dpi=200)
    print(json.dumps({k: v for k, v in out.items() if k != "materials"}, indent=1))


def main() -> None:
    mode, res, hf = sys.argv[1], sys.argv[2], sys.argv[3]
    if mode == "prep":
        prep(res)
    elif mode == "final":
        final(res, hf)
    else:
        raise SystemExit("mode must be prep or final")


if __name__ == "__main__":
    main()
