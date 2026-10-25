"""Print metadata (sha, license, file list with sizes, README head) of Hub datasets.

Usage: python -I hf_inspect.py OUT_DIR ID [ID ...]
Downloads only README.md (into OUT_DIR/<id with / -> __>/).
"""

import json
import os
import sys

from huggingface_hub import HfApi, hf_hub_download


def main() -> None:
    out_dir = sys.argv[1]
    api = HfApi()
    meta = {}
    for rid in sys.argv[2:]:
        info = api.dataset_info(rid, files_metadata=True)
        files = [(s.rfilename, s.size) for s in (info.siblings or [])]
        cd = info.card_data.to_dict() if info.card_data else {}
        meta[rid] = {"sha": info.sha, "last_modified": str(info.last_modified), "license": cd.get("license"), "files": files}
        print(rid, info.sha, cd.get("license"))
        for f, s in files[:60]:
            print("   ", f, s)
        sub = os.path.join(out_dir, rid.replace("/", "__"))
        os.makedirs(sub, exist_ok=True)
        if any(f == "README.md" for f, _ in files):
            p = hf_hub_download(rid, "README.md", repo_type="dataset", revision=info.sha, local_dir=sub)
            print(open(p, encoding="utf-8", errors="replace").read()[:3000])
    json.dump(meta, open(os.path.join(out_dir, "inspect.json"), "w"), indent=1)


if __name__ == "__main__":
    main()
