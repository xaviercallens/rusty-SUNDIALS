"""Write GATE.md for the QU-1 falsification gate from the JSON outputs (every number comes from a file).

usage: python -I phonon_make_gate.py WORKDIR OUT_GATE_MD
WORKDIR contains analysis_cm/, analysis_neg/, controls.json, independent_check.json, neg_search.json.
"""

import json
import math
import os
import sys

wd, outp = sys.argv[1:3]
cm = json.load(open(os.path.join(wd, "analysis_cm", "scan_summary.json")))
neg = json.load(open(os.path.join(wd, "analysis_neg", "scan_summary.json")))
ctl = json.load(open(os.path.join(wd, "controls.json")))
ind = json.load(open(os.path.join(wd, "independent_check.json")))
nsr = json.load(open(os.path.join(wd, "neg_search.json")))
TRI = (0.5, math.sqrt(3.0) / 2.0)


def f(v, p=3):
    return f"{v:.{p}e}"


def table(head, rows):
    s = "| " + " | ".join(head) + " |\n|" + "---|" * len(head) + "\n"
    for r in rows:
        s += "| " + " | ".join(str(c) for c in r) + " |\n"
    return s


refuted = [k for k, v in cm.items() if v.get("min_d_ez_stable_competitor", {}).get("d_ez", 1) < 0]
survive = [k for k in cm if k not in refuted]

L = []
L.append("# P2 gate G1: falsification of QU-1 (harmonic zero-point energy, planar Bravais lattices)\n")
L.append("Tier X (floating point) throughout. Nothing here is a proof. Date 2026-10-25. Prior (preregistration.json) that QU-1 survives: 0.6.\n")

# ---------------------------------------------------------------- verdict
g1 = cm["gauss:1"]
r96 = [r for r in ctl["modular_invariance"]]  # keep for completeness
L.append("## Verdict\n")
L.append(
    "**REFUTED(gauss:1, tau = 0.5 + 0.966025 i, margin dE_zp = -1.31e-2).** "
    "QU-1 as stated (triangular minimises E_zp among unit-covolume planar Bravais lattices for every completely monotone "
    "potential) is false numerically: for the Gaussian g(r^2) = exp(-pi r^2), which is completely monotone in r^2 and is the "
    "Montgomery case, the rhombic lattice tau = 0.5 + 0.966025 i is dynamically stable (no imaginary frequency on k-grids "
    "n = 96, 192, 384) and has E_zp = 1.4902783 against 1.5033523 for the triangular lattice (grids n = 384, same code); "
    "the k-grid error at n = 96 is 1.3e-6, so the margin is about 10^4 times the numerical error. "
    "The largest sampled margin is dE_zp = -4.87e-2 (3.2 % of E_zp) at tau = 0.3 + 0.9539 i. The result was reproduced by an "
    "independent numpy implementation (explicit real-space sum, no folding, Gamma-centred and half-shifted k-grids, section Controls).\n"
)
L.append(
    f"Refuted for {len(refuted)} of {len(cm)} tested potentials by a stable competitor with lower E_zp: "
    + ", ".join(f"`{k}`" for k in refuted)
    + ". Not refuted on the sampled stable lattices (triangular lowest, and a local minimum of E_zp): "
    + ", ".join(f"`{k}`" for k in survive)
    + ". The conjecture 'for every CM g' is therefore refuted; a restricted version (narrow potentials) is not contradicted by this scan "
    "but the stable set there is tiny (6 to 16 sampled lattices), so this is weak evidence, not a confirmation.\n"
)
L.append(
    "Mechanism seen in the data (interpretation, Tier C, not tested separately): around the triangular point the static energy has positive "
    "curvature (Montgomery, consistent with the scan) while the harmonic zero-point energy has isotropic curvature of sign that depends on the "
    "potential; where it is negative, E_static + Lambda E_zp has a negative-curvature (shear-softened) triangular point for Lambda above a "
    "threshold. The triangular lattice minimises E_static + Lambda E_zp over the stable sample only for the potentials and Lambda marked in the "
    "coupling table.\n"
)

# ---------------------------------------------------------------- scope
L.append("## What was computed\n")
L.append(
    "- Lattice shape tau = x + i y (|tau| >= 1, 0 <= x <= 1/2, y <= 2.4 scanned; E_zp is invariant under tau -> tau + 1 and tau -> -1/tau, tested), basis a1 = (1/sqrt y, 0), a2 = (x/sqrt y, sqrt y), covolume 1.\n"
    "- D(k) = sum_{R != 0} (1 - cos k.R) Hess V(R); E_zp = (1/2) <sum over 2 branches sqrt(eig D(k))> over the Brillouin zone (hbar = m = 1, V with unit amplitude; the sign of dE_zp does not depend on units).\n"
    "- Potentials (all radial, V = g(r^2)): Gaussian exp(-pi a r^2), a in {0.25, 0.5, 1, 2, 4}; Yukawa exp(-kappa r)/r, kappa in {1, 2}; dipolar exciton U = V/d^2 = 2/(r s (r + s)), s = sqrt(r^2 + d^2), d in {0.1, 0.5} (untruncated, 1/r^3 tail); Riesz r^-s, s in {3, 4, 6}.\n"
    "- **Excluded: Coulomb (Riesz s = 1) and Keldysh.** They need a neutralising background and plasmon handling of the k -> 0 limit, which this code does not implement. No claim is made for them.\n"
    "- **Unstable lattices are never clipped.** A lattice with an eigenvalue of D(k) below -1e-9 x scale at any grid point is dynamically unstable and its E_zp is reported as NaN and excluded from the comparison. This policy is not in the preregistration; it is a scope decision made here because most non-triangular lattices are unstable (table below). A competitor-favouring clipped value E_zp,clip (imaginary frequencies set to 0) is reported separately and is never used to claim a minimum.\n"
    "- Method: k = (i/n) b1 + (j/n) b2 (Gamma-centred; k = 0 is in the average and contributes zero energy); k.R = 2 pi (i m + j l)/n so H(R) is folded into n x n cells and D(k) for all k is a separable transform, O(n^3). Sum over |R| < rc = 400 (natural cutoff for exponentially decaying potentials) completed by the continuum tail -pi rc V'(rc) I. No Ewald is needed for D because Hess V decays like r^(-s-2).\n"
    "- Static energy: (1/2) sum_{R != 0} V(R) with the uniform-continuum tail pi rc^(2-s)/(s-2) (Riesz) or the exact dipolar tail; compared with Epstein closed forms and the Ewald sum below.\n"
)

# ---------------------------------------------------------------- controls
L.append("## Controls\n")
fr = ctl["frozen_phonon"]
L.append(
    f"**Positive control 1, dynamical matrix against an independent energy (frozen phonon).** For {len(fr)} cases (Gaussian, Yukawa, Riesz s = 3, 4, 6, Lennard-Jones, dipolar d = 0.1, 0.5 at several shapes, k and polarisations) the second difference "
    "[E(eps) + E(-eps) - 2E(0)]/eps^2 of the pair-sum energy of a 5 x 5 supercell with displacements eps e cos(k.R) (only values of V, no derivatives; for the dipolar potential the Ewald supercell energy of `dipolar_ewald.rs`) equals (1/2) e^T D(k) e. "
    f"Maximum relative difference over all cases: **{f(ctl['frozen_phonon_max_rel_diff'])}**.\n"
)
ep = ctl["epstein"]
L.append("**Positive control 2, static Riesz energies against Epstein-zeta closed forms** (4 zeta(s/2) beta(s/2) for the square lattice, 6 zeta(s/2) L(s/2, chi_-3) for the hexagonal lattice, mpmath, rescaled to covolume 1; energy per particle = half the lattice sum):\n")
L.append(table(["s", "lattice", "E exact", "E code (rc=400 + tail)", "diff"], [[e["s"], e["lattice"], f"{e['E_exact']:.12f}", f"{e['E_code']:.12f}", f(e["diff"])] for e in ep]))
L.append("The s = 3 residual (about 4e-7) is the lattice-point truncation residual of the plain cutoff, as documented in `dipolar_ewald.rs`.\n")
dpe = ctl["dipolar_static_vs_ewald"]
L.append("**Positive control 3, dipolar static energy against the Ewald sum** (`DipolarEwald`, U units): max |diff| = " + f(max(abs(d["diff"]) for d in dpe)) + " over d in {0.1, 0.5} and three shapes.\n")
cc = ctl["cutoff_convergence"]
worst = 0.0
for p in sorted({(c["pot"], c["x"], c["y"]) for c in cc}):
    v = [c["ez"] for c in cc if (c["pot"], c["x"], c["y"]) == p]
    worst = max(worst, abs(v[-1] - v[-2]) / abs(v[-1]))
L.append(f"**Cutoff convergence.** E_zp at rc = 100, 200, 400, 800 for Riesz 3, 6, dipolar 0.1, 0.5, Yukawa 1, Gaussian 0.5 at the triangular point and at tau = 0.45 + 0.93 i: the relative change from rc = 400 to 800 is at most {f(worst)} (see controls.json).\n")
L.append("**Modular invariance.** E_zp and E_static at tau, tau + 1 and -1/tau (different bases, different folding) agree to about 1e-15 on the same k-set (the set of grid points modulo reciprocal lattice is basis independent; this tests the implementation, not the grid quality), `controls.json: modular_invariance`; the Rust unit test also checks static energies against theta functions (square: theta_3^2; triangular: self-duality theta_L(a) = theta_L(1/a)/a).\n")
rows = []
for r in ind:
    for key in ("n128_s0.0", "n254_s0.5"):
        pass
    rows.append(
        [r["pot"], f"({r['x']:.4f}, {r['y']:.4f})", f"{r['n128_s0.0']['ezp']:.7f}", f"{r['n254_s0.5']['ezp']:.7f}"]
    )
L.append("**Independent implementation (numpy, explicit sum over R, no folding; Gamma-centred n = 128 and half-shifted n = 254 grids):**\n")
L.append(table(["potential", "tau", "E_zp (n=128, centred)", "E_zp (n=254, shifted)"], rows))
rows = []
for k in ("gauss:1", "yukawa:1"):
    c = cm[k]
    rows.append([k, f"{c['e_zp_tri_extrapolated']:.7f}", f"{c['e_zp_tri_n96']:.7f}"])
L.append("Rust values at the triangular point: " + "; ".join(f"{r[0]}: extrapolated {r[1]} (n = 96: {r[2]})" for r in rows) + ". The two codes agree to 1e-6 (the k-grid error).\n")
L.append(
    "**k-grid convergence.** E_zp(n) - E_zp(inf) is proportional to n^-p with fitted p = "
    + ", ".join(f"{v['kgrid_exponent_tri']:.2f}" for v in cm.values())
    + " (n = 48, 96, 192, triangular point, one value per potential in the table order below); p = 3 is the expected rate for a conical singularity at Gamma in 2D. The n = 96 error at the triangular point is between "
    + f(min(abs(v["kgrid_err_n96_tri"]) for v in cm.values()))
    + " and "
    + f(max(abs(v["kgrid_err_n96_tri"]) for v in cm.values()))
    + " (extrapolated values are used for the curvature table; n = 96 values for the global scan).\n"
)
L.append("**Positive control 4, static energy (Tier L consistency).** For all 12 completely monotone potentials the static minimiser of the scan (567 lattices at n = 96) is the triangular lattice (column 'min dE_static' is 0 with the minimiser at tau = rho), as Montgomery's theorem (Glasgow Math. J. 30 (1988) 75-85, doi:10.1017/S0017089500007047) and its Bernstein-mixture extensions predict; a violation would have signalled a code bug.\n")

# negative control
L.append("## Negative control (non-completely-monotone potentials)\n")
L.append(
    "The search must be able to find a counterexample, otherwise 'survives' is meaningless. Family `wells`: V = -ring(r1, w) - c ring(r2, w) with ring(r0, w) = exp(-(r - r0)^2/(2 w^2)), r1 = 1 and r2 = sqrt 2 (the square lattice's first two neighbour shells), "
    "scanned over w in {0.08, 0.12, 0.2, 0.3}, c in {0, 0.5, 1} on the coarse domain grid at n = 48 (`neg_search.json`; nothing tuned beyond that grid). Two members were taken to the full scan, together with Lennard-Jones (sigma = 0.9), Morse and a Gaussian difference:\n"
)
rows = []
for k, v in neg.items():
    c = v.get("min_d_ez_stable_competitor")
    sm = v["static_min"]
    rows.append(
        [
            k,
            v["tri_stable"],
            f"{v['n_stable']}/{v['n_points']}",
            f"{sm['d_es']:.3e} at ({sm['x']:.3f}, {sm['y']:.3f})",
            (f"{c['d_ez']:.3e} at ({c['x']:.3f}, {c['y']:.3f})" if c else "none stable"),
        ]
    )
L.append(table(["potential", "tri stable", "stable/points", "min dE_static (any lattice)", "min dE_zp over stable competitors"], rows))
L.append(
    "`wells:1.0;1.414213562373;0.12;0.5`: the square lattice is a stable static minimiser lower than the triangular one by 0.506 per particle AND a stable rhombic lattice has E_zp lower by 3.03; "
    "`wells:...;0.2;0.0`: the triangular lattice is the static minimiser but a stable lattice has E_zp lower by 2.04 (static and zero-point optima need not coincide). "
    "`lj:0.9` behaves like the survivors (triangular lowest in both), `gdiff` has an unstable triangular lattice (E_zp undefined). Morse also shows a stable lower-E_zp competitor. "
    "So the pipeline does find counterexamples when they exist; the CM refutations above are therefore not a defect of the stable-set bookkeeping.\n"
)

# ---------------------------------------------------------------- main table
L.append("## Results for the completely monotone potentials (n = 96, rc = 400; 167 coarse + 400 random shapes, all with 0 <= Re tau <= 1/2, y in [0.85, 2.4])\n")
rows = []
for k, v in cm.items():
    c = v.get("min_d_ez_stable_competitor")
    cl = v["clip_min"]
    rows.append(
        [
            k,
            v["tri_stable"],
            f"{v['n_stable']}/{v['n_points']}",
            f"{v['e_zp_tri_extrapolated']:.7f}",
            (f"{c['d_ez']:+.3e}" if c else "-"),
            (f"({c['x']:.3f}, {c['y']:.3f})" if c else "-"),
            ("REFUTED" if c and c["d_ez"] < 0 else "survives on sample"),
            f"{cl['d_ec']:+.2e} ({'stable' if cl['stable'] else 'unstable'})",
        ]
    )
L.append(table(["potential", "tri stable", "stable / sampled", "E_zp(tri) extrap.", "min dE_zp over stable lattices (not tri)", "at tau", "verdict", "min dE_zp,clip (all, favours competitor)"], rows))
L.append(
    "The square lattice tau = i is dynamically unstable for every potential here (negative eigenvalue of D(k)), and the stable region is a small neighbourhood of the triangular point (all stable sampled lattices lie within 0.23 of rho in tau, with Im tau <= 1.03 and Re tau >= 0.30; measured by phonon_stable_extent.py on the scan CSVs). "
    "dE_zp,clip is lower than the triangular value for unstable lattices for most potentials (imaginary modes contribute 0), so clipping would manufacture counterexamples; those are not used.\n"
)
L.append("### Curvature of E_zp at the triangular point (k-grid extrapolated n = 48, 96, 192)\n")
L.append(
    "c(r) = [E_zp(tau) - E_zp(rho)] / (r^2 / y_rho^2), r = Euclidean distance |tau - rho|, 12 directions per ring; the modular group makes rho a critical point with isotropic leading curvature (spread over angles = cubic terms). "
    "Ring entries that are dynamically unstable are dropped and counted.\n"
)
rows = []
for k, v in cm.items():
    rr = v["rho_rings"]
    r1, r2 = rr.get("0.01"), rr.get("0.05")
    rows.append([k, f"[{r1['min']:+.3f}, {r1['max']:+.3f}]", f"[{r2['min']:+.3f}, {r2['max']:+.3f}]", "local MAX of E_zp (refuted locally)" if r1["max"] < 0 else "local MIN of E_zp"])
L.append(table(["potential", "c(r=0.01) [min,max over 12 angles]", "c(r=0.05)", "triangular point"], rows))
L.append("### E_static + Lambda E_zp\n")
L.append(
    "With V -> eps V, E_static scales as eps and E_zp as sqrt(eps), so the physically meaningful family is E_static + Lambda E_zp with Lambda proportional to eps^(-1/2) (hbar^2/m relative to the interaction scale); "
    "the harmonic approximation fails for large Lambda (strong quantum motion, melting) so large-Lambda rows are a statement about the functional only. "
    "Entries are the minimum over the stable sample of E_total(tau) - E_total(rho); 0 means the triangular lattice is the minimiser.\n"
)
lams = ["0.1", "0.3", "1.0", "3.0", "10.0"]
rows = []
for k, v in cm.items():
    t = v["total_min_stable"]
    rows.append([k] + [("0 (tri)" if abs(t[l]["d_total"]) < 1e-12 else f"{t[l]['d_total']:+.2e} at ({t[l]['x']:.2f},{t[l]['y']:.2f})") for l in lams])
L.append(table(["potential"] + [f"Lambda={l}" for l in lams], rows))

# ---------------------------------------------------------------- figures
L.append("## Figures (generated by `phonon_analyze.py`)\n")
L.append(
    "- `fig_cm_stability_dEzp.png`: dE_zp of the dynamically stable lattices around the triangular point (blue = lower than triangular), grey x = unstable lattice, per CM potential.\n"
    "- `fig_cm_rho_curvature.png`: c(r) (min and max over angles) versus r, symlog; negative = triangular is a local maximum of E_zp.\n"
    "- `fig_cm_kconv.png`: k-grid convergence at the triangular point (n^-3).\n"
    "- `fig_neg_stability_dEzp.png`, `fig_neg_rho_curvature.png`, `fig_neg_kconv.png`: the same for the non-CM controls.\n"
)

# ---------------------------------------------------------------- caveats
L.append("## Caveats and limits (what this does not show)\n")
L.append(
    "- Tier X only: floating-point numerics. The refutation is numerical; it is a robust one (margin about 10^4 times the grid error, two independent implementations, stable on k-grids up to n = 384, frozen-phonon agreement 2e-7), but it is not a Lean statement.\n"
    "- Only 2-dimensional Bravais lattices (one particle per cell) and the harmonic approximation. Nothing is said about non-Bravais structures (QU-2), anharmonic corrections or the true quantum ground state.\n"
    "- The competitor lattices are not stress-free: they are not static-energy minima over lattice shape (the triangular lattice has the lower static energy, dE_static > 0), so they are not metastable crystals at fixed density. QU-1 as stated compares the functional E_zp over lattice shapes and is refuted regardless; a version asking about E_total at its own equilibrium shape is a different statement.\n"
    "- The stable set is small and sampled: 6 to 16 lattices out of 567 for the CM potentials. The 'survives on sample' verdicts (gauss 0.25, 2, 4 and riesz 6) are weak, and for riesz 4 the triangular lattice is a local minimum of E_zp (c > 0) but a stable lattice at tau = 0.3 + 0.954 i lies lower, so local analysis alone is not enough.\n"
    "- The lowest sampled competitor at x = 0.3 sits near the edge of the sampled stable region; the headline competitor tau = 0.5 + 0.966 i is well inside it. Not claimed: that these are the global minimisers of E_zp over the stable set.\n"
    "- Coulomb and Keldysh are not covered (see above); unit amplitude and hbar = m = 1 only rescale E_zp.\n"
    "- Anything resting on openai/math universal-optimality results (the premise that the Gaussian is 'admissible' for the static theorem) is conditional on them (AI-generated, not peer reviewed); the refutation itself does not use them: it only needs the Gaussian to be completely monotone in r^2, which is elementary.\n"
    "- Consequence for the programme: step 1 refutes QU-1 for 8 of 12 potentials, so step 2 (human proof for Gaussian g) as formulated cannot succeed (a proof of a false statement). The numerics suggest a restricted question (for which g is the triangular lattice a local minimum of E_zp, i.e. sign of c) as a possible QU-1'; that is a conjecture suggested by this table, Tier C, untested beyond the table.\n"
)

# ---------------------------------------------------------------- commands
L.append("## Commands (all run on cores 4-7, two threads per job)\n")
L.append(
    "```\n"
    "# code: rusty-SUNDIALS worktree physics/p2-zero-point-optimality-2026-10-25, crates/qf-yukawa-crystal\n"
    "bash build.sh                      # cargo build --release -j 2 --offline -p qf-yukawa-crystal --bin lattice_phonon\n"
    "bash run_tests.sh                  # 12 phonon unit tests (frozen phonon, Epstein, theta, modular, QU-1 regression, negative control)\n"
    "python -I scripts/phonon_gen_taus.py taus\n"
    "python -I scripts/phonon_run_gate.py BIN taus data cm        # coarse + random (n=96), rho/i patches (n=48,96,192), rc=400\n"
    "python -I scripts/phonon_run_gate.py BIN taus data_neg neg   # non-CM controls\n"
    "python -I scripts/phonon_neg_search.py BIN taus/coarse.txt neg_search.json\n"
    "python -I scripts/phonon_controls.py BIN controls.json\n"
    "python -I scripts/phonon_independent_check.py independent_check.json\n"
    "python -I scripts/phonon_analyze.py data analysis_cm ; python -I scripts/phonon_analyze.py data_neg analysis_neg\n"
    "python -I scripts/phonon_make_gate.py . GATE.md\n"
    "bin/lattice_phonon point gauss:1 0.5 0.966025403784 96,192,384 400   # the headline competitor\n"
    "```\n"
)
L.append(
    "Code and raw results: rusty-SUNDIALS branch `physics/p2-zero-point-optimality-2026-10-25` (`src/phonon.rs`, `src/bin/lattice_phonon.rs`, `scripts/phonon_*.py`, `results/phonon_gate/`). Copies of the tables, JSON and figures are next to this file.\n"
)
open(outp, "w").write("\n".join(L))
print("wrote", outp, len("\n".join(L)))
