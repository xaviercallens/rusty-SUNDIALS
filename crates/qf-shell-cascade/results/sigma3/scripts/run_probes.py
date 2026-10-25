"""Supplementary sigma = 2 probes (same binary, same seeds as P1/C2), written as JSON lines.

Usage: run_probes.py BINARY OUT_JSONL
  (a) decaying (nonlinear) data, sigma = 2, n_max 4 and 6, T = 2: tests H_2 conservation with real cascade;
  (b) top-shell data, sigma = 2, n_max 7..10, T = 0.5: extends the C2 growth law.
Each probe call also runs the other initial family; all output lines are kept.
"""
import os
import subprocess
import sys


def main() -> None:
    binary, out = sys.argv[1], sys.argv[2]
    os.sched_setaffinity(0, {4, 5, 6, 7})
    jobs = [("4", "2"), ("6", "2")]
    jobs_top = [("7", "0.5"), ("8", "0.5"), ("9", "0.5"), ("10", "0.5")]
    with open(out, "w") as fh:
        for n, t in jobs + jobs_top:
            r = subprocess.run([binary, "probe", n, "2", "0.05", t, "1e-10"], capture_output=True, text=True,
                               timeout=1700)
            sys.stderr.write(r.stderr)
            for line in r.stdout.splitlines():
                fh.write(line.replace('"label":"probe_', '"label":"supp_T' + t + '_probe_') + "\n")
            fh.flush()


if __name__ == "__main__":
    main()
