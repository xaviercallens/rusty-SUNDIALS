"""Apply the dual-scale diffraction test to superfluid 4He with Tier L literature data.

Data (NOT from HuggingFace; no S(k) dataset exists on the Hub, see hf_search.json):
  Godfrin, Beauvois, Sultan, Krotscheck, Dawidowski, Fak, Ollivier, "The dispersion relation of
  Landau elementary excitations and the thermodynamic properties of superfluid 4He",
  arXiv:2012.09067v1 (2020), Table VI: eps(k) at saturated vapour pressure, T < 100 mK.
  Below k = 0.15 1/A the table is ultrasound data, 0.15-0.3 combined, above 0.3 neutron data.
  Same paper, Sec. II A: c = 238.3 +- 0.1 m/s (ultrasound, SVP), m4 = 4.0026032 g/mol,
  number density 0.021836 atoms/A^3.
  Cross-check: Beauvois et al., arXiv:1802.08120v3, Table I: Delta_R = 0.7416(10) meV,
  Q_R = 1.9260(2) 1/A at P = 0; Godfrin et al. Table V: Delta_M = 1.191(1) meV, k_M = 1.103 1/A.

What the data can decide. Table VI gives eps(k), not S(k). Feynman's variational inequality
(the lowest excitation energy is at most the Bijl-Feynman energy eps0/S) gives the UPPER bound
S(k) <= S_up(k) = eps0(k)/eps(k). So wherever S_up(k) < sqrt(k/(2 k_s)), the diffraction test is
provably SILENT for 4He at that k (given Feynman's inequality); where S_up >= the threshold the
dispersion cannot decide, and a measured S(k) would be needed. The data can never show the test
FIRES. Separately, eps(k) is compared with the Bogoliubov eps_B(k) built from the same c.

Controls (same code path): Bogoliubov S_B = eps0/eps_B must never fire; the free value S = 1
must fire for every k < 2 k_s.

Usage: helium_tier_l.py OUT.json
"""
from __future__ import annotations

import json
import math
import sys

import numpy as np
import scipy
from scipy import constants as C

# (k [1/A], eps [meV], err [meV] or None) transcribed from arXiv:2012.09067v1 Table VI.
TABLE_VI: list[tuple[float, float, float | None]] = [
    (0.00, 0.0000, None), (0.05, 0.0787, None), (0.10, 0.1587, None),
    (0.15, 0.2407, 0.0014), (0.20, 0.3244, 0.0012), (0.25, 0.4092, 0.0010),
    (0.30, 0.4938, 0.0010), (0.35, 0.5782, 0.0011), (0.40, 0.6581, 0.0012),
    (0.45, 0.7338, 0.0013), (0.50, 0.8040, 0.0014), (0.55, 0.8701, 0.0015),
    (0.60, 0.9279, 0.0016), (0.65, 0.9812, 0.0017), (0.70, 1.0267, 0.0018),
    (0.75, 1.0667, 0.0018), (0.80, 1.1002, 0.0019), (0.85, 1.1280, 0.0019),
    (0.90, 1.1518, 0.0020), (0.95, 1.1678, 0.0020), (1.00, 1.1809, 0.0020),
    (1.05, 1.1882, 0.0020), (1.10, 1.1907, 0.0020), (1.15, 1.1884, 0.0020),
    (1.20, 1.1814, 0.0020), (1.25, 1.1698, 0.0020), (1.30, 1.1542, 0.0020),
    (1.35, 1.1310, 0.0019), (1.40, 1.1055, 0.0019), (1.45, 1.0743, 0.0018),
    (1.50, 1.0381, 0.0018), (1.55, 0.9999, 0.0017), (1.60, 0.9563, 0.0016),
    (1.65, 0.9113, 0.0015), (1.70, 0.8672, 0.0015), (1.75, 0.8242, 0.0014),
    (1.80, 0.7877, 0.0013), (1.85, 0.7584, 0.0013), (1.90, 0.7427, 0.0013),
    (1.95, 0.7459, 0.0013), (2.00, 0.7681, 0.0013), (2.05, 0.8094, 0.0014),
    (2.10, 0.8637, 0.0015), (2.15, 0.9374, 0.0016), (2.20, 1.0154, 0.0017),
    (2.25, 1.0944, 0.0019), (2.30, 1.1701, 0.0020), (2.35, 1.2370, 0.0023),
    (2.40, 1.2914, 0.0025), (2.45, 1.3363, 0.0030), (2.50, 1.3718, 0.0035),
    (2.55, 1.3985, 0.0039), (2.60, 1.4214, 0.0044), (2.65, 1.4375, 0.0049),
    (2.70, 1.4571, 0.0053), (2.75, 1.4617, 0.0058), (2.80, 1.4746, 0.0062),
    (2.85, 1.4776, 0.0066), (2.90, 1.4737, 0.0071), (2.95, 1.4818, 0.0075),
    (3.00, 1.4784, 0.0080), (3.05, 1.4793, 0.0085), (3.10, 1.4781, 0.0089),
    (3.15, 1.4767, 0.0094), (3.20, 1.4805, 0.0098), (3.25, 1.4920, 0.010),
    (3.30, 1.4855, 0.011), (3.35, 1.4954, 0.011), (3.40, 1.4940, 0.012),
    (3.45, 1.5012, 0.012), (3.50, 1.5199, 0.012), (3.55, 1.5377, 0.013),
    (3.60, 1.5538, 0.013),
]
C_SOUND = 238.3  # m/s
C_SOUND_ERR = 0.1
M4_G_PER_MOL = 4.0026032


def fires(s: np.ndarray, k: np.ndarray, ks: float) -> np.ndarray:
    """The diffraction test: S(k) > sqrt(k / (2 k_s))."""
    return s > np.sqrt(k / (2.0 * ks))


def main(out: str) -> None:
    k = np.array([r[0] for r in TABLE_VI])
    eps = np.array([r[1] for r in TABLE_VI])
    err = np.array([r[2] if r[2] is not None else 0.0 for r in TABLE_VI])

    # Transcription controls.
    assert len(TABLE_VI) == 73
    assert np.all(np.diff(k) > 0) and np.allclose(np.diff(k), 0.05)
    i_min = int(np.argmin(np.where(k > 1.5, eps, np.inf)))
    i_max = int(np.argmax(np.where(k < 1.5, eps, -np.inf)))
    transcription = {
        "n_rows": len(TABLE_VI),
        "roton_min_table": [k[i_min], eps[i_min]],
        "maxon_max_table": [k[i_max], eps[i_max]],
        "roton_ref_1802.08120_TableI": [1.9260, 0.7416],
        "maxon_ref_2012.09067_TableV": [1.103, 1.191],
    }
    assert abs(k[i_min] - 1.926) <= 0.05 and abs(eps[i_min] - 0.7416) < 0.005
    assert abs(k[i_max] - 1.103) <= 0.05 and abs(eps[i_max] - 1.191) < 0.005

    m = M4_G_PER_MOL * 1e-3 / C.N_A  # kg
    hbar = C.hbar
    mev = 1e-3 * C.e
    ks = 2.0 * m * C_SOUND / hbar * 1e-10  # 1/A
    hb2_2m = hbar ** 2 / (2.0 * m) / mev * 1e20  # meV A^2
    hbar_c = hbar * C_SOUND / mev * 1e10  # meV A
    assert 2.9 < ks < 3.1, ks
    # Phonon-limit control: eps(0.05) ~ hbar c k.
    phonon_check = {"eps_table_0.05": eps[1], "hbar_c_k_0.05": hbar_c * 0.05,
                    "rel_diff": (eps[1] - hbar_c * 0.05) / (hbar_c * 0.05)}
    assert abs(phonon_check["rel_diff"]) < 0.01

    kk, ee, er = k[1:], eps[1:], err[1:]
    eps0 = hb2_2m * kk ** 2
    thr = np.sqrt(kk / (2.0 * ks))
    s_up = eps0 / ee
    s_up_cons = eps0 / np.maximum(ee - er, 1e-12)  # conservative (largest) upper bound
    silent = s_up_cons < thr
    # Stricter variant: in the anomalous-dispersion range (eps > hbar c k) multi-phonon states may
    # lie near hbar c k, below the measured branch; take omega_min = min(eps - err, hbar c k).
    omega_min = np.minimum(ee - er, hbar_c * kk)
    s_up_strict = eps0 / omega_min
    silent_strict = s_up_strict < thr
    eps_b = np.sqrt((hbar_c * kk) ** 2 + eps0 ** 2)
    s_b = eps0 / eps_b

    # Controls through the same test function.
    ctl_bog_fires = fires(s_b, kk, ks)
    ctl_free_fires = fires(np.ones_like(kk), kk, ks)
    assert not ctl_bog_fires.any()
    assert ctl_free_fires[kk < 2 * ks].all()

    i_r = int(np.argmin(np.abs(kk - 1.926)))
    silent_ks = kk[silent]
    first_undecided = float(kk[~silent][0]) if (~silent).any() else None
    res = {
        "source": {
            "dispersion": "arXiv:2012.09067v1 Table VI (Godfrin et al. 2020), SVP, T<100 mK",
            "c": f"{C_SOUND} +- {C_SOUND_ERR} m/s (arXiv:2012.09067v1 Sec. II A, ultrasound)",
            "m4": f"{M4_G_PER_MOL} g/mol (same paper)",
            "roton_crosscheck": "arXiv:1802.08120v3 Table I",
            "huggingface": "none: no structure-factor dataset found on the Hub (hf_search.json)",
            "constants": f"scipy {scipy.__version__} CODATA (hbar, N_A, e)",
        },
        "transcription_controls": transcription,
        "phonon_check": phonon_check,
        "k_s_inv_A": ks,
        "hbar2_over_2m_meV_A2": hb2_2m,
        "threshold_at_roton": {"k": float(kk[i_r]), "sqrt_k_over_2ks": float(thr[i_r]),
                               "S_up_feynman": float(s_up[i_r]),
                               "eps_over_epsB": float(ee[i_r] / eps_b[i_r])},
        "threshold_at_Q_R_1.9260": math.sqrt(1.9260 / (2.0 * ks)),
        "threshold_at_Q_R_c_minus_plus_err": [
            math.sqrt(1.9260 / (2.0 * ks * (C_SOUND + s * C_SOUND_ERR) / C_SOUND)) for s in (-1, 1)],
        "silent_k_range_inv_A": [float(silent_ks.min()), float(silent_ks.max())] if silent.any() else None,
        "silent_all_k_up_to": float(kk[np.argmin(silent)] - 0.05) if (~silent).any() else float(kk[-1]),
        "first_undecided_k": first_undecided,
        "silent_strict_k": [float(x) for x in kk[silent_strict]],
        "silent_set_unchanged_under_strict_variant": bool(np.array_equal(silent, silent_strict)),
        "margin_at_k_0.5": {"S_up_strict": float(s_up_strict[9]), "threshold": float(thr[9]), "k": float(kk[9])},
        "n_silent": int(silent.sum()), "n_undecided": int((~silent).sum()),
        "controls": {"bogoliubov_fires_anywhere": bool(ctl_bog_fires.any()),
                     "free_fires_for_all_k_lt_2ks": bool(ctl_free_fires[kk < 2 * ks].all())},
        "rows": [{"k": float(a), "eps": float(b), "err": float(c_), "eps0": float(d),
                  "S_up": float(e_), "S_up_conservative": float(f), "threshold": float(g),
                  "silent": bool(h), "eps_B": float(i_), "S_B": float(j), "eps_over_epsB": float(b / i_)}
                 for a, b, c_, d, e_, f, g, h, i_, j in
                 zip(kk, ee, er, eps0, s_up, s_up_cons, thr, silent, eps_b, s_b)],
    }
    json.dump(res, open(out, "w"), indent=1)
    print("k_s =", ks, "1/A;  hbar^2/2m =", hb2_2m, "meV A^2;  hbar c =", hbar_c, "meV A")
    print("transcription", transcription)
    print("phonon check", phonon_check)
    print("roton", res["threshold_at_roton"])
    print("silent k (S_up_cons < threshold):", [float(x) for x in silent_ks])
    print("first undecided k:", first_undecided)
    print("strict variant silent set unchanged:", bool(np.array_equal(silent, silent_strict)), res["margin_at_k_0.5"])
    print("controls", res["controls"])
    print("eps/eps_B min:", float((ee / eps_b).min()), "at k =", float(kk[np.argmin(ee / eps_b)]))


if __name__ == "__main__":
    main(sys.argv[1])
