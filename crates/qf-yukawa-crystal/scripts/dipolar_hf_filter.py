"""Print the Hub search hits that came from materials-relevant queries. Usage: python -I dipolar_hf_filter.py <hf_search.json>"""
from __future__ import annotations

import json
import sys

KEEP = {"BiDB", "bilayer", "heterobilayer", "interlayer", "2D materials", "2D material", "2d-materials",
        "van der Waals", "monolayer", "exfoliable", "heterostructure", "WSe2", "MoSe2",
        "transition metal dichalcogenide", "C2DB", "c2db", "matbench"}

for r in json.load(open(sys.argv[1])):
    q = set(r.get("queries", []))
    if q & KEEP:
        lic = [t for t in r.get("tags", []) if t.startswith("license")]
        print(r["id"], r.get("downloads"), r.get("sha"), lic, sorted(q))
