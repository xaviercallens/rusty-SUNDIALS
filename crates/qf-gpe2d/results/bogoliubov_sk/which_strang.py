"""Decide which nonlinear half-step the pinned Python reference used.

The pinned 4x4 data in crates/qf-gpe2d/tests/python_cross_check.rs (PSI0, AFTER_ONE_STEP;
dx = 0.3, dt = 0.05, g = mu = 1) came from `quantumfluids.gpe.solver2d.step`. Two candidates:

* "reuse": the second nonlinear half-step reuses the phase factor computed from |psi|^2 at the
  start of the step (what qf-gpe2d's step_inplace does);
* "strang": the second half-step recomputes |psi|^2 after the kinetic step (textbook Strang).

Prints the max deviation of each candidate from AFTER_ONE_STEP.
"""
from __future__ import annotations

import re
import sys

import numpy as np


def parse(src: str, name: str) -> np.ndarray:
    block = src.split(f"const {name}")[1].split("];")[0]
    pairs = re.findall(r"\(\s*([-0-9.e]+),\s*([-0-9.e]+)\)", block)
    return np.array([complex(float(a), float(b)) for a, b in pairs]).reshape(4, 4)


def main(path: str) -> None:
    src = open(path).read()
    psi0 = parse(src, "PSI0")
    after = parse(src, "AFTER_ONE_STEP")
    n, dx, dt, g, mu = 4, 0.3, 0.05, 1.0, 1.0
    k = 2 * np.pi * np.fft.fftfreq(n, d=dx)
    k2 = k[:, None] ** 2 + k[None, :] ** 2
    kin = np.exp(-1j * dt * k2 / 2)

    half0 = np.exp(-1j * dt * (g * np.abs(psi0) ** 2 - mu) / 2)
    p = np.fft.ifft2(np.fft.fft2(psi0 * half0) * kin)
    reuse = p * half0
    strang = p * np.exp(-1j * dt * (g * np.abs(p) ** 2 - mu) / 2)
    print("max |reuse  - AFTER_ONE_STEP| =", np.abs(reuse - after).max())
    print("max |strang - AFTER_ONE_STEP| =", np.abs(strang - after).max())


if __name__ == "__main__":
    main(sys.argv[1])
