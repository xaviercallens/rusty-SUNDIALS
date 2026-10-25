"""List files and sizes of candidate The Well datasets on the HuggingFace Hub (metadata only)."""
import sys

from huggingface_hub import HfApi


def main() -> None:
    api = HfApi()
    for repo in sys.argv[1:]:
        try:
            info = api.dataset_info(repo, files_metadata=True)
        except Exception as e:  # noqa: BLE001 - report and continue
            print(repo, "ERROR", type(e).__name__, e)
            continue
        lic = None
        if info.card_data is not None:
            lic = info.card_data.get("license")
        print("REPO", repo, "sha", info.sha, "license", lic)
        sib = sorted(info.siblings, key=lambda s: (s.size or 0))
        for s in sib[:40]:
            print("  ", s.rfilename, s.size)
        print("  n_files", len(sib), "total_bytes", sum((s.size or 0) for s in sib))


if __name__ == "__main__":
    main()
