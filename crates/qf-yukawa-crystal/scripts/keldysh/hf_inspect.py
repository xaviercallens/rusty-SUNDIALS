"""List files, revision and card metadata of candidate HF datasets (no large downloads)."""

import json
import sys

from huggingface_hub import HfApi


def main() -> None:
    api = HfApi()
    out = {}
    for rid in sys.argv[1:]:
        info = api.dataset_info(rid, files_metadata=True)
        card = info.card_data.to_dict() if info.card_data else {}
        out[rid] = {
            "sha": info.sha,
            "last_modified": str(info.last_modified),
            "license": card.get("license"),
            "card": {k: card[k] for k in list(card)[:20]},
            "files": [(s.rfilename, s.size) for s in (info.siblings or [])][:200],
        }
    json.dump(out, sys.stdout, indent=1, default=str)


if __name__ == "__main__":
    main()
