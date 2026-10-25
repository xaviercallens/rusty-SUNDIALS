"""Second Hub search: broader queries for 2D dielectric / polarizability / exciton data."""

from huggingface_hub import HfApi


def main() -> None:
    api = HfApi()
    for q in ["dielectric", "exciton", "2d material", "two-dimensional", "van der waals", "monolayers", "materials property", "matbench", "colabfit", "alpha_2d", "screening length", "c2db"]:
        ids = [d.id for d in api.list_datasets(search=q, limit=60)]
        print(q, ids)


if __name__ == "__main__":
    main()
