"""List files / card of candidate HF datasets, and colabfit JARVIS datasets. Usage: python -I dipolar_hf_inspect.py <out.json>"""
from __future__ import annotations

import json
import sys

from huggingface_hub import HfApi

api = HfApi()
out: dict = {"colabfit_jarvis": [], "candidates": {}}
for d in api.list_datasets(author="colabfit", search="JARVIS", limit=100):
    out["colabfit_jarvis"].append(d.id)
for d in api.list_datasets(author="colabfit", search="2D", limit=100):
    out["colabfit_jarvis"].append(d.id)
for d in api.list_datasets(search="bilayer", limit=100):
    out["colabfit_jarvis"].append("bilayer-search:" + d.id)
for rid in ["colabfit/JARVIS_C2DB", "HZPM/C2DBench", "colabfit/JARVIS_DFT_3D", "colabfit/JARVIS-DFT-3D",
            "colabfit/JARVIS_DFT_2D"]:
    try:
        info = api.dataset_info(rid, files_metadata=True)
        out["candidates"][rid] = {
            "sha": info.sha,
            "card": (info.card_data.to_dict() if info.card_data else None),
            "files": [(s.rfilename, s.size) for s in (info.siblings or [])],
        }
    except Exception as exc:
        out["candidates"][rid] = {"error": repr(exc)}
json.dump(out, open(sys.argv[1], "w"), indent=1, default=str)
print(json.dumps(out, indent=1, default=str)[:6000])
