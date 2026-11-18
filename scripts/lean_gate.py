#!/usr/bin/env python3
"""Lean strict gate (ratchet) for proofs/lean4.  Python standard library only.

Run as:  python3 -I scripts/lean_gate.py --selftest | --check | --write-baseline

What it does
  STATIC pass (every .lean file under proofs/lean4, recursive): comments (line, nested block,
  doc) and string literals are stripped, then the code is searched for `sorry`, `admit`,
  `native_decide`, `axiom` declarations, vacuous statements (`: True :=`, `:= True` aliases)
  and output-forgery constructs (`#exit`, `#print "..."`, `#eval`, `run_cmd`, `IO.println`,
  notation/macro hijacking).
  COMPILE pass (only files with NO imports, i.e. pure Lean core): the file is staged in a temp
  dir with `#print axioms <name>` appended for every theorem/lemma, compiled with `lean`, and
  rejected on a compile error, a timeout, `sorryAx`, or any axiom outside
  {propext, Classical.choice, Quot.sound}.
  Files with imports (Mathlib, local modules) are NOT compiled in CI: that needs a Mathlib
  build.  They only get the static pass, and the output says so.

The ratchet compares against proofs/lean4/GATE_BASELINE.json (see --check rules in the PR).
The gate checks axioms and obvious vacuity.  It does not check that a statement means what
its name claims.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parent.parent
DEFAULT_ROOT = REPO / "proofs" / "lean4"
DEFAULT_BASELINE = DEFAULT_ROOT / "GATE_BASELINE.json"
ALLOWED_AXIOMS = {"propext", "Classical.choice", "Quot.sound"}
TIMEOUT_S = float(os.environ.get("LEAN_GATE_TIMEOUT", "300"))
COUNT_KEYS = ("code_sorry", "admit", "native_decide", "axiom_decls", "vacuity", "forgery")
REASON_CLASSES = ("FAIL-SORRY", "FAIL-AXIOM", "FAIL-IMPORT", "FAIL-COMPILE", "VACUITY")
BANNER = "Lean strict gate (ratchet): {n} files are known failing and are not hidden"

# ----------------------------------------------------------------------------- static pass


def strip_code(src: str) -> str:
    """Return `src` with comments and string literal contents blanked (newlines preserved)."""
    out: list[str] = []
    i, n = 0, len(src)
    while i < n:
        if src.startswith("/-", i):  # block comment, nested, doc comments included
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/-", j):
                    depth += 1
                    j += 2
                elif src.startswith("-/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            out.append("".join("\n" if ch == "\n" else " " for ch in src[i:j]))
            i = j
        elif src.startswith("--", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif src.startswith("'\"'", i):  # char literal holding a double quote
            out.append("'_'")
            i += 3
        elif src[i] == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            j = min(j + 1, n)
            seg = src[i:j]
            out.append('""' + "".join("\n" if ch == "\n" else " " for ch in seg[2:]))
            i = j
        else:
            out.append(src[i])
            i += 1
    return "".join(out)


_FORGERY_PATTERNS = (
    r"(?:^|\s)#exit\b",
    r"#print\s+\"",
    r"(?:^|\s)(?:#eval|run_cmd)\b|\bIO\.print(?:ln)?\b",
    r"(?:^|\s)(?:macro_rules|(?:local|scoped)\s+notation)\b",
)
_VACUITY_PATTERNS = (
    r"\b(?:theorem|lemma)\s+([^\s:({\[]+)(?:(?!:=)[\s\S])*:\s*True\s*:=",
    r"\b(?:abbrev|def)\s+([^\s:({\[]+)(?:(?!:=)[^\n])*:=\s*(?:True|Trivial)\b",
)
_AXIOM_DECL = r"(?m)^\s*(?:@\[[^\]]*\]\s*)*(?:(?:private|protected|noncomputable|unsafe)\s+)*axiom\b"
_TOP_KW = re.compile(
    r"^(?:@\[[^\]]*\]\s*)*(?:(?:private|protected|noncomputable|unsafe|partial)\s+)*"
    r"(theorem|lemma|def|abbrev|instance|structure|inductive|class|axiom|opaque|example|"
    r"namespace|section|end|open|variable|macro|syntax|notation|#\w+)\b"
)


def analyse_source(src: str) -> dict[str, Any]:
    code = strip_code(src)
    imports = re.findall(r"^import\s+(\S+)", code, re.M)
    return {
        "imports": imports,
        "raw_sorry": len(re.findall(r"\bsorry\b", src)),
        "code_sorry": len(re.findall(r"\bsorry\b", code)),
        "admit": len(re.findall(r"\badmit\b", code)),
        "native_decide": len(re.findall(r"\bnative_decide\b", code)),
        "axiom_decls": len(re.findall(_AXIOM_DECL, code)),
        "vacuity": sum(len(re.findall(p, code)) for p in _VACUITY_PATTERNS),
        "forgery": sum(len(re.findall(p, code)) for p in _FORGERY_PATTERNS),
    }


def theorem_names(src: str) -> list[str]:
    """Fully qualified names of the non-private theorems/lemmas (namespace/section aware)."""
    code = strip_code(src)
    scopes: list[tuple[str, str]] = []
    names: list[str] = []
    for line in code.split("\n"):
        m = _TOP_KW.match(line)
        if not m:
            continue
        kw, rest = m.group(1), line[m.end():].strip()
        if kw in ("namespace", "section"):
            scopes.append((kw, rest.split()[0] if rest else ""))
        elif kw == "end":
            if scopes:
                scopes.pop()
        elif kw in ("theorem", "lemma"):
            if "private" in line[: m.end()]:
                continue
            nm = re.match(r"([^\s:({\[]+)", rest)
            if not nm:
                continue
            name = re.sub(r"\.\{[^}]*\}$", "", nm.group(1))
            if name.startswith("_root_."):
                full = name[len("_root_."):]
            else:
                prefix = ".".join(s[1] for s in scopes if s[0] == "namespace" and s[1])
                full = (prefix + "." if prefix else "") + name
            if full not in names:
                names.append(full)
    return names


def static_violations(info: dict[str, Any]) -> list[str]:
    out = []
    for key in COUNT_KEYS:
        if info[key]:
            out.append(f"{key}={info[key]}")
    return out


# ---------------------------------------------------------------------------- compile pass

_AX_RE = re.compile(r"'([^']+)' depends on axioms: \[(.*?)\]", re.S)
_NOAX_RE = re.compile(r"'([^']+)' does not depend on any axioms")


def lean_version(lean: str) -> str:
    try:
        r = subprocess.run([lean, "--version"], capture_output=True, text=True, timeout=60)
        return r.stdout.strip() or r.stderr.strip()
    except (OSError, subprocess.SubprocessError) as exc:
        return f"UNAVAILABLE: {exc}"


def compile_file(src: str, lean: str, timeout: float = TIMEOUT_S) -> tuple[bool, str]:
    """Compile pure-core Lean source with `#print axioms` appended.  (ok, detail)."""
    names = theorem_names(src)
    staged = src if src.endswith("\n") else src + "\n"
    staged += "\n" + "".join(f"#print axioms {nm}\n" for nm in names)
    with tempfile.TemporaryDirectory(prefix="lean_gate_") as td:
        f = Path(td) / "Staged.lean"
        f.write_text(staged, encoding="utf-8")
        try:
            r = subprocess.run([lean, str(f)], cwd=td, capture_output=True, text=True, timeout=timeout)
        except subprocess.TimeoutExpired:
            return False, f"timeout after {timeout:.0f}s"
        except OSError as exc:
            return False, f"cannot run lean: {exc}"
    out = (r.stdout or "") + (r.stderr or "")
    first_err = next((ln for ln in out.splitlines() if "error" in ln), "").replace(td, "")[:200]
    if r.returncode != 0 or re.search(r"\berror\b", out):
        return False, "compile error: " + (first_err or f"rc={r.returncode}")
    found: dict[str, list[str]] = {}
    for m in _AX_RE.finditer(out):
        found[m.group(1)] = [a.strip() for a in m.group(2).replace("\n", " ").split(",") if a.strip()]
    for m in _NOAX_RE.finditer(out):
        found[m.group(1)] = []
    missing = [nm for nm in names if nm not in found]
    if missing:
        return False, f"no axiom report for {len(missing)} theorem(s), e.g. {missing[0]}"
    bad: dict[str, list[str]] = {}
    for nm, axs in found.items():
        extra = [a for a in axs if a not in ALLOWED_AXIOMS]
        if extra or "sorryAx" in axs:
            bad[nm] = extra
    if bad:
        nm, extra = next(iter(bad.items()))
        return False, f"axiom audit failed for {len(bad)} theorem(s), e.g. {nm}: {', '.join(extra)}"
    return True, f"{len(names)} theorem(s), axioms within whitelist"


# --------------------------------------------------------------------------------- scanning


def scan(root: Path, lean: str | None, do_compile: bool = True) -> dict[str, dict[str, Any]]:
    res: dict[str, dict[str, Any]] = {}
    for p in sorted(root.rglob("*.lean")):
        if ".lake" in p.parts:
            continue
        rel = p.relative_to(root).as_posix()
        src = p.read_text(encoding="utf-8", errors="replace")
        info = analyse_source(src)
        info["compiled_in_ci"] = not info["imports"]
        info["compile_ok"] = None
        info["compile_detail"] = "compile not run in CI: needs Mathlib" if info["imports"] else ""
        if info["compiled_in_ci"] and do_compile and lean:
            ok, detail = compile_file(src, lean)
            info["compile_ok"], info["compile_detail"] = ok, detail
        res[rel] = info
    return res


def is_clean(info: dict[str, Any]) -> bool:
    return not static_violations(info) and info["compile_ok"] is not False


# ------------------------------------------------------------------------------- baseline


def write_baseline(root: Path, out: Path, lean: str, reasons_file: Path | None, commit: str) -> int:
    reasons: dict[str, dict[str, str]] = {}
    if reasons_file:
        reasons = json.loads(reasons_file.read_text(encoding="utf-8"))
    cur = scan(root, lean)
    files: dict[str, Any] = {}
    problems = []
    for rel, info in cur.items():
        mine_pass = is_clean(info)
        given = reasons.get(rel)
        if given is None:
            if mine_pass:
                cls, reason = "PASS", ""
            elif info["code_sorry"] or info["admit"]:
                cls, reason = "KNOWN_FAIL", "FAIL-SORRY"
            elif info["axiom_decls"] or info["native_decide"]:
                cls, reason = "KNOWN_FAIL", "FAIL-AXIOM"
            elif info["vacuity"] or info["forgery"]:
                cls, reason = "KNOWN_FAIL", "VACUITY"
            else:
                cls, reason = "KNOWN_FAIL", "FAIL-COMPILE"
        else:
            cls = "PASS" if given["class"] == "PASS" else "KNOWN_FAIL"
            reason = "" if cls == "PASS" else given["class"]
            if cls == "PASS" and not mine_pass:
                problems.append(f"{rel}: earlier report says PASS but this script finds {static_violations(info)} / compile={info['compile_detail']}")
            if cls == "KNOWN_FAIL" and reason not in REASON_CLASSES:
                problems.append(f"{rel}: unknown reason class {reason}")
        entry: dict[str, Any] = {
            "imports": info["imports"],
            "compiled_in_ci": info["compiled_in_ci"],
            "static": {k: info[k] for k in COUNT_KEYS},
            "raw_sorry_incl_comments": info["raw_sorry"],
            "expected": cls,
        }
        if reason:
            entry["reason_class"] = reason
        if info["compiled_in_ci"]:
            entry["compile_expected_ok"] = bool(info["compile_ok"])
        note = (given or {}).get("note")
        if note:
            entry["note"] = note
        files[rel] = entry
    if problems:
        print("BASELINE INCONSISTENCIES (not written):", *problems, sep="\n  ")
        return 2
    meta = {
        "lean_version": lean_version(lean),
        "generated_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%d"),
        "generated_at_commit": commit,
        "tool": "scripts/lean_gate.py --write-baseline",
        "reason_classes_source": (
            "docs/verification/lean_gate_2026-11-15.md (full Lean gate with a Mathlib build, 2026-11-15); "
            "audit_unsoundness_demo/fusion_sop_flagno/fusion_sop_monopole from lean_gate_head_2026-11-15.md"
            if reasons_file else "derived by this script from its own measurements"
        ),
        "rules": "see scripts/lean_gate.py --check",
    }
    out.write_text(json.dumps({"schema": 1, "meta": meta, "files": files}, indent=1, sort_keys=True) + "\n",
                   encoding="utf-8")
    n_pass = sum(1 for e in files.values() if e["expected"] == "PASS")
    print(f"baseline written: {out} ({len(files)} files, {n_pass} PASS, {len(files) - n_pass} KNOWN_FAIL)")
    return 0


# ----------------------------------------------------------------------------------- check


def run_check(root: Path, baseline_path: Path, lean: str | None) -> tuple[list[str], list[str], str]:
    """Returns (violations, hints, markdown_summary)."""
    base = json.loads(baseline_path.read_text(encoding="utf-8"))["files"]
    cur = scan(root, lean)
    viol: list[str] = []
    hints: list[str] = []
    for rel in sorted(base):
        if rel not in cur:
            viol.append(f"{rel}: listed in GATE_BASELINE.json but the file no longer exists (regenerate the baseline)")
    for rel, info in sorted(cur.items()):
        b = base.get(rel)
        sv = static_violations(info)
        cdet = info["compile_detail"]
        if b is None:  # (a) new file must be clean
            if sv:
                viol.append(f"{rel}: NEW file not in baseline must be clean, found {', '.join(sv)}")
            if info["compile_ok"] is False:
                viol.append(f"{rel}: NEW file fails the compile pass: {cdet}")
            continue
        if b["expected"] == "PASS":  # (b)
            if sv:
                viol.append(f"{rel}: baseline PASS file regressed (static): {', '.join(sv)}")
            if info["compile_ok"] is False:
                viol.append(f"{rel}: baseline PASS file regressed (compile): {cdet}")
        else:  # (c) ratchet
            for k in COUNT_KEYS:
                if info[k] > b["static"].get(k, 0):
                    viol.append(f"{rel}: KNOWN_FAIL file got worse: {k} {b['static'].get(k, 0)} -> {info[k]}")
            better = [k for k in COUNT_KEYS if info[k] < b["static"].get(k, 0)]
            if better:
                hints.append(f"{rel}: improved ({', '.join(better)} decreased); regenerate the baseline with --write-baseline")
            if not sv and info["compile_ok"] is True:
                hints.append(f"{rel}: now passes static and compile; regenerate the baseline to promote it to PASS")
        if info["compiled_in_ci"] != b["compiled_in_ci"]:
            viol.append(f"{rel}: imports changed (compiled_in_ci {b['compiled_in_ci']} -> {info['compiled_in_ci']}); regenerate the baseline")
    return viol, hints, summary_table(cur, base)


def summary_table(cur: dict[str, dict[str, Any]], base: dict[str, Any]) -> str:
    kf = sorted(r for r in cur if r in base and base[r]["expected"] != "PASS")
    n_pass = sum(1 for r in cur if r in base and base[r]["expected"] == "PASS")
    in_ci = [r for r, i in cur.items() if i["compiled_in_ci"]]
    not_ci = [r for r, i in cur.items() if not i["compiled_in_ci"]]
    compiled_ok = sum(1 for r in in_ci if cur[r]["compile_ok"])
    lines = [
        f"## {BANNER.format(n=len(kf))}",
        "",
        "| Measure | Files |",
        "|---|---|",
        f"| Lean files scanned (static pass) | {len(cur)} |",
        f"| Baseline PASS | {n_pass} |",
        f"| Baseline KNOWN_FAIL | {len(kf)} |",
        f"| Compiled in CI (no imports) | {len(in_ci)} ({compiled_ok} compile with axioms inside the whitelist) |",
        f"| Not compiled in CI (compile not run in CI: needs Mathlib) | {len(not_ci)} |",
        "",
        "Known-failing files (reason class from the 2026-11-15 full gate):",
        "",
        "| File | Reason class | Compiled in CI |",
        "|---|---|---|",
    ]
    for r in kf:
        lines.append(f"| {r} | {base[r].get('reason_class', '?')} | {'yes' if cur[r]['compiled_in_ci'] else 'no (needs Mathlib)'} |")
    lines += ["", "The gate checks axioms and obvious vacuity, not whether a statement means what its name claims.", ""]
    return "\n".join(lines)


def cmd_check(root: Path, baseline: Path, lean: str) -> int:
    if not baseline.exists():
        print(f"FAIL: baseline {baseline} is missing; run --write-baseline", file=sys.stderr)
        return 1
    viol, hints, md = run_check(root, baseline, lean)
    print(md)
    step = os.environ.get("GITHUB_STEP_SUMMARY")
    if step:
        with open(step, "a", encoding="utf-8") as fh:
            fh.write(md + "\n")
    for h in hints:
        print("HINT:", h)
    if viol:
        print(f"\nLEAN GATE FAILED: {len(viol)} violation(s)", file=sys.stderr)
        for v in viol:
            print("  -", v, file=sys.stderr)
        return 1
    print("\nLEAN GATE OK: no regression against GATE_BASELINE.json")
    return 0


# ---------------------------------------------------------------------------------- selftest


def cmd_selftest(lean: str) -> int:
    failures: list[str] = []

    def judge(src: str) -> tuple[bool, str]:
        info = analyse_source(src)
        sv = static_violations(info)
        ok, detail = compile_file(src, lean)
        return (not sv and ok), f"static={sv or 'clean'}; compile={'ok' if ok else detail}"

    reject = {
        "theorem proved with sorry": "theorem s : 1 = 2 := by sorry\n",
        "theorem proved with native_decide": "theorem nd : 2 + 2 = 4 := by native_decide\n",
        "custom axiom used by a theorem": "axiom foo : False\ntheorem t : 1 = 2 := foo.elim\n",
        "vacuous 'theorem t : True := trivial'": "theorem t : True := trivial\n",
        "sorry in code after a comment mentioning it": "/- not sorry here -/\ntheorem s : 1 = 2 := by sorry\n",
    }
    for label, src in reject.items():
        ok, why = judge(src)
        print(f"selftest {'ok  ' if not ok else 'FAIL'} REJECT expected: {label}  [{why}]")
        if ok:
            failures.append(f"accepted: {label}")
    # layer checks: compile alone must catch sorry / native_decide / axiom (not only the static pass)
    for label in ("theorem proved with sorry", "theorem proved with native_decide", "custom axiom used by a theorem"):
        ok, why = compile_file(reject[label], lean)
        print(f"selftest {'ok  ' if not ok else 'FAIL'} compile pass alone rejects: {label}  [{why}]")
        if ok:
            failures.append(f"compile pass alone accepted: {label}")
    # comment stripping: the word in comments/strings must not trigger, in code it must
    commented = (
        "/- sorry admit native_decide /- nested sorry -/ axiom bad : False -/\n"
        "-- sorry in a line comment, theorem x : True := trivial\n"
        "/-- doc comment with sorry -/\n"
        "theorem two : 1 + 1 = 2 := rfl\n"
        "def msg : String := \"sorry native_decide\"\n"
    )
    info = analyse_source(commented)
    good = not static_violations(info) and info["raw_sorry"] > 0
    print(f"selftest {'ok  ' if good else 'FAIL'} comments/strings containing sorry do NOT trigger  [{static_violations(info) or 'clean'}]")
    if not good:
        failures.append("comment stripping wrong")
    ok, why = judge(commented)
    print(f"selftest {'ok  ' if ok else 'FAIL'} ACCEPT expected: commented file compiles clean  [{why}]")
    if not ok:
        failures.append("rejected commented file")
    ok, why = judge("theorem two : 1 + 1 = 2 := rfl\n")
    print(f"selftest {'ok  ' if ok else 'FAIL'} ACCEPT expected: theorem two : 1 + 1 = 2 := rfl  [{why}]")
    if not ok:
        failures.append("rejected the good theorem")
    # namespace-aware naming must produce a name Lean accepts
    ns_src = "namespace A\nnamespace B\ntheorem inner : 1 + 1 = 2 := rfl\nend B\ntheorem outer : 2 = 2 := rfl\nend A\n"
    ok, why = compile_file(ns_src, lean)
    print(f"selftest {'ok  ' if ok else 'FAIL'} ACCEPT expected: nested namespaces get qualified #print axioms  [{why}]")
    if not ok or theorem_names(ns_src) != ["A.B.inner", "A.outer"]:
        failures.append("namespace tracking wrong")
    # ratchet controls on a temp tree
    with tempfile.TemporaryDirectory(prefix="lean_gate_ratchet_") as td:
        root = Path(td) / "lean4"
        root.mkdir()
        (root / "good.lean").write_text("theorem g : 1 + 1 = 2 := rfl\n", encoding="utf-8")
        (root / "bad.lean").write_text("theorem b : 1 = 2 := by sorry\ntheorem c : 1 = 3 := by sorry\n", encoding="utf-8")
        base_path = Path(td) / "base.json"
        reasons = Path(td) / "reasons.json"
        reasons.write_text(json.dumps({"bad.lean": {"class": "FAIL-SORRY"}, "good.lean": {"class": "PASS"}}))
        rc = write_baseline(root, base_path, lean, reasons, "selftest")
        v, _, _ = run_check(root, base_path, lean)
        step = "ok  " if rc == 0 and not v else "FAIL"
        print(f"selftest {step} ratchet: fresh baseline is green  [{v or 'no violations'}]")
        if rc or v:
            failures.append("fresh baseline not green")
        data = json.loads(base_path.read_text())
        data["files"]["bad.lean"]["static"]["code_sorry"] = 1  # pretend there used to be fewer
        base_path.write_text(json.dumps(data))
        v, _, _ = run_check(root, base_path, lean)
        step = "ok  " if any("got worse" in x for x in v) else "FAIL"
        print(f"selftest {step} ratchet: lowered baseline count makes --check fail  [{v[:1]}]")
        if step == "FAIL":
            failures.append("lowered count not detected")
        data["files"]["bad.lean"]["static"]["code_sorry"] = 5  # improvement is allowed
        base_path.write_text(json.dumps(data))
        v, h, _ = run_check(root, base_path, lean)
        step = "ok  " if not v and h else "FAIL"
        print(f"selftest {step} ratchet: an improvement passes with a regenerate hint  [{h[:1]}]")
        if step == "FAIL":
            failures.append("improvement mishandled")
        data["files"]["bad.lean"]["static"]["code_sorry"] = 2
        base_path.write_text(json.dumps(data))
        (root / "good.lean").write_text("theorem g : 1 + 1 = 2 := by sorry\n", encoding="utf-8")
        v, _, _ = run_check(root, base_path, lean)
        step = "ok  " if any("baseline PASS file regressed" in x for x in v) else "FAIL"
        print(f"selftest {step} ratchet: sorry added to a baseline PASS file fails  [{v[:1]}]")
        if step == "FAIL":
            failures.append("PASS regression not detected")
        (root / "good.lean").write_text("theorem g : 1 + 1 = 2 := rfl\n", encoding="utf-8")
        (root / "new.lean").write_text("theorem n : 1 = 2 := by sorry\n", encoding="utf-8")
        v, _, _ = run_check(root, base_path, lean)
        step = "ok  " if any("NEW file" in x for x in v) else "FAIL"
        print(f"selftest {step} ratchet: a new unclean file is rejected  [{v[:1]}]")
        if step == "FAIL":
            failures.append("new unclean file not detected")
        (root / "new.lean").unlink()
        (root / "bad.lean").unlink()
        v, _, _ = run_check(root, base_path, lean)
        step = "ok  " if any("no longer exists" in x for x in v) else "FAIL"
        print(f"selftest {step} ratchet: a baseline entry whose file is gone is an error  [{v[:1]}]")
        if step == "FAIL":
            failures.append("missing file not detected")
    if failures:
        print(f"\nSELFTEST FAILED: {failures}", file=sys.stderr)
        return 1
    print("\nSELFTEST OK: all negative and positive controls behaved as required")
    return 0


# -------------------------------------------------------------------------------------- main


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--check", action="store_true", help="CI: compare against the baseline")
    g.add_argument("--write-baseline", action="store_true", help="regenerate GATE_BASELINE.json from the current files")
    g.add_argument("--selftest", action="store_true", help="negative/positive controls")
    ap.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    ap.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    ap.add_argument("--lean", default=shutil.which("lean") or "lean")
    ap.add_argument("--reasons", type=Path, default=None, help="JSON {file: {class, note}} from an earlier full gate (baseline generation only)")
    ap.add_argument("--commit", default="unknown")
    a = ap.parse_args()
    if a.selftest:
        return cmd_selftest(a.lean)
    if a.write_baseline:
        return write_baseline(a.root, a.baseline, a.lean, a.reasons, a.commit)
    return cmd_check(a.root, a.baseline, a.lean)


if __name__ == "__main__":
    raise SystemExit(main())
