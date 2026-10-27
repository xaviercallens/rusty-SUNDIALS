"""Run the lattice_phonon scans for the QU-1 gate.

usage: python -I phonon_run_gate.py BIN TAUDIR OUTDIR GROUP
GROUP: cm (all completely monotone potentials) | neg (non-CM controls) | one POT_SPEC
For each potential: coarse (n=96), random (n=96), patch_rho and patch_i (n=48,96,192), rc = 400.
Cutoff convergence at the triangular point is run separately by the analysis script.
"""

import os
import subprocess
import sys
import time

binp, taudir, outdir, group = sys.argv[1:5]
os.makedirs(outdir, exist_ok=True)
CM = (
    [f"gauss:{a}" for a in (0.25, 0.5, 1, 2, 4)]
    + [f"yukawa:{k}" for k in (1, 2)]
    + [f"dipolar:{d}" for d in (0.1, 0.5)]
    + [f"riesz:{s}" for s in (3, 4, 6)]
)
NEG = [
    "wells:1.0,1.414213562373,0.12,0.5",
    "wells:1.0,1.414213562373,0.2,0.0",
    "lj:0.9",
    "morse:3,1.0",
    "gdiff:0.3,1.5,0.5",
]
pots = CM if group == "cm" else NEG if group == "neg" else [group]
RC = "400"
for spec in pots:
    tag = spec.replace(":", "_").replace(",", "_")
    for name, ns in (("coarse", "96"), ("random", "96"), ("patch_rho", "48,96,192"), ("patch_i", "48,96,192")):
        dst = os.path.join(outdir, f"{tag}__{name}.csv")
        if os.path.exists(dst):
            continue
        t0 = time.time()
        subprocess.run(
            ["taskset", "-c", os.environ.get("LP_CORES", "4-5"), binp, "scan", spec, ns, RC, os.path.join(taudir, f"{name}.txt"), dst],
            check=True,
        )
        print(f"{spec} {name} {time.time() - t0:.1f}s", flush=True)
