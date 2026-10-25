"""Compare the local OC20 shard used for the initial conditions with the Hub's LFS sha256 at the current revision,
and walk the repo history to find which revision (if any) carries that exact file.

Usage: python -I hf_shard_check.py LOCAL_PARQUET OUT_JSON
"""

import hashlib
import json
import sys

from huggingface_hub import HfApi

REPO = "colabfit/OC20_IS2RES_val_id"


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> None:
    local, out = sys.argv[1], sys.argv[2]
    api = HfApi()
    local_sha = sha256(local)
    info = api.dataset_info(REPO, files_metadata=True)
    hub = {}
    for s in info.siblings or []:
        if s.rfilename.endswith("co_10.parquet"):
            hub = {"path": s.rfilename, "size": s.size, "lfs_sha256": s.lfs.sha256 if s.lfs else None}
    res = {
        "repo": REPO,
        "current_revision": info.sha,
        "local_path": local,
        "local_sha256": local_sha,
        "hub_file_at_current_revision": hub,
        "match_current_revision": hub.get("lfs_sha256") == local_sha,
        "license": (info.card_data.to_dict() if info.card_data else {}).get("license"),
    }
    try:
        commits = api.list_repo_commits(REPO, repo_type="dataset")
        res["n_commits"] = len(commits)
        res["commits"] = [{"id": c.commit_id, "date": str(c.created_at)} for c in commits[:20]]
    except Exception as e:  # metadata only; report, do not hide
        res["commits_error"] = repr(e)
    json.dump(res, open(out, "w"), indent=1)
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    main()
