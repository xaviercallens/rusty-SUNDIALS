"""Python side of the statistical end-to-end check: N=64, L=32, e=0.60, random_state -> 200 t.u. equilibration ->
300 t.u. measurement (round2.run_blocks), seeds 1..3 (default). Usage: thermal_stat_python.py OUT.json [SEEDS], e.g. 4,5. Writes the JSON read by the ignored test."""
import sys, json
sys.path.insert(0, "/home/xavkal/xdev/SocrateAI-Scientific-QuantumFluids/exploration/pgpe")
import numpy as np
from pgpe import PGPE
from round2 import run_blocks
rows = []
seeds = [int(x) for x in sys.argv[2].split(',')] if len(sys.argv) > 2 else [1, 2, 3]
for seed in seeds:
    s = PGPE(N=64, L=32.0)
    c = s.random_state(1.0, 0.60, np.random.default_rng(seed))
    c, blocks, whole = run_blocks(s, c, 500.0, 200.0, seed=seed)
    rows.append({"seed": seed, "T": whole["T"], "nsn": whole["ns_over_n"], "nv": whole["n_v"], "cond": whole["cond"]})
    print(rows[-1], flush=True)
m = lambda k: float(np.mean([r[k] for r in rows]))
out = {"rows": rows, "T_mean": m("T"), "nsn_mean": m("nsn"), "nv_mean": m("nv"), "cond_mean": m("cond")}
json.dump(out, open(sys.argv[1] if len(sys.argv) > 1 else "thermal_stat_python.json", "w"), indent=1)
print(out)
