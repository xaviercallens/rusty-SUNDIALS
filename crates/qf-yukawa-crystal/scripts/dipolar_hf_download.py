"""Download specific files of HF datasets at pinned revisions into an untrusted-data directory.

Usage: python -I dipolar_hf_download.py <dest_dir> <repo>@<sha>:<file>[,<file>...] ...
Writes <dest_dir>/provenance.json with repo, revision, file, size, sha256.
"""
from __future__ import annotations

import hashlib
import json
import os
import sys

from huggingface_hub import hf_hub_download

dest = sys.argv[1]
prov = []
for spec in sys.argv[2:]:
    repo_rev, files = spec.split(":", 1)
    repo, rev = repo_rev.split("@")
    for f in files.split(","):
        p = hf_hub_download(repo_id=repo, filename=f, repo_type="dataset", revision=rev,
                            local_dir=os.path.join(dest, repo.replace("/", "__")))
        h = hashlib.sha256(open(p, "rb").read()).hexdigest()
        prov.append({"repo": repo, "revision": rev, "file": f, "path": p, "bytes": os.path.getsize(p), "sha256": h})
        print(repo, rev, f, os.path.getsize(p), h)
old = []
pp = os.path.join(dest, "provenance.json")
if os.path.exists(pp):
    old = json.load(open(pp))
json.dump(old + prov, open(pp, "w"), indent=1)
