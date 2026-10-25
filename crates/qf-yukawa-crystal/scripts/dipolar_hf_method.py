"""Print method/software/metadata columns of chosen JARVIS rows. Usage: python -I dipolar_hf_method.py <shard> <name> ..."""
from __future__ import annotations

import sys

import pyarrow.parquet as pq

want = set(sys.argv[2:])
pf = pq.ParquetFile(sys.argv[1])
cols = [c for c in pf.schema_arrow.names if c in ("names", "method", "software", "metadata", "configuration_id",
                                                  "property_id", "dataset_id", "chemical_formula_hill")]
print("columns used:", cols)
for b in pf.iter_batches(batch_size=8192, columns=cols):
    t = b.to_pydict()
    for i, nm in enumerate(t["names"]):
        if want & set(nm or []):
            print({c: (str(t[c][i])[:300]) for c in cols})
