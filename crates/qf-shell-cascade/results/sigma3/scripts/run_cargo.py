"""Run cargo for this crate pinned to cores 4-7 with the shared target dir.

Usage: run_cargo.py <log_file> <cargo args...>
The manifest path and CARGO_TARGET_DIR are fixed; stdout+stderr go to the log file and the tail is printed.
"""
import os
import subprocess
import sys

MANIFEST = "/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-ar-h7/Cargo.toml"
TARGET = "/home/callensxavier_gmail_com/rusty-target-ar"


def main() -> None:
    log = sys.argv[1]
    args = sys.argv[2:]
    os.sched_setaffinity(0, {4, 5, 6, 7})
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = TARGET
    cmd = ["cargo"] + args[:1] + ["--manifest-path", MANIFEST] + args[1:]
    with open(log, "w") as fh:
        rc = subprocess.call(cmd, stdout=fh, stderr=subprocess.STDOUT, env=env)
    with open(log) as fh:
        lines = fh.readlines()
    sys.stdout.write("".join(lines[-40:]))
    print("RC", rc)
    sys.exit(rc)


if __name__ == "__main__":
    main()
