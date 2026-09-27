# Retraction notice: fusion / ITER results in rusty-SUNDIALS

**Date:** 2026-09-27. **Branch:** `audit/fusion-2026-09-27`. **Baseline commit audited:** `af4886f`.
**Evidence:** [README.md](README.md) (consolidated findings) and the three audit reports
[A_paper_claims.md](A_paper_claims.md), [B_numerics.md](B_numerics.md), [C_lean_provenance.md](C_lean_provenance.md).

This notice retracts the fusion / ITER-disruption results listed below. No research document
or data file was deleted. Documents carry a banner that points here. Data files are listed in
`discoveries/README-RETRACTED.md` and their contents were not changed. Code that printed
fabricated verdicts was changed to print what it measures, or to say "NOT MEASURED".

## Why

The audit found that the numbers these documents report were not produced by any code in the
repository. They were hardcoded literals, `sleep`-manufactured timings, closed-form formulas
presented as benchmarks, or a closed-form trajectory presented as an MHD simulation. The Lean
"certificates" that were cited as formal backing are vacuous, or rest on axioms from which
`False` is derivable (kernel-checked in `proofs/lean4/audit_unsoundness_demo.lean`). Details and
file:line evidence are in README.md.

## Retracted

### Documents (banner added at the top of each)
| Document | What is retracted |
|---|---|
| `docs/Fusion_Disruptions_Scientific_Paper.md` (and the `.pdf`, which is the same text rendered; the PDF is not modified) | All numerical results: Tables 1-6, the conclusion table, the claim "All results are formally specified in Lean 4", and the deployed-API claim `POST /fusion/{1-4}`. No code in the repository produces them. The B-field (6.37 T) and tearing growth rate (~10³ s⁻¹) contradict the repository's own ITER data (5.3 T, 10.57 s⁻¹). |
| `paper/manuscript_v17.md` | The ~150× Neural-FGMRES speedup, the 0.9 ms FP8 figure, H100 execution, C-vs-Rust parity (150 vs 142 ms), the GNN preconditioner (45,000 parameters, ~2.5 GPU-hours, "frozen across 2000 steps"), the ~50-run break-even, Figures 2, 3, 4 and 8, and the description of Figures 5-7 as output of a "reduced-MHD proxy model" (they are renders of a prescribed closed-form trajectory). In the artifact manifest, the "Pre-trained preconditioner" `data/gnn_weights/` does not exist, the "Archived JSON benchmarks" in `data/fusion/poc_output/` are formula output, and `examples/iter_disruption.rs` is not a "168K DOF MHD simulation". The status "✅ Complete" for Theorems 1-2 is withdrawn: as stated they need ε‖A‖ < α, not ε < α. |
| `docs/Standard Operating Procedure (SOP)/Fusion Final Submission v2.md` | All results and "mechanized structural truths" (three of the five named theorems exist in no `.lean` file). The timing table repeats the fusion paper's timings under different method names. |
| `docs/Standard Operating Procedure (SOP)/Fusion Standard Operating Procedure (SOP).md` | The reproduction procedure and its "expected outputs". The directories, cargo features, `benchmark_*` binaries and `/execute_step` route it names do not exist in the repository. |

### Data (listed in `discoveries/README-RETRACTED.md`; contents unchanged)
- `discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json`: a fabricated execution log. It reports runs of four `cargo run --bin benchmark_*` targets that do not exist, at commit `9712004` or at HEAD. It was committed (`6f0c4ab`, 2026-05-14 16:47:19Z) 23 minutes before its own `timestamp_end` (17:10:07Z). Its verdict is "REPRODUCED, deviance 0.00%".
- `discoveries/phase3_fusion_telemetry.json`: written by `autoresearch_agent/iter_phase3_serverless.py`, which computed nothing. It slept 1 s per "protocol" and wrote `"status": "success"` unconditionally.

### Lean certificates (withdrawn)
- `CERT-FUS-FLAGNO-002` (`proofs/lean4/fusion_sop_flagno.lean`): the axiom `flagno_l4_telemetry_oracle` proves `False`, and the theorems are vacuous.
- `CERT-FUS-MONO-001` (`proofs/lean4/fusion_sop_monopole.lean`): the axiom `gcp_l4_telemetry_oracle` proves `False`. The file does not parse under Lean 4 (`constant`), and the theorems are vacuous or restate their hypotheses.
- `CERT-FUS-LSS-003` (`proofs/lean4/fusion_sop_lss_hdc.lean`): the theorems are vacuous or arithmetic on literals (report C §1.3). This file was not given a header in this branch.
- `CERT-FUS-110` (`mission-control/src/api/mockData.js`): it refers to a theorem and file that do not exist.

### Derived material (labelled in code, not deleted)
- `data/fusion/poc_output/v12_poc_results.json` holds synthetic curves from `scripts/reproduce_v12_poc.py`. The script and the `/api/peer_review/poc` endpoint now label them `"synthetic": true`. The committed JSON was not regenerated.
- `data/fusion/rust_sim_output/` and `rust_sim_output_3d/` hold a prescribed closed-form trajectory, not a simulation. The committed 2-D CSVs match the closed form to 2e-9 and were written on an 80×180 grid that the current code cannot produce (report B §4).
- The auto-research results served by `backend/main.py` (GPU ablation, adaptive precision, architecture comparison) and `/api/benchmarks` are now served as `"status": "not_measured"`. `backend/storage.json` was not edited.

## Publication metadata: owner action required (not edited in this branch)
`.zenodo.json` repeats retracted claims. The repository owner decides how to correct them:
- `.zenodo.json:3` (`description`): "All Lean 4 certificates are sorry-free with 0 undefined operators. Empirically validated at $0.04996 / 62.45s per exascale integration cycle on GCP L4 infrastructure (Execution ID: L4-SERV-88219-FUS)". It also cites CERT-FUS-FLAGNO-002, CERT-FUS-MONO-001 and CERT-FUS-LSS-003 as original innovations.
- `.zenodo.json:57` (`notes`): "Lean 4 formal proofs: sorry-free, 0 tautologies, 0 undefined operators. GCP telemetry: Execution ID L4-SERV-88219-FUS, $0.04996 / 62.45s … Frozen artifacts: discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json …".
- `NOTICE:63` also cites CERT-FUS-FLAGNO-002 for "FLAGNO … O(1) anisotropic XMHD preconditioning". The Lean statement contains no preconditioner.

If a Zenodo record was minted from this metadata, it should get a new version or a retraction note.

## Covered by this notice but not bannered in this branch
- `docs/Iter Autonomoua Phase 3.md`: every number in it comes from the hardcoded literals in `iter_phase3_serverless.py` (report A §8).
- `docs/Contrarian Peer Review Fusion Articel`, `paper/Fusion Iter - PEER REVIEW REPORT – ROUNd 1 .md`: these are not independent reviews. They were generated in the same pass as the rewrites, and neither checked code (report A §6).
- `docs/verification/PAPER_EXP3_FLAGNO.md`: inconsistent reruns with no log (report A §5).

## What would be needed to reinstate a claim
Each number needs a command that regenerates it from the repository, a raw log committed next to it, and a positive and a negative control. Each Lean statement needs a compile plus `#print axioms` with a footprint inside {propext, Classical.choice, Quot.sound} and no `sorryAx`.
