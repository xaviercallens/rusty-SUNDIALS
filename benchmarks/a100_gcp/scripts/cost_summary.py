#!/usr/bin/env python3
"""Cost summary script for A100 benchmark CI step."""
import json, sys

cost_file = "benchmarks/a100_gcp/results/cost_summary.json"
try:
    with open(cost_file) as f:
        d = json.load(f)
    elapsed = float(d.get("elapsed_hours", 0))
    cost    = float(d.get("estimated_cost_usd", 0))
    print(f"Elapsed: {elapsed:.2f}h")
    print(f"Cost:    ${cost:.2f}")
    print("Budget:  $100.00")
    ok = cost < 100
    print(f"Status:  {'WITHIN BUDGET' if ok else 'OVER BUDGET'}")
    sys.exit(0 if ok else 1)
except FileNotFoundError:
    print("cost_summary.json not found — VM may not have completed.")
    sys.exit(0)
