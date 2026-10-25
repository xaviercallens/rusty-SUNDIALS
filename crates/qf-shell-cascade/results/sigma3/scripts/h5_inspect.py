"""Print the HDF5 tree (names, shapes, dtypes, attrs) of one The Well file."""
import sys

import h5py


def show(name: str, obj: object) -> None:
    if isinstance(obj, h5py.Dataset):
        print("DS", name, obj.shape, obj.dtype, dict(obj.attrs))
    else:
        print("GR", name, dict(obj.attrs))


def main() -> None:
    with h5py.File(sys.argv[1], "r") as f:
        print("ROOT attrs", dict(f.attrs))
        f.visititems(show)


if __name__ == "__main__":
    main()
