"""Search the Hugging Face Hub for dusty-plasma / colloid datasets and record OC20 revision metadata.

Usage: python -I hf_search.py OUT_JSON
Public metadata only (no token, no file downloads).
"""

import json
import sys

from huggingface_hub import HfApi


def main() -> None:
    out = sys.argv[1]
    api = HfApi()
    hits = {}
    for term in ["dusty plasma", "dusty-plasma", "complex plasma", "colloid", "colloidal", "yukawa", "wigner crystal"]:
        rows = []
        for d in api.list_datasets(search=term, limit=50):
            rows.append({"id": d.id, "downloads": getattr(d, "downloads", None), "last_modified": str(getattr(d, "last_modified", ""))})
        hits[term] = rows
    info = api.dataset_info("colabfit/OC20_IS2RES_val_id")
    oc20 = {
        "id": info.id,
        "sha": info.sha,
        "last_modified": str(info.last_modified),
        "license": (info.card_data or {}).get("license") if info.card_data else None,
        "n_files": len(info.siblings or []),
    }
    json.dump({"search": hits, "oc20": oc20}, open(out, "w"), indent=1)
    for k, v in hits.items():
        print(k, [r["id"] for r in v])
    print("oc20", oc20)


if __name__ == "__main__":
    main()
