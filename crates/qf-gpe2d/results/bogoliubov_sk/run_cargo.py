"""Run a cargo subcommand for qf-gpe2d pinned to cores 4-7, logging output to a file.

Usage: run_cargo.py LOGFILE SUBCOMMAND [extra args...]
SUBCOMMAND is one of: test, fmt, clippy.
"""
from __future__ import annotations

import os
import subprocess
import sys

WT = "/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-ar-h8/Cargo.toml"
TARGET = "/home/callensxavier_gmail_com/rusty-target-ar"


def main(log: str, sub: str, extra: list[str]) -> int:
    env = dict(os.environ, CARGO_TARGET_DIR=TARGET)
    if sub == "test":
        cmd = ["cargo", "test", "-j", "2", "--offline", "--manifest-path", WT, "-p", "qf-gpe2d",
               *extra, "--", "--test-threads=2"]
    elif sub == "fmt":
        cmd = ["cargo", "fmt", "--manifest-path", WT, "-p", "qf-gpe2d", *extra]
    elif sub == "clippy":
        cmd = ["cargo", "clippy", "-j", "2", "--offline", "--manifest-path", WT, "-p", "qf-gpe2d",
               "--all-targets", *extra]
    else:
        raise SystemExit(f"unknown subcommand {sub}")
    os.sched_setaffinity(0, {4, 5, 6, 7})
    r = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=1700)
    with open(log, "w") as fh:
        fh.write("$ " + " ".join(cmd) + "\n")
        fh.write(r.stdout)
        fh.write(r.stderr)
        fh.write(f"\nexit {r.returncode}\n")
    return r.returncode


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2], sys.argv[3:]))
