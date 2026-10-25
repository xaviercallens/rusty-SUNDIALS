"""Analyse the lattice_phonon scans: tables (JSON + Markdown) and figures for the QU-1 gate.

usage: python -I phonon_analyze.py DATADIR OUTDIR
Reads DATADIR/<tag>__{coarse,random,patch_rho,patch_i}.csv written by phonon_run_gate.py.
"""

import csv
import glob
import json
import math
import os
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402

data, out = sys.argv[1:3]
os.makedirs(out, exist_ok=True)
RHO = (0.5, math.sqrt(3.0) / 2.0)
LAMBDAS = (0.1, 0.3, 1.0, 3.0, 10.0)


def load(path):
    rows = []
    with open(path) as f:
        for r in csv.DictReader(f):
            rows.append(
                {
                    "x": float(r["x"]),
                    "y": float(r["y"]),
                    "n": int(r["n"]),
                    "es": float(r["e_static"]),
                    "ez": float(r["e_zp"]),
                    "ec": float(r["e_zp_clip"]),
                    "lam": float(r["lam_min"]),
                    "nu": int(r["n_unstable"]),
                }
            )
    return rows


def tag_of(spec):
    return spec.replace(":", "_").replace(",", "_").replace(";", "_")


def richardson(vals):
    """vals = [E(n), E(2n), E(4n)] -> (E_inf, p) with E(n) - E_inf ~ n^-p; p fitted, falls back to 3."""
    e1, e2, e3 = vals
    d1, d2 = e1 - e2, e2 - e3
    if d2 == 0 or d1 / d2 <= 1.0 or not math.isfinite(d1 / d2):
        return e3, float("nan")
    p = math.log2(d1 / d2)
    return e3 - d2 / (2.0**p - 1.0), p


def patch_table(rows):
    """Group a patch file by (x, y); returns {(x,y): {n: row}}."""
    g = {}
    for r in rows:
        g.setdefault((round(r["x"], 9), round(r["y"], 9)), {})[r["n"]] = r
    return g


def analyse(spec):
    tag = tag_of(spec)
    res = {"pot": spec}
    coarse = load(os.path.join(data, f"{tag}__coarse.csv"))
    rnd = load(os.path.join(data, f"{tag}__random.csv"))
    prho = patch_table(load(os.path.join(data, f"{tag}__patch_rho.csv")))
    pi_ = patch_table(load(os.path.join(data, f"{tag}__patch_i.csv")))
    rho_key = (round(RHO[0], 9), round(RHO[1], 9))
    tri = prho[rho_key]
    # n=96 reference for the global scans
    e_tri = tri[96]["ez"]
    es_tri = tri[96]["es"]
    res["tri_stable"] = tri[96]["nu"] == 0
    res["e_zp_tri_n96"] = e_tri
    res["e_static_tri"] = es_tri
    ext, p = richardson([tri[48]["ez"], tri[96]["ez"], tri[192]["ez"]])
    res["e_zp_tri_extrapolated"] = ext
    res["kgrid_exponent_tri"] = p
    res["kgrid_err_n96_tri"] = tri[96]["ez"] - ext
    allp = [r for r in coarse + rnd if r["n"] == 96]
    res["n_points"] = len(allp)
    stable = [r for r in allp if r["nu"] == 0 and math.isfinite(r["ez"])]
    res["n_stable"] = len(stable)
    # distance to the triangular point in the hyperbolic-like coordinate used to exclude the immediate neighbourhood
    res["n_stable_not_tri"] = sum(1 for r in stable if math.hypot(r["x"] - RHO[0], r["y"] - RHO[1]) > 1e-9)
    # static minimiser (Tier L: Montgomery / Bernstein mixtures) and E_zp minimiser among stable lattices
    smin = min(allp, key=lambda r: r["es"])
    res["static_min"] = {"x": smin["x"], "y": smin["y"], "d_es": smin["es"] - es_tri}
    if stable:
        zmin = min(stable, key=lambda r: r["ez"])
        res["zp_min_stable"] = {"x": zmin["x"], "y": zmin["y"], "d_ez": zmin["ez"] - e_tri}
        others = [r for r in stable if math.hypot(r["x"] - RHO[0], r["y"] - RHO[1]) > 1e-6]
        if others:
            o = min(others, key=lambda r: r["ez"] - e_tri)
            res["min_d_ez_stable_competitor"] = {"x": o["x"], "y": o["y"], "d_ez": o["ez"] - e_tri}
    cmin = min(allp, key=lambda r: r["ec"])
    res["clip_min"] = {"x": cmin["x"], "y": cmin["y"], "d_ec": cmin["ec"] - e_tri, "stable": cmin["nu"] == 0}
    # total energies E_static + Lambda * E_zp over stable lattices
    tot = {}
    for lam in LAMBDAS if stable else ():
        best = min(stable, key=lambda r: r["es"] + lam * r["ez"])
        tot[str(lam)] = {
            "x": best["x"],
            "y": best["y"],
            "d_total": (best["es"] + lam * best["ez"]) - (es_tri + lam * e_tri),
        }
    res["total_min_stable"] = tot
    # local analysis around rho with grid extrapolation
    rings = {}
    for (x, y), d in prho.items():
        if (x, y) == rho_key or not all(k in d for k in (48, 96, 192)):
            continue
        r = math.hypot(x - RHO[0], y - RHO[1])
        if d[96]["nu"] > 0:
            rings.setdefault(round(r, 6), []).append(float("nan"))
            continue
        einf, _ = richardson([d[48]["ez"], d[96]["ez"], d[192]["ez"]])
        # hyperbolic distance squared ~ r^2 / y^2 near rho
        rings.setdefault(round(r, 6), []).append((einf - ext) / (r * r / RHO[1] ** 2))
    res["rho_rings"] = {
        str(r): {
            "n": len(v),
            "n_unstable": int(sum(1 for t in v if t != t)),
            "min": float(np.nanmin(v)) if any(t == t for t in v) else None,
            "max": float(np.nanmax(v)) if any(t == t for t in v) else None,
        }
        for r, v in sorted(rings.items())
    }
    # same for the square point
    sq = pi_[(0.0, 1.0)]
    res["square_point"] = {"stable": sq[96]["nu"] == 0, "lam_min": sq[96]["lam"], "n_unstable_n96": sq[96]["nu"]}
    ns = 0
    nt = 0
    for (x, y), d in pi_.items():
        if (x, y) == (0.0, 1.0):
            continue
        nt += 1
        ns += d[96]["nu"] == 0
    res["near_square_stable_fraction"] = ns / nt
    return res, coarse, rnd, prho


def md_table(rows, head):
    s = "| " + " | ".join(head) + " |\n|" + "---|" * len(head) + "\n"
    for r in rows:
        s += "| " + " | ".join(r) + " |\n"
    return s


def main():
    specs = []
    for f in sorted(glob.glob(os.path.join(data, "*__coarse.csv"))):
        t = os.path.basename(f)[: -len("__coarse.csv")]
        with open(f) as fh:
            next(fh)
            specs.append(next(fh).split(",")[0])
        assert tag_of(specs[-1]) == t
    allres = {}
    store = {}
    for spec in specs:
        r, coarse, rnd, prho = analyse(spec)
        allres[spec] = r
        store[spec] = (coarse, rnd, prho)
    with open(os.path.join(out, "scan_summary.json"), "w") as f:
        json.dump(allres, f, indent=1)
    # Markdown tables
    rows = []
    for spec, r in allres.items():
        c = r.get("min_d_ez_stable_competitor")
        rows.append(
            [
                spec,
                str(r["tri_stable"]),
                f"{r['n_stable']}/{r['n_points']}",
                f"{r['e_zp_tri_extrapolated']:.8f}",
                f"{r['kgrid_exponent_tri']:.2f}",
                f"{r['kgrid_err_n96_tri']:.1e}",
                f"{r['static_min']['d_es']:.2e}",
                (f"{c['d_ez']:.2e} at ({c['x']:.3f},{c['y']:.3f})" if c else "none stable"),
                f"{r['clip_min']['d_ec']:.2e} ({'stable' if r['clip_min']['stable'] else 'unstable'}) at ({r['clip_min']['x']:.3f},{r['clip_min']['y']:.3f})",
            ]
        )
    md = md_table(
        rows,
        [
            "potential",
            "tri stable",
            "stable/points",
            "E_zp(tri) extrap.",
            "k-exp p",
            "n=96 err",
            "min dE_static",
            "min dE_zp over stable competitors",
            "min dE_zp,clip (all)",
        ],
    )
    with open(os.path.join(out, "scan_table.md"), "w") as f:
        f.write(md)
    # figures: signed dE_zp of the dynamically stable lattices (zoom on the stable region), unstable shown as grey x
    from matplotlib.colors import SymLogNorm

    n = len(specs)
    cols = 4
    nrow = math.ceil(n / cols)
    fig, axs = plt.subplots(nrow, cols, figsize=(4.4 * cols, 3.6 * nrow), squeeze=False)
    for ax, spec in zip(axs.flat, specs):
        coarse, rnd, prho = store[spec]
        pts = [r for r in coarse + rnd if r["n"] == 96] + [d[96] for d in prho.values() if 96 in d]
        e_tri = allres[spec]["e_zp_tri_n96"]
        pts = [r for r in pts if r["x"] >= 0.0]
        xs = np.array([r["x"] for r in pts])
        ys = np.array([r["y"] for r in pts])
        st = np.array([r["nu"] == 0 for r in pts])
        dz = np.array([(r["ez"] - e_tri) if r["nu"] == 0 else np.nan for r in pts])
        ax.scatter(xs[~st], ys[~st], s=8, c="lightgray", marker="x")
        vmax = max(1e-3, float(np.nanmax(np.abs(dz))) if st.any() else 1e-3)
        sc = ax.scatter(xs[st], ys[st], s=22, c=dz[st], cmap="RdBu_r", norm=SymLogNorm(linthresh=1e-4, vmin=-vmax, vmax=vmax))
        ax.plot([RHO[0]], [RHO[1]], "k*", ms=10)
        ax.set_xlim(0.0, 0.62)
        ax.set_ylim(0.62, 1.25)
        ax.set_title(spec, fontsize=9)
        ax.set_xlabel("Re tau")
        ax.set_ylabel("Im tau")
        plt.colorbar(sc, ax=ax, label="dE_zp = E_zp - E_zp(tri), stable only")
    for ax in list(axs.flat)[n:]:
        ax.axis("off")
    fig.suptitle("Stable lattices coloured by dE_zp (blue: lower than triangular = counterexample); grey x: dynamically unstable; star: triangular", fontsize=9)
    fig.tight_layout()
    fig.savefig(os.path.join(out, "fig_stability_dEzp.png"), dpi=110)
    plt.close(fig)
    # ring curvature plot
    fig, axs = plt.subplots(1, 2, figsize=(11, 4))
    for spec in specs:
        r = allres[spec]["rho_rings"]
        rs = [float(k) for k in r]
        mn = [r[k]["min"] if r[k]["min"] is not None else float("nan") for k in r]
        mx = [r[k]["max"] if r[k]["max"] is not None else float("nan") for k in r]
        axs[0].plot(rs, mn, "o-", label=spec)
        axs[1].plot(rs, mx, "o-", label=spec)
        for ax in axs:
            ax.axhline(0.0, color="k", lw=0.5)
    for ax, t in zip(axs, ("min over angles", "max over angles")):
        ax.set_xscale("log")
        ax.set_yscale("symlog", linthresh=0.05)
        ax.set_xlabel("Euclidean distance from rho")
        ax.set_ylabel("dE_zp / (r^2 / y^2)  (extrapolated in k-grid)")
        ax.set_title(t)
    axs[0].legend(fontsize=6, ncol=2)
    fig.tight_layout()
    fig.savefig(os.path.join(out, "fig_rho_curvature.png"), dpi=110)
    plt.close(fig)
    # k-grid convergence at the triangular point
    fig, ax = plt.subplots(figsize=(6, 4))
    for spec in specs:
        t = store[spec][2][(round(RHO[0], 9), round(RHO[1], 9))]
        ext, _ = richardson([t[48]["ez"], t[96]["ez"], t[192]["ez"]])
        ns = [48, 96, 192]
        ax.loglog(ns, [abs(t[k]["ez"] - ext) / ext + 1e-17 for k in ns], "o-", label=spec)
    ax.loglog([48, 192], [1e-5, 1e-5 * (48 / 192) ** 3], "k--", label="n^-3")
    ax.set_xlabel("k-grid n")
    ax.set_ylabel("|E_zp(n) - E_extrap| / E_extrap")
    ax.legend(fontsize=6, ncol=2)
    fig.tight_layout()
    fig.savefig(os.path.join(out, "fig_kconv.png"), dpi=110)
    plt.close(fig)
    print(md)


main()
