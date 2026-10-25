"""Dyadic shell energies from The Well turbulent_radiative_layer_2D velocity (one pinned file).

Run with an interpreter that has h5py (the AutoevolveAI .venv does not):
  /mnt/disks/disk-socrateai-local-1/venv-pta/bin/python -I well_spectrum.py HDF5 OUT_JSON

Method (velocity only, not sqrt(rho) u):
  * x (128 points) is PERIODIC, y (384) is OPEN, so the FFT is taken along x only.
  * u_hat(kx) = FFT_x(u)/Nx for each row y and each component; kx = 0 (the x-mean, which carries the mean
    shear flow) is removed.
  * one-sided spectrum e(kx) = sum over +-kx of 1/2 |u_hat|^2, averaged over y and summed over the two
    components, so that sum_{kx>=1} e(kx) = 1/2 <|u - <u>_x|^2>  (Parseval; checked).
  * shell n collects kx in [2^n, 2^{n+1}), n = 0..5 (kx = 64 is the Nyquist mode, kept separately and
    reported, not used). E_n = sum e(kx), |v_n| = sqrt(2 E_n).
Controls on the same pipeline: Parseval identity on the data; a synthetic sin(2 pi 8 x / Lx) field must put
all energy in shell 3; white noise gives E_n proportional to the number of modes per shell (2^n).
"""
import json
import sys

import h5py
import numpy as np

NSHELL = 6  # shells n = 0..5 fully supported by Nx = 128


def shell_energies(u: np.ndarray) -> tuple[np.ndarray, float, float, float]:
    """u: (Nx, Ny, 2) velocity. Returns (E_n for n<NSHELL, nyquist energy, parseval lhs, parseval rhs)."""
    nx = u.shape[0]
    uh = np.fft.fft(u, axis=0) / nx  # (Nx, Ny, 2)
    p = 0.5 * np.abs(uh) ** 2  # per signed mode
    p = p.mean(axis=1).sum(axis=-1)  # average over y, sum components -> (Nx,)
    e = np.zeros(nx // 2 + 1)
    e[0] = p[0]
    for kx in range(1, nx // 2):
        e[kx] = p[kx] + p[nx - kx]
    e[nx // 2] = p[nx // 2]
    en = np.array([e[2 ** n: 2 ** (n + 1)].sum() for n in range(NSHELL)])
    fluct = u - u.mean(axis=0, keepdims=True)
    parseval_rhs = 0.5 * (fluct ** 2).sum(axis=-1).mean()
    parseval_lhs = e[1:].sum()
    return en, float(e[nx // 2]), float(parseval_lhs), float(parseval_rhs)


def main() -> None:
    path, out = sys.argv[1], sys.argv[2]
    rep: dict = {"source_file": path, "method": __doc__, "snapshots": []}
    with h5py.File(path, "r") as f:
        time = f["dimensions/time"][:]
        x = f["dimensions/x"][:]
        vel = f["t1_fields/velocity"]
        rep["tcool"] = float(f["scalars/tcool"][()])
        rep["shape"] = list(vel.shape)
        rep["x_first_last"] = [float(x[0]), float(x[-1])]
        for ti in (25, 50, 75, 100):
            u = np.asarray(vel[0, ti], dtype=np.float64)  # (128, 384, 2)
            en, nyq, pl, pr = shell_energies(u)
            rep["snapshots"].append({"time_index": ti, "time": float(time[ti]), "E_n": en.tolist(),
                                     "amp_n": np.sqrt(2 * en).tolist(), "nyquist_energy": nyq,
                                     "parseval_lhs": pl, "parseval_rhs": pr,
                                     "parseval_rel_err": abs(pl - pr) / pr,
                                     "E_shells_over_E_fluct": float(en.sum() / pr)})
            print("t", ti, "E_n", en, "parseval", pl, pr, flush=True)
    # controls
    nx, ny = 128, 384
    xs = np.arange(nx) / nx
    syn = np.zeros((nx, ny, 2))
    syn[:, :, 0] = np.sin(2 * np.pi * 8 * xs)[:, None]
    en_s, _, pl_s, pr_s = shell_energies(syn)
    rng = np.random.default_rng(20261025)
    wn = rng.normal(size=(nx, ny, 2))
    en_w, _, pl_w, pr_w = shell_energies(wn)
    rep["control_sin_k8"] = {"E_n": en_s.tolist(), "fraction_in_shell3": float(en_s[3] / en_s.sum()),
                             "parseval_lhs": pl_s, "parseval_rhs": pr_s}
    rep["control_white_noise"] = {"E_n": en_w.tolist(),
                                  "E_n_over_modes": (en_w / 2.0 ** np.arange(NSHELL)).tolist()}
    data_slope = [float(np.polyfit(np.arange(NSHELL), np.log2(s["E_n"]), 1)[0]) for s in rep["snapshots"]]
    noise_slope = float(np.polyfit(np.arange(NSHELL), np.log2(en_w), 1)[0])
    rep["log2_slope_E_n_data"] = data_slope
    rep["log2_slope_E_n_white_noise"] = noise_slope
    ok = (all(s["parseval_rel_err"] < 1e-10 for s in rep["snapshots"])
          and rep["control_sin_k8"]["fraction_in_shell3"] > 1 - 1e-12
          and abs(noise_slope - 1.0) < 0.1 and all(sl < 0 for sl in data_slope))
    rep["controls_pass"] = bool(ok)
    print("slopes data", data_slope, "noise", noise_slope, "controls_pass", ok)
    with open(out, "w") as fh:
        json.dump(rep, fh, indent=1)


if __name__ == "__main__":
    main()
