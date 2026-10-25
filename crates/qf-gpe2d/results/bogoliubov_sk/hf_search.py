"""Search the HuggingFace Hub for public structure-factor datasets (liquid helium / BEC).

Records every query and every hit (id, sha, license tag, downloads) to hf_search.json.
No token is used; only public metadata is read.
"""
from __future__ import annotations

import json
import sys

from huggingface_hub import HfApi

QUERIES: list[str] = [
    "structure factor",
    "static structure factor",
    "dynamic structure factor",
    "liquid helium",
    "helium-4",
    "superfluid helium",
    "neutron scattering",
    "neutron diffraction",
    "x-ray diffraction liquid",
    "Bose-Einstein condensate",
    "BEC",
    "Bragg spectroscopy",
    "cold atoms",
    "ultracold",
    "absorption imaging",
    "quantum gas",
    "superfluid",
    "Bogoliubov",
    "pair distribution function",
    "radial distribution function",
    "helium",
    "condensate",
    "phonon",
    "scattering",
    "neutron",
    "quantum-gas",
    "cold-atom",
    "ultracold-atoms",
    "liquid-structure",
    "SANS",
]


def main(out_path: str) -> None:
    api = HfApi()
    record: dict[str, list[dict[str, object]]] = {}
    for q in QUERIES:
        hits: list[dict[str, object]] = []
        for d in api.list_datasets(search=q, limit=60, full=True):
            lic = [t for t in (d.tags or []) if t.startswith("license:")]
            hits.append(
                {
                    "id": d.id,
                    "sha": d.sha,
                    "license": lic,
                    "downloads": d.downloads,
                    "last_modified": str(d.last_modified),
                    "tags": [t for t in (d.tags or []) if not t.startswith("license:")][:12],
                }
            )
        record[q] = hits
        print(f"{q!r}: {len(hits)} hits")
        for h in hits:
            print("   ", h["id"], h["license"])
    with open(out_path, "w") as fh:
        json.dump(record, fh, indent=1)


if __name__ == "__main__":
    main(sys.argv[1])
