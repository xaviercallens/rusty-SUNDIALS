"""Search the HuggingFace Hub for public 2D-materials datasets that may carry 2D polarizabilities (C2DB alpha_2D)."""

import json
import sys

from huggingface_hub import HfApi


def main() -> None:
    api = HfApi()
    out: dict[str, list[dict[str, object]]] = {}
    for q in ["c2db", "C2DB", "2d materials", "2D-materials", "jarvis", "monolayer", "polarizability", "tmd", "2dmatpedia", "materials project 2d", "computational 2d"]:
        rows = []
        for d in api.list_datasets(search=q, limit=40):
            rows.append({"id": d.id, "sha": getattr(d, "sha", None), "downloads": getattr(d, "downloads", None), "tags": list(getattr(d, "tags", []) or [])[:12]})
        out[q] = rows
    json.dump(out, sys.stdout, indent=1)


if __name__ == "__main__":
    main()
