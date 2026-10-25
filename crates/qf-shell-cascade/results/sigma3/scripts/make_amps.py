"""Write well_amps.txt (time_index a_0..a_5) from well_spectrum.json for the Rust study binary."""
import json
import sys


def main() -> None:
    rep = json.load(open(sys.argv[1]))
    with open(sys.argv[2], "w") as fh:
        fh.write("# time_index |v_0| ... |v_5| (sqrt(2 E_n), data units; the study normalises S = 1)\n")
        for s in rep["snapshots"]:
            fh.write(str(s["time_index"]) + " " + " ".join(repr(a) for a in s["amp_n"]) + "\n")


if __name__ == "__main__":
    main()
