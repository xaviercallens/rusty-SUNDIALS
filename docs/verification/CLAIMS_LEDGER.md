# Claims ledger

Generated from `CLAIMS_LEDGER.json` by `scripts/check_claims_ledger.py`; do not edit by hand.
A row is only as honest as its evidence column; the script checks structure and two consistency rules, not truth.

Tier legend:

- **A**: kernel-checked (strict Lean gate: compile, axiom whitelist, no sorry, not flagged trivial)
- **C**: computed and audited (a command or measurement that was actually run)
- **L**: cited from the literature
- **X**: unverified

| id | claim | readme line | tier | status | evidence | checked on |
|---|---|---|---|---|---|---|
| RUST-TESTS | cargo test --workspace --no-fail-fast: 344 passed, 0 failed, 6 ignored at commit 4e3e32d (rustc/cargo 1.96.1) | 44 | C | SUPPORTED | command: cargo test --workspace --no-fail-fast at commit 4e3e32d; README Verification status | 2026-11-15 |
| PY-TESTS | 134 Python tests (80 + 54) in autoresearch_agent/tests | 45 | X | UNVERIFIED | not run; autoresearch_agent/tests | 2026-11-15 |
| COVERAGE | Line coverage 98.4% | 46 | X | UNVERIFIED | no coverage tool was run; the README badge says not re-measured | 2026-11-15 |
| BENCHMARKS | 10 benchmarks | 47 | X | NOT_RUN | not re-run | 2026-11-15 |
| NO-UNSAFE | No unsafe block, fn or impl in the workspace crates (dependencies and macro expansions not audited) | 48 | C | SUPPORTED | search of the workspace crates for unsafe; two #![forbid(unsafe_code)] attributes in crates/sundials-mcp; README Verification status | 2026-11-15 |
| LEAN-GATE | Strict Lean gate on proofs/lean4 at af4886f: 4 of 58 top-level files pass (1 FAIL-SORRY, 6 FAIL-AXIOM, 4 FAIL-IMPORT, 43 FAIL-COMPILE) | 49 | C | SUPPORTED | docs/verification/lean_gate_2026-11-15.md (gate anse/formal/lean_runner.py of AutoevolveAI, Lean 4.34.0-rc2) | 2026-11-15 |
| LEAN-PASS-EQUIV-CVODE-LS | equiv_cvode_ls.lean passes the strict gate, 6 theorems, none flagged trivial. Statement fidelity and non-vacuity have not been reviewed by a person or an independent agent, so this is not yet called Tier A (kernel-checked with a reviewed statement). | 49 | C | SUPPORTED | equiv_cvode_ls.lean PASS, flagged 0 in docs/verification/lean_gate_2026-11-15.md; statement fidelity to the C code not assessed | 2026-11-15 |
| LEAN-PASS-SOP2-FLAGNO | sop2_flagno_scaling.lean passes the strict gate, 2 theorems, none flagged trivial. Statement fidelity and non-vacuity have not been reviewed by a person or an independent agent, so this is not yet called Tier A (kernel-checked with a reviewed statement). | 49 | C | SUPPORTED | sop2_flagno_scaling.lean PASS, flagged 0 in docs/verification/lean_gate_2026-11-15.md; statement fidelity not assessed | 2026-11-15 |
| LEAN-PASS-SOP3-CLOUD | sop3_cloud_economics.lean passes the strict gate, 2 theorems, none flagged trivial. Statement fidelity and non-vacuity have not been reviewed by a person or an independent agent, so this is not yet called Tier A (kernel-checked with a reviewed statement). | 49 | C | SUPPORTED | sop3_cloud_economics.lean PASS, flagged 0 in docs/verification/lean_gate_2026-11-15.md; statement fidelity not assessed | 2026-11-15 |
| LEAN-MATRIX-REVIEW | equiv_sundials_matrix.lean passes the gate but 1 of 5 theorems is flagged trivial; needs a statement review | 49 | X | UNVERIFIED | equiv_sundials_matrix.lean PASS with flagged 1 in docs/verification/lean_gate_2026-11-15.md | 2026-11-15 |
| LEAN-OTHER | The remaining 54 top-level Lean files in proofs/lean4 do not pass the strict gate and are not verified | 49 | X | NOT_SUPPORTED | 54 non-PASS rows in docs/verification/lean_gate_2026-11-15.md | 2026-11-15 |
| LEAN-CI-JOB | CI job 'Verify Lean 4 Proofs' checks the proofs: it runs only lean --run cvode.lean with no axiom check, and that file fails the axiom gate (native_decide) | 54 | C | NOT_SUPPORTED | cvode.lean FAIL-AXIOM in docs/verification/lean_gate_2026-11-15.md; .github/workflows/ci.yml job lean_proofs | 2026-11-15 |
| PROVENANCE-CRATE | crates/sundials-provenance records build-time git SHA, dirty-tree hash, toolchain and preregistration hash |  | C | SUPPORTED | crates/sundials-provenance; commit b7825ce (PR #84); its unit tests run in cargo test --workspace | 2026-11-17 |
| NEGATIVE-CONTROLS | The accuracy checker accepts correct Robertson and decay results and rejects broken ones (k3, k1, loose tolerance, mutated reference); it tests the checker and two ODEs only, not the solver in general |  | C | SUPPORTED | command: cargo test -p cvode --test audit_controls -- --nocapture; file crates/cvode/tests/audit_controls.rs | 2026-11-17 |
| LEDGER-SCRIPT | The ledger script rejects invalid ledgers (duplicate id, bad tier/status/date, empty evidence, SUPPORTED with tier X, tier A without a PASS file or with a flagged one) |  | C | SUPPORTED | command: python3 -I scripts/check_claims_ledger.py --selftest | 2026-11-17 |
