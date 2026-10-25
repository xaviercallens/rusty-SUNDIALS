"""Independent numpy re-implementation (1D, since the kick is along x) of the linear-response
measurement in qf_gpe2d::linear_response, used as a two-language cross-check.

Usage: numpy_crosscheck.py X G DT_SAFETY EPS [N] [M]
Prints omega (zero crossings), fitted amplitude, S_amp and S_freq.
"""
from __future__ import annotations

import sys

import numpy as np


def run(x: float, g: float, safety: float, eps: float, n: int = 32, m: int = 2,
        periods: int = 6, spp: int = 200) -> dict[str, float]:
    ks = 2.0 * np.sqrt(g if g > 0 else 1.0)
    ks = 2.0  # reference k_s of the g=1 gas (as in the Rust example)
    k = x * ks
    e0 = 0.5 * k * k
    w_ref = np.sqrt(e0 * (e0 + 2.0 * g))
    L = 2.0 * np.pi * m / k
    dx = L / n
    xs = np.arange(n) * dx
    kk = 2.0 * np.pi * np.fft.fftfreq(n, d=dx)
    k2max = 2.0 * np.max(kk ** 2)  # 2D grid corner, to match Rust's max_dt on an n x n grid
    dt = min(2.0 * np.pi / (w_ref * spp), safety * 2.0 / k2max)
    nsteps = int(np.ceil(periods * 2.0 * np.pi / w_ref / dt))
    psi = np.ones(n, dtype=complex) * np.exp(1j * eps * np.cos(k * xs))
    mu = g
    kin = np.exp(-1j * dt * kk ** 2 / 2.0)
    ts, amps = [0.0], [2.0 * np.mean(np.abs(psi) ** 2 * np.cos(k * xs))]
    for s in range(1, nsteps + 1):
        psi = psi * np.exp(-1j * dt * (g * np.abs(psi) ** 2 - mu) / 2.0)
        psi = np.fft.ifft(np.fft.fft(psi) * kin)
        psi = psi * np.exp(-1j * dt * (g * np.abs(psi) ** 2 - mu) / 2.0)
        ts.append(s * dt)
        amps.append(2.0 * np.mean(np.abs(psi) ** 2 * np.cos(k * xs)))
    t = np.array(ts)
    a = np.array(amps)
    cr = []
    for i in range(1, len(a) - 1):
        if a[i] != 0.0 and a[i] * a[i + 1] < 0.0:
            cr.append(t[i] + a[i] / (a[i] - a[i + 1]) * (t[i + 1] - t[i]))
    j = np.arange(1, len(cr) + 1)
    slope = np.polyfit(j, np.array(cr), 1)[0]
    w = np.pi / slope
    X = np.c_[np.sin(w * t), np.cos(w * t)]
    c = np.linalg.lstsq(X, a, rcond=None)[0]
    amp = float(np.hypot(*c))
    return {"k": k, "dt": dt, "nsteps": nsteps, "omega": w, "omega_B": w_ref,
            "rel_err": (w - w_ref) / w_ref, "amp": amp, "max_abs": float(np.abs(a).max()),
            "S_amp": amp / (2.0 * eps), "S_freq": e0 / w, "S_B": e0 / w_ref}


if __name__ == "__main__":
    args = sys.argv[1:]
    r = run(float(args[0]), float(args[1]), float(args[2]), float(args[3]),
            *(int(v) for v in args[4:6]))
    for key, val in r.items():
        print(f"{key} {val!r}")
