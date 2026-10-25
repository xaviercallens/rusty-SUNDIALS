"""Download one pinned file of The Well's turbulent_radiative_layer_2D (public, CC-BY-4.0) plus its README."""
import hashlib
import json
import sys

from huggingface_hub import hf_hub_download

REPO = "polymathic-ai/turbulent_radiative_layer_2D"
REV = "2ee7756575ff6f90981d0308cbcb6a2ab5995bdc"
FILES = ["README.md", "turbulent_radiative_layer_2D.yaml", "stats.yaml",
         "data/test/turbulent_radiative_layer_tcool_0.32.hdf5"]


def main() -> None:
    out_dir = sys.argv[1]
    rec = {"repo_id": REPO, "repo_type": "dataset", "revision": REV, "license": "cc-by-4.0", "files": []}
    for f in FILES:
        p = hf_hub_download(REPO, f, repo_type="dataset", revision=REV, local_dir=out_dir)
        h = hashlib.sha256()
        with open(p, "rb") as fh:
            for chunk in iter(lambda: fh.read(1 << 20), b""):
                h.update(chunk)
        rec["files"].append({"path": f, "local": p, "sha256": h.hexdigest()})
        print(f, p, h.hexdigest(), flush=True)
    with open(out_dir.rstrip("/") + "/download_record.json", "w") as fh:
        json.dump(rec, fh, indent=1)


if __name__ == "__main__":
    main()
