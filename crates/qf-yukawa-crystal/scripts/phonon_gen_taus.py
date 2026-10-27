"""Generate the tau (lattice-shape) point sets for the lattice-phonon scans (QU-1 gate).

usage: python -I phonon_gen_taus.py OUTDIR
writes coarse.txt (fundamental-domain grid), patch_rho.txt, patch_i.txt (rings around the triangular and square
points, any tau is a valid lattice, points outside the fundamental domain are modular images) and random.txt
(400 uniform-random points of the fundamental domain, seed 20261025).
"""

import math
import os
import random
import sys

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
RHO = (0.5, math.sqrt(3.0) / 2.0)
YMAX = 2.4


def write(name, pts):
    with open(os.path.join(out, name), "w") as f:
        for x, y in pts:
            f.write(f"{x:.12f} {y:.12f}\n")
    print(name, len(pts))


coarse = []
for ix in range(0, 11):
    x = 0.05 * ix
    y0 = math.sqrt(max(0.0, 1.0 - x * x))
    y = y0
    while y <= YMAX + 1e-9:
        coarse.append((x, y))
        y += 0.1
write("coarse.txt", coarse)

ring = []
for r in (0.01, 0.02, 0.05, 0.1, 0.2):
    for k in range(12):
        a = 2 * math.pi * k / 12
        ring.append((r, a))
write("patch_rho.txt", [(RHO[0] + r * math.cos(a), RHO[1] + r * math.sin(a)) for r, a in ring] + [RHO])
write("patch_i.txt", [(r * math.cos(a), 1.0 + r * math.sin(a)) for r, a in ring] + [(0.0, 1.0)])

rng = random.Random(20261025)
pts = []
while len(pts) < 400:
    x = rng.uniform(0.0, 0.5)
    y = rng.uniform(0.85, YMAX)
    if x * x + y * y >= 1.0:
        pts.append((x, y))
write("random.txt", pts)
