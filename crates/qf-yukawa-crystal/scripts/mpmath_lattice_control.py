"""Independent high-precision positive/negative control for the exact planar Yukawa lattice energies.

Shares no code with the Rust crate. For kappa in {1, 2} and the triangular / square lattices at density one, computes
E = (1/2) sum_{v != 0} exp(-kappa |v|)/|v| with mpmath at 40 digits two ways:
  (a) direct disk sum to radius R with R - 2 delta >= 64/kappa (omitted tail rigorously < 1e-27), and
  (b) an Ewald-type splitting (Gaussian integral representation, closed forms with erfc) at eta = 0.7.
Then compares with:
  * the Rust f64 values in results/exact/exact_refs.jsonl  -> positive control, must agree to < 1e-9;
  * the published shifted-force values in results/references.jsonl (rc = 8) -> negative control, must DIFFER by
    about the published erratum amount (1.8e-2 at kappa = 1, 9.5e-6 at kappa = 2), i.e. fail the 1e-9 test.

Usage: python -I mpmath_lattice_control.py CRATE_DIR OUT_JSON
"""

import json
import sys
import time

import mpmath as mp

mp.mp.dps = 40


def bases():
    a = mp.sqrt(2 / mp.sqrt(3))
    return {
        "tri": ((a, mp.mpf(0)), (a / 2, a * mp.sqrt(3) / 2), a / mp.sqrt(3)),
        "square": ((mp.mpf(1), mp.mpf(0)), (mp.mpf(0), mp.mpf(1)), mp.sqrt(2) / 2),
    }


def vectors(b0, b1, r):
    """All v = i b0 + j b1, v != 0, |v| < r (b0 on the x axis)."""
    out = []
    jm = int(mp.ceil(r / abs(b1[1]))) + 1
    for j in range(-jm, jm + 1):
        x0 = j * b1[0]
        y = j * b1[1]
        ilo = int(mp.floor((-r - x0) / b0[0])) - 1
        ihi = int(mp.ceil((r - x0) / b0[0])) + 1
        for i in range(ilo, ihi + 1):
            if i == 0 and j == 0:
                continue
            x = x0 + i * b0[0]
            d = mp.sqrt(x * x + y * y)
            if d < r:
                out.append(d)
    return out


def direct(kappa, b0, b1, delta):
    r = mp.mpf(64) / kappa + 2 * delta + 1
    s0 = r - 2 * delta
    area = abs(b0[0] * b1[1] - b0[1] * b1[0])
    tail = mp.pi / area * (1 + delta / s0) * mp.exp(-kappa * s0) / kappa
    ds = vectors(b0, b1, r)
    e = mp.fsum(mp.exp(-kappa * d) / d for d in ds) / 2
    return e, tail, len(ds), r


def ewald(kappa, b0, b1, eta):
    area = abs(b0[0] * b1[1] - b0[1] * b1[0])
    b = kappa / (2 * eta)
    r_real = (mp.mpf(11) + b) / eta  # erfc(11) ~ 1e-54
    real = mp.fsum(
        (mp.exp(kappa * d) * mp.erfc(d * eta + b) + mp.exp(-kappa * d) * mp.erfc(d * eta - b)) / (2 * d)
        for d in vectors(b0, b1, r_real)
    )
    det = b0[0] * b1[1] - b0[1] * b1[0]
    g0 = (2 * mp.pi * b1[1] / det, -2 * mp.pi * b1[0] / det)
    g1 = (-2 * mp.pi * b0[1] / det, 2 * mp.pi * b0[0] / det)
    qcut = 23 * eta  # erfc(11.5) ~ 1e-59
    gm = int(mp.ceil(qcut / min(mp.sqrt(g0[0] ** 2 + g0[1] ** 2), mp.sqrt(g1[0] ** 2 + g1[1] ** 2)))) + 2
    terms = []
    for i in range(-gm, gm + 1):
        for j in range(-gm, gm + 1):
            gx = i * g0[0] + j * g1[0]
            gy = i * g0[1] + j * g1[1]
            q = mp.sqrt(gx * gx + gy * gy + kappa * kappa)
            terms.append(mp.erfc(q / (2 * eta)) / q)
    recip = 2 * mp.pi / area * mp.fsum(terms)
    selfterm = 2 / mp.sqrt(mp.pi) * (eta * mp.exp(-b * b) - kappa * mp.sqrt(mp.pi) / 2 * mp.erfc(b))
    return (real + recip - selfterm) / 2


def main() -> None:
    crate, out = sys.argv[1], sys.argv[2]
    rust = {}
    for line in open(f"{crate}/results/exact/exact_refs.jsonl"):
        o = json.loads(line)
        if o["kind"] == "lattice":
            rust[o["kappa"]] = {"tri": o["tri"]["exact_partial"], "square": o["square"]["exact_partial"]}
    sf = {}
    for line in open(f"{crate}/results/references.jsonl"):
        o = json.loads(line)
        sf[o["kappa"]] = {"tri": o["e_tri_sum"], "square": o["e_square_sum"]}
    # Scratch R = 60 unshifted sums quoted in the textbook, ch3 footnote fn:unshifted (printed f64 values).
    scratch = {1: {"tri": 1.6253213396062602, "square": 1.6348554953171857},
               2: {"tri": 0.3935428489323898, "square": 0.40097424876505755}}
    res = {"dps": mp.mp.dps, "rows": []}
    pos_ok, neg_ok = True, True
    for kappa in (1, 2):
        for name, (b0, b1, delta) in bases().items():
            t0 = time.time()
            e, tail, n, r = direct(mp.mpf(kappa), b0, b1, delta)
            ew = ewald(mp.mpf(kappa), b0, b1, mp.mpf("0.7"))
            d_rust = abs(e - rust[kappa][name])
            d_sf = e - sf[kappa][name]
            row = {
                "kappa": kappa,
                "lattice": name,
                "mp_direct": mp.nstr(e, 30),
                "mp_direct_R": mp.nstr(r, 10),
                "mp_direct_terms": n,
                "mp_tail_bound": mp.nstr(tail, 5),
                "mp_ewald_eta0.7": mp.nstr(ew, 30),
                "direct_minus_ewald": mp.nstr(e - ew, 5),
                "rust_f64": rust[kappa][name],
                "abs_diff_rust": mp.nstr(d_rust, 5),
                "pos_pass_lt_1e-9": bool(d_rust < mp.mpf("1e-9")),
                "published_sf_rc8": sf[kappa][name],
                "exact_minus_published_sf": mp.nstr(d_sf, 8),
                "neg_detects_sf_gt_1e-9": bool(abs(d_sf) > mp.mpf("1e-9")),
                "abs_diff_textbook_scratch_R60": mp.nstr(abs(e - scratch[kappa][name]), 5),
                "seconds": round(time.time() - t0, 1),
            }
            pos_ok &= row["pos_pass_lt_1e-9"] and abs(e - ew) < mp.mpf("1e-25")
            neg_ok &= row["neg_detects_sf_gt_1e-9"]
            res["rows"].append(row)
            print(json.dumps(row))
    res["positive_control_pass"] = bool(pos_ok)
    res["negative_control_pass"] = bool(neg_ok)
    gaps = {}
    for kappa in (1, 2):
        rows = {r["lattice"]: r for r in res["rows"] if r["kappa"] == kappa}
        gaps[kappa] = {
            "gap_exact_mp": mp.nstr(mp.mpf(rows["square"]["mp_direct"]) - mp.mpf(rows["tri"]["mp_direct"]), 15),
            "gap_published_sf": sf[kappa]["square"] - sf[kappa]["tri"],
        }
    res["square_minus_tri"] = gaps
    print(json.dumps({"positive_control_pass": pos_ok, "negative_control_pass": neg_ok, "gaps": gaps}))
    json.dump(res, open(out, "w"), indent=1)


if __name__ == "__main__":
    main()
