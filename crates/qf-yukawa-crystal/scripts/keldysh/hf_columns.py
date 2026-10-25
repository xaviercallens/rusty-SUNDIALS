"""Print the columns of the colabfit JARVIS_C2DB parquet files and the rows for the four MX2 TMD monolayers.

Usage: hf_columns.py DATA_DIR
"""

import json
import sys

import pyarrow.parquet as pq


def main() -> None:
    d = sys.argv[1]
    ds = pq.read_table(d + "/ds.parquet")
    print("ds columns:", ds.column_names)
    co = pq.read_table(d + "/co/co_0.parquet")
    print("co columns:", co.column_names, "rows:", co.num_rows)
    rows = co.to_pylist()
    print(json.dumps({k: str(v)[:200] for k, v in rows[0].items()}, indent=1))
    keys = [k for k in co.column_names if "formula" in k or "name" in k or "label" in k]
    print("formula-like keys:", keys)


if __name__ == "__main__":
    main()
