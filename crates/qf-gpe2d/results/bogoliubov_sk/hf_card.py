"""Fetch only the dataset card (README.md) and the file list of the one plausible HF hit, to
record why it is out of scope. No data files are downloaded.

Usage: hf_card.py OUT_DIR OUT_JSON
"""
from __future__ import annotations

import json
import sys

from huggingface_hub import HfApi, hf_hub_download

REPO = "popoff4rtem/2D-Neutron-Diffraction-Dataset-for-Discriminative-Models"


def main(out_dir: str, out_json: str) -> None:
    api = HfApi()
    info = api.dataset_info(REPO, files_metadata=True)
    files = [{"name": s.rfilename, "size": s.size} for s in (info.siblings or [])]
    card_path = None
    if any(f["name"] == "README.md" for f in files):
        card_path = hf_hub_download(REPO, "README.md", repo_type="dataset", revision=info.sha,
                                    local_dir=out_dir)
    card = open(card_path).read() if card_path else ""
    rec = {"repo": REPO, "sha": info.sha, "license_tags": [t for t in info.tags if t.startswith("license:")],
           "tags": info.tags, "files": files, "card_path": card_path, "card_head": card[:3000]}
    json.dump(rec, open(out_json, "w"), indent=1)
    print(json.dumps({k: rec[k] for k in ("repo", "sha", "license_tags", "files")}, indent=1)[:3000])
    print("---- card head ----")
    print(card[:2500])


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
