"""Independent (Python/mpmath, 40 digits) reference values for the Rust Keldysh module.

V(r) = (pi/(2 r0)) [H0(r/r0) - Y0(r/r0)] (closed form, Struve/Neumann); erfcx(x) = e^{x^2} erfc(x).
Usage: mpmath_refs.py OUT_JSON
"""

import json
import sys

import mpmath as mp

mp.mp.dps = 40


def v_closed(r: mp.mpf, r0: mp.mpf) -> mp.mpf:
    x = r / r0
    return mp.pi / (2 * r0) * (mp.struveh(0, x) - mp.bessely(0, x))


def main() -> None:
    out = {"erfcx": [], "V": []}
    for x in ["0", "0.5", "3", "25.5", "26.5", "1000", "1e8"]:
        xm = mp.mpf(x)
        out["erfcx"].append([float(xm), float(mp.exp(xm * xm) * mp.erfc(xm))])
    for r0 in ["0.05", "0.3", "1", "4"]:
        for r in ["0.1", "0.5", "1", "1.0745699318", "2", "5", "12"]:
            out["V"].append([float(mp.mpf(r0)), float(mp.mpf(r)), float(v_closed(mp.mpf(r), mp.mpf(r0)))])
    json.dump(out, open(sys.argv[1], "w"), indent=1)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
