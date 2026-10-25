"""List files (with sizes) and card of given HF dataset repos. Usage: python -I dipolar_hf_files.py <out.json> <repo> [<repo> ...]"""
from __future__ import annotations

import json
import sys

from huggingface_hub import HfApi

api = HfApi()
out = {}
for rid in sys.argv[2:]:
    info = api.dataset_info(rid, files_metadata=True)
    out[rid] = {"sha": info.sha, "license": (info.card_data.to_dict().get("license") if info.card_data else None),
                "files": [(s.rfilename, s.size) for s in (info.siblings or [])]}
    print(rid, info.sha, out[rid]["license"], len(out[rid]["files"]), "files, total",
          sum(s or 0 for _, s in out[rid]["files"]), "bytes")
    for f, s in out[rid]["files"][:12]:
        print("   ", f, s)
json.dump(out, open(sys.argv[1], "w"), indent=1)
