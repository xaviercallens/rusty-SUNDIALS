"""Search the Hugging Face Hub for datasets that could give interlayer separations of TMD heterobilayers.

Writes the raw search hits (id, sha, tags, downloads) to a JSON file. Read-only; no token needed.
Usage: python -I dipolar_hf_search.py <out.json>
"""
from __future__ import annotations

import json
import sys

from huggingface_hub import HfApi

QUERIES = [
    "C2DB", "c2db", "BiDB", "bilayer", "heterobilayer", "JARVIS", "jarvis", "2D materials", "2d-materials",
    "2D material", "van der Waals", "vdW", "TMD", "MoS2", "WSe2", "MoSe2", "transition metal dichalcogenide",
    "heterostructure", "materials project", "matbench", "monolayer", "exfoliable", "layered", "alexandria",
    "OMat", "interlayer",
]


def main(out: str) -> None:
    api = HfApi()
    hits: dict[str, dict] = {}
    for q in QUERIES:
        try:
            res = list(api.list_datasets(search=q, limit=60, full=True))
        except Exception as exc:  # network errors are recorded, not hidden
            hits[f"__error__{q}"] = {"error": repr(exc)}
            continue
        for d in res:
            rec = hits.setdefault(d.id, {"id": d.id, "queries": [], "sha": getattr(d, "sha", None),
                                         "tags": list(getattr(d, "tags", []) or []),
                                         "downloads": getattr(d, "downloads", None),
                                         "last_modified": str(getattr(d, "last_modified", ""))})
            rec["queries"].append(q)
    with open(out, "w") as fh:
        json.dump(sorted(hits.values(), key=lambda r: r["id"]), fh, indent=1)
    for r in sorted(hits.values(), key=lambda r: r["id"]):
        if "error" in r:
            print("ERR", r)
            continue
        lic = [t for t in r["tags"] if t.startswith("license:")]
        print(r["id"], r["downloads"], lic, ",".join(r["queries"]))


if __name__ == "__main__":
    main(sys.argv[1])
