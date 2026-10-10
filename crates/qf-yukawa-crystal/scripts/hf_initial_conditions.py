"""Disordered 2D initial conditions from the Hugging Face dataset colabfit/OC20_IS2RES_val_id (CC-BY-4.0).

For each initial condition: draw OC20 structures at random from one parquet shard, rotate each uniformly at random
in 3D, project onto the plane, rescale so the median projected nearest-neighbour distance is 0.9 a, place the patch
at a random offset, and accept points that keep a minimum separation of 0.5 a from points already accepted, until
N points exist. Output: CSV (x, y) per initial condition plus a provenance JSON (row indices, configuration ids).

Usage: python hf_initial_conditions.py <shard.parquet> <out_dir>
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import numpy as np
import pyarrow.parquet as pq

A = math.sqrt(2.0 / math.sqrt(3.0))  # triangular lattice spacing at density 1
DMIN = 0.5 * A


def random_rotation(rng: np.random.Generator) -> np.ndarray:
    q, r = np.linalg.qr(rng.normal(size=(3, 3)))
    q = q * np.sign(np.diag(r))
    if np.linalg.det(q) < 0:
        q[:, 0] = -q[:, 0]
    return q


def patch(pos: np.ndarray, rng: np.random.Generator) -> np.ndarray:
    p = (pos - pos.mean(axis=0)) @ random_rotation(rng).T
    xy = p[:, :2]
    d = np.sqrt(((xy[:, None, :] - xy[None, :, :]) ** 2).sum(-1))
    np.fill_diagonal(d, np.inf)
    nn = np.median(d.min(axis=1))
    return xy * (0.9 * A / nn) if nn > 0 else xy


def fill(kind: str, n: int, rng: np.random.Generator, table, box: tuple[float, float] | None, radius: float | None):
    pts: list[np.ndarray] = []
    used: list[dict] = []
    rows = table.num_rows
    while len(pts) < n:
        i = int(rng.integers(rows))
        rec = table.slice(i, 1).to_pylist()[0]
        pos = np.asarray(rec["positions"], dtype=float)
        if len(pos) < 3:
            continue
        xy = patch(pos, rng)
        if box is not None:
            off = rng.uniform([0, 0], box)
        else:
            rr, th = radius * math.sqrt(rng.uniform()), rng.uniform(0, 2 * math.pi)
            off = np.array([rr * math.cos(th), rr * math.sin(th)])
        added = 0
        for q in xy + off:
            if box is not None:
                q = np.mod(q, box)
            elif q @ q > radius ** 2:
                continue
            ok = True
            for p in pts:
                dv = q - p
                if box is not None:
                    dv = dv - np.asarray(box) * np.round(dv / np.asarray(box))
                if dv @ dv < DMIN ** 2:
                    ok = False
                    break
            if ok:
                pts.append(q)
                added += 1
                if len(pts) == n:
                    break
        used.append({"row": i, "configuration_id": rec["configuration_id"], "n_atoms": len(pos), "accepted": added})
    return np.array(pts), used


def main() -> None:
    shard, out = Path(sys.argv[1]), Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    table = pq.read_table(shard, columns=["positions", "configuration_id"])
    lx, ly = 16 * A, 9 * math.sqrt(3) * A
    lsq = math.sqrt(289.0)
    rdisk = math.sqrt(300 / math.pi)
    prov = {"dataset": "colabfit/OC20_IS2RES_val_id", "license": "CC-BY-4.0", "doi": "10.60732/b4005525",
            "shard": shard.name, "rows_in_shard": table.num_rows, "dmin": DMIN, "items": []}
    specs = [("tri_box", 288, (lx, ly), None, 20), ("square_box", 289, (lsq, lsq), None, 10), ("disk", 300, None, rdisk, 10)]
    for kind, n, box, radius, count in specs:
        for k in range(count):
            rng = np.random.default_rng(1000 * len(kind) + k)
            pts, used = fill(kind, n, rng, table, box, radius)
            np.savetxt(out / f"oc20_{kind}_{k:02d}.csv", pts, delimiter=",", fmt="%.12f")
            prov["items"].append({"file": f"oc20_{kind}_{k:02d}.csv", "kind": kind, "n": n, "seed": 1000 * len(kind) + k,
                                  "structures_used": len(used), "first_structures": used[:5]})
            print(kind, k, "points", len(pts), "structures", len(used), flush=True)
    (out / "provenance.json").write_text(json.dumps(prov, indent=1))


if __name__ == "__main__":
    main()
