import csv
import glob
import math

for f in sorted(glob.glob("/home/callensxavier_gmail_com/programmes-2026-10-25/P2/data/*__coarse.csv")):
    rows = []
    for g in (f, f.replace("coarse", "random")):
        rows += [r for r in csv.DictReader(open(g)) if int(r["n"]) == 96]
    st = [r for r in rows if int(r["n_unstable"]) == 0]
    d = [math.hypot(float(r["x"]) - 0.5, float(r["y"]) - math.sqrt(3) / 2) for r in st]
    print(
        f.split("/")[-1][:-12],
        len(st),
        "max y %.3f" % max(float(r["y"]) for r in st),
        "min x %.3f" % min(float(r["x"]) for r in st),
        "max dist from rho %.3f" % max(d),
    )
