"""Download specific small files of a public HF dataset at a pinned revision into a given directory.

Usage: hf_fetch.py REPO_ID REVISION OUT_DIR FILE [FILE ...]
"""

import sys

from huggingface_hub import hf_hub_download


def main() -> None:
    repo, rev, out = sys.argv[1], sys.argv[2], sys.argv[3]
    for f in sys.argv[4:]:
        p = hf_hub_download(repo_id=repo, repo_type="dataset", filename=f, revision=rev, local_dir=out)
        print(p)


if __name__ == "__main__":
    main()
