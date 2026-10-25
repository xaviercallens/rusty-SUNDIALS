"""Extract TMD (MX2, M in {Mo, W}, X in {S, Se}) structures from colabfit JARVIS parquet shards (untrusted data).

For each matching configuration, report the lattice, atom positions, and for layered 2H/1H structures:
  - metal-plane spacing along the stacking axis (bulk: interlayer spacing between adjacent metal planes),
  - the in-plane lattice constant a,
  - intra-layer chalcogen-chalcogen thickness t_XX (vertical X-X distance within one layer).
Usage: python -I dipolar_tmd_extract.py <out.json> <shard.parquet> [<shard.parquet> ...]
"""
from __future__ import annotations

import json
import sys

import numpy as np
import pyarrow.parquet as pq

Z = {"Mo": 42, "W": 74, "S": 16, "Se": 34}
TARGET = {(42, 16): "MoS2", (42, 34): "MoSe2", (74, 16): "WS2", (74, 34): "WSe2"}


def classify(nums: list[int]) -> str | None:
    s = set(nums)
    if len(s) != 2:
        return None
    m = [z for z in s if z in (42, 74)]
    x = [z for z in s if z in (16, 34)]
    if len(m) != 1 or len(x) != 1:
        return None
    if nums.count(x[0]) != 2 * nums.count(m[0]):
        return None
    return TARGET[(m[0], x[0])]


def geometry(cell: np.ndarray, pos: np.ndarray, nums: list[int]) -> dict:
    # stacking axis = cell vector most orthogonal to the other two (largest |c . (a x b)|/|c| ...); use the
    # cell vector with the largest angle to the plane of the other two, i.e. the normal component.
    best = None
    for k in range(3):
        i, j = [q for q in range(3) if q != k]
        nrm = np.cross(cell[i], cell[j])
        nrm /= np.linalg.norm(nrm)
        comp = abs(cell[k] @ nrm)
        a_in = 0.5 * (np.linalg.norm(cell[i]) + np.linalg.norm(cell[j]))
        if best is None or comp / a_in > best[0]:
            best = (comp / a_in, k, nrm, comp, np.linalg.norm(cell[i]), np.linalg.norm(cell[j]))
    _, k, nrm, period, la, lb = best
    z = (pos @ nrm) % period
    nums = np.asarray(nums)
    zm = np.sort(z[nums >= 42])
    zx = np.sort(z[nums < 42])
    gaps_m = np.diff(np.concatenate([zm, [zm[0] + period]]))
    out = {"stacking_period": float(period), "a_in_plane": [float(la), float(lb)],
           "metal_planes_z": zm.tolist(), "metal_plane_spacings": gaps_m.tolist()}
    # chalcogen pairs bracketing each metal plane: nearest X below and above
    thick = []
    for zz in zm:
        dz = (zx - zz + period / 2) % period - period / 2
        above = dz[dz > 0].min() if (dz > 0).any() else np.nan
        below = -dz[dz < 0].max() if (dz < 0).any() else np.nan
        thick.append(float(above + below))
    out["t_XX"] = thick
    return out


def main(out_path: str, shards: list[str]) -> None:
    res = []
    for sh in shards:
        pf = pq.ParquetFile(sh)
        cols = pf.schema_arrow.names
        print(sh, "columns:", cols)
        for batch in pf.iter_batches(batch_size=4096):
            tab = batch.to_pydict()
            for r in range(len(tab["atomic_numbers"])):
                nums = list(tab["atomic_numbers"][r])
                name = classify(nums)
                if name is None:
                    continue
                cell = np.asarray(tab["cell"][r], dtype=float).reshape(3, 3)
                pos = np.asarray(tab["positions"][r], dtype=float).reshape(-1, 3)
                rec = {"shard": sh, "formula": name, "natoms": len(nums)}
                for key in ("id", "names", "configuration_id", "dataset_id", "energy", "electronic_band_gap",
                            "formation_energy", "metadata", "chemical_formula_hill", "pbc"):
                    if key in tab:
                        v = tab[key][r]
                        rec[key] = v if not isinstance(v, (bytes,)) else v.decode()
                try:
                    rec.update(geometry(cell, pos, nums))
                except Exception as exc:
                    rec["geometry_error"] = repr(exc)
                res.append(rec)
    json.dump(res, open(out_path, "w"), indent=1, default=str)
    for rec in res:
        print(rec["formula"], rec["natoms"], rec.get("names"), "period", round(rec.get("stacking_period", 0), 4),
              "a", [round(v, 4) for v in rec.get("a_in_plane", [])],
              "dM", [round(v, 4) for v in rec.get("metal_plane_spacings", [])],
              "tXX", [round(v, 4) for v in rec.get("t_XX", [])], "gap", rec.get("electronic_band_gap"))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])
