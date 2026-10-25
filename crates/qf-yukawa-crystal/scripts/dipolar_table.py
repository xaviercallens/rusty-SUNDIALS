"""Print a compact run table from figures/summary.json. Usage: python -I dipolar_table.py <summary.json>"""
from __future__ import annotations

import json
import sys

s = json.load(open(sys.argv[1]))
print("| d | label | family | rel_gap_exact | gap_exact | gap_sf | defects | hess_neg | max_force | wall_s |")
print("|---|---|---|---|---|---|---|---|---|---|")
for r in sorted(s["runs"], key=lambda r: (r["d"], r.get("family", ""), r["label"])):
    print(f"| {r['d']} | {r['label']} | {r.get('family')} | {r['rel_gap_exact']:.3e} | {r['gap_exact']:.3e} | "
          f"{r['gap_sf']:.3e} | {r['defects']} | {r.get('hess_negative')} | {r['max_force']:.1e} | {r['wall_s']:.0f} |")
comp = [r for r in s["runs"] if not r.get("family", "").startswith("control_ctl_pos")]
above = [r for r in comp if r["rel_gap_exact"] > 1e-8]
at = [r for r in comp if abs(r["rel_gap_exact"]) <= 1e-8]
print("competitors (excluding positive controls):", len(comp), "above (>1e-8 rel):", len(above),
      "at lattice (|rel|<=1e-8):", len(at), "min rel gap among those above:",
      min(r["rel_gap_exact"] for r in above), "most negative rel gap overall:", min(r["rel_gap_exact"] for r in comp))
diff = [abs(r["gap_exact"] - r["gap_sf"]) for r in comp]
print("max |gap_exact - gap_sf| over competitors:", max(diff))
