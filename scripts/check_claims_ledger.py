#!/usr/bin/env python3
"""Validate docs/verification/CLAIMS_LEDGER.json and regenerate CLAIMS_LEDGER.md.

Standard library only; run as `python3 -I scripts/check_claims_ledger.py`.
  (no flag)   validate, then write CLAIMS_LEDGER.md
  --check     validate, then fail if the committed .md differs from regenerated
  --selftest  feed deliberately invalid ledgers; each rule must reject them
"""
import datetime
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "verification" / "CLAIMS_LEDGER.json"
MD = ROOT / "docs" / "verification" / "CLAIMS_LEDGER.md"
GATE = ROOT / "docs" / "verification" / "lean_gate_2026-11-15.md"
TIERS = ("A", "C", "L", "X")
STATUSES = ("SUPPORTED", "NOT_SUPPORTED", "UNVERIFIED", "NOT_RUN")
FIELDS = ("id", "claim", "readme_line", "tier", "status", "evidence", "checked_on")
# Optional fields (added 2026-11-22). "convention": the parametrisation or definition the claim uses (for example
# r^-s versus r^-2s, open versus closed disks, coverage over source files only). "hypotheses": the exact hypotheses of
# the cited theorem or measurement. Both are required for tier A rows, where a mis-cited hypothesis does most damage.
OPTIONAL_FIELDS = ("convention", "hypotheses")
LEGEND = {
    "A": "kernel-checked (strict Lean gate: compile, axiom whitelist, no sorry, not flagged trivial)",
    "C": "computed and audited (a command or measurement that was actually run)",
    "L": "cited from the literature",
    "X": "unverified",
}


def gate_pass_files(gate_text):
    """Return {file: flagged_count} for rows classed PASS in the gate report."""
    out = {}
    for line in gate_text.splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) >= 4 and cells[1] == "PASS" and cells[0].endswith(".lean"):
            try:
                out[cells[0]] = int(cells[3])
            except ValueError:
                out[cells[0]] = -1
    return out


def validate(rows, gate_text):
    """Return a list of error strings (empty means valid)."""
    errs = []
    if not isinstance(rows, list) or not rows:
        return ["ledger must be a non-empty JSON array"]
    passed = gate_pass_files(gate_text)
    seen = set()
    for i, r in enumerate(rows):
        if not isinstance(r, dict):
            errs.append(f"row {i}: not an object")
            continue
        rid = r.get("id", f"#{i}")
        for f in FIELDS:
            if f not in r:
                errs.append(f"{rid}: missing field {f}")
        if rid in seen:
            errs.append(f"{rid}: duplicate id")
        seen.add(rid)
        if r.get("tier") not in TIERS:
            errs.append(f"{rid}: tier must be one of {TIERS}")
        if r.get("status") not in STATUSES:
            errs.append(f"{rid}: status must be one of {STATUSES}")
        ev = r.get("evidence")
        if not isinstance(ev, str) or not ev.strip():
            errs.append(f"{rid}: evidence is empty")
            ev = ""
        try:
            datetime.date.fromisoformat(str(r.get("checked_on")))
        except ValueError:
            errs.append(f"{rid}: checked_on is not an ISO date")
        for f in OPTIONAL_FIELDS:
            if f in r and (not isinstance(r[f], str) or not r[f].strip()):
                errs.append(f"{rid}: {f} is present but empty")
            if r.get("tier") == "A" and f not in r:
                errs.append(f"{rid}: tier A rows must state {f}")
        if r.get("status") == "SUPPORTED" and r.get("tier") == "X":
            errs.append(f"{rid}: status SUPPORTED is not allowed with tier X")
        if r.get("tier") == "A":
            named = [f for f in passed if f in ev]
            if not named:
                errs.append(f"{rid}: tier A evidence names no file that is PASS in the gate report")
            for f in named:
                if passed[f] != 0:
                    errs.append(f"{rid}: tier A file {f} is flagged in the gate report")
    return errs


def render(rows):
    lines = [
        "# Claims ledger",
        "",
        "Generated from `CLAIMS_LEDGER.json` by `scripts/check_claims_ledger.py`; do not edit by hand.",
        "A row is only as honest as its evidence column; the script checks structure and two consistency rules, not truth.",
        "",
        "Tier legend:",
        "",
    ]
    lines += [f"- **{t}**: {LEGEND[t]}" for t in TIERS]
    lines += ["", "| id | claim | readme line | tier | status | evidence | checked on | convention | hypotheses |",
              "|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        cells = [str(r.get(f, "")).replace("|", "\\|").replace("\n", " ") for f in FIELDS + OPTIONAL_FIELDS]
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines) + "\n"


def selftest(gate_text):
    good = {"id": "ok", "claim": "c", "readme_line": "1", "tier": "C", "status": "SUPPORTED",
            "evidence": "cmd", "checked_on": "2026-11-17"}
    passed = gate_pass_files(gate_text)
    clean = sorted(f for f, n in passed.items() if n == 0)
    flagged = sorted(f for f, n in passed.items() if n != 0)
    if not clean or not flagged:
        print("selftest FAILED: gate report needs one clean and one flagged PASS file")
        return 1
    bad = {
        "duplicate id": [good, dict(good)],
        "bad tier": [dict(good, id="b", tier="Z")],
        "bad status": [dict(good, id="b", status="MAYBE")],
        "empty evidence": [dict(good, id="b", evidence="  ")],
        "bad date": [dict(good, id="b", checked_on="yesterday")],
        "SUPPORTED with tier X": [dict(good, id="b", tier="X")],
        "tier A naming no PASS file": [dict(good, id="b", tier="A", evidence="nonexistent_file.lean")],
        "tier A naming a flagged PASS file": [dict(good, id="b", tier="A", evidence=flagged[0])],
        "missing field": [{k: v for k, v in good.items() if k != "claim"}],
        "tier A without convention": [dict(good, id="b", tier="A", evidence=clean[0], hypotheses="h")],
        "tier A without hypotheses": [dict(good, id="b", tier="A", evidence=clean[0], convention="c")],
        "empty convention": [dict(good, id="b", convention="  ")],
    }
    failed = 0
    for label, rows in (("a valid row", [good]),
                        ("a valid tier A row", [dict(good, id="a", tier="A", evidence=clean[0],
                                                      convention="c", hypotheses="h")])):
        if validate(rows, gate_text):
            print(f"selftest FAILED: {label} was rejected")
            failed += 1
    with tempfile.TemporaryDirectory() as tmp:
        for name, rows in bad.items():
            p = Path(tmp) / "bad.json"
            p.write_text(json.dumps(rows), encoding="utf-8")
            errs = validate(json.loads(p.read_text(encoding="utf-8")), gate_text)
            print(f"selftest {'ok' if errs else 'FAILED'}: rejects {name}")
            failed += 0 if errs else 1
    return 1 if failed else 0


def main(argv):
    gate_text = GATE.read_text(encoding="utf-8")
    if "--selftest" in argv:
        return selftest(gate_text)
    rows = json.loads(LEDGER.read_text(encoding="utf-8"))
    errs = validate(rows, gate_text)
    if errs:
        print("\n".join(errs))
        return 1
    text = render(rows)
    if "--check" in argv:
        if not MD.exists() or MD.read_text(encoding="utf-8") != text:
            print("CLAIMS_LEDGER.md is out of date; run python3 -I scripts/check_claims_ledger.py")
            return 1
        print(f"ledger ok: {len(rows)} rows, markdown up to date")
        return 0
    MD.write_text(text, encoding="utf-8")
    print(f"wrote {MD} ({len(rows)} rows)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
