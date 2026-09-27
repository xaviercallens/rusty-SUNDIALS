# Fusion / ITER audit, 2026-09-27: consolidated findings and remediation

- **Branch:** `audit/fusion-2026-09-27` (worktree `rusty-SUNDIALS-wt-fusion-audit`).
- **Baseline audited:** HEAD `af4886f`. **Every file:line citation below refers to `af4886f`**, i.e. before this branch's edits. Banners and header comments added in this branch shift line numbers in the edited files. Use `git show af4886f:<path>` to check a citation.
- **Source reports:**
  - [A_paper_claims.md](A_paper_claims.md) (paper claims)
  - [B_numerics.md](B_numerics.md) (numerical runs and controls)
  - [C_lean_provenance.md](C_lean_provenance.md) (Lean and data provenance)
- **Retraction:** [RETRACTION_NOTICE.md](RETRACTION_NOTICE.md).

Every finding restated here was re-read at the cited lines during remediation, unless it is marked *(per report, not re-run)*. Measurements (timings, error tables, exit codes) come from report B's runs and are quoted unchanged. The remediation did not re-run iter_disruption or iter_disruption_3d (see "What remains").

**Verdict vocabulary.**
- **SUPPORTED:** repository code or data produces the claim.
- **UNSUPPORTED:** no producing code or output exists.
- **CONTRADICTED:** the repository's own code or data says otherwise.
- **FABRICATED:** a hardcoded literal, a `sleep`-simulated timing, or a formula output presented as a measurement. Report A calls this "FABRICATION-PATTERN".

Each claim table also gives the source report's original verdict verbatim. Hedged verdicts were not escalated.

---

## 1. Findings, severity-ranked

### Critical: results presented as measured that were not measured

| # | Finding | Evidence (at `af4886f`) | Found by |
|---|---|---|---|
| C1 | **The fusion SOP "reproduction log" is fabricated.** It gives results for `cargo run --release --bin benchmark_monopole_suppression / benchmark_flagno / benchmark_lss_shadowing / benchmark_hdc_trigger`. No such binaries exist, at the cited commit `9712004` or at HEAD (re-checked with `git ls-tree`). The file was committed in `6f0c4ab` (2026-05-14 18:47:19 +0200) before its own `timestamp_end` of 17:10:07Z. Verdict "REPRODUCED", deviance "0.00%". It is published as a "frozen artifact" in `.zenodo.json:57`. | `discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json:5, :8, :47, :54, :63, :71, :97-98` | A §8, C §2.1 |
| C2 | **The H100 / ~150× / 0.9 ms / C-vs-Rust / GNN numbers are literals.** The GPU ablation, adaptive-precision and architecture-comparison endpoints return hardcoded dicts or `if step < 8` schedules as `"status": "completed"` and persist them. `/api/benchmarks` returns constants (C 150 ms, Rust 142 ms, FP8 0.9 ms, 157.8×). Manuscript v17 reports these as measured. | `backend/main.py:156-186, :188-237, :239-286, :292-316`; `paper/manuscript_v17.md:19, :94, :99, :162, :196` | A §0.3, C §4 |
| C3 | **The "ITER disruption simulation" contains no physics.** The RHS never reads `y` and returns the analytic derivative of a prescribed trajectory, so ∂f/∂y ≡ 0. The FGMRES/Tensor-Core/FP8 lines are bare `println!`, and the linear-solver line is commented out. At n=168,000 the example died on a dense allocation at `af4886f` (rc=134 under an 8 GB cap; SIGKILL after 214.6 s uncapped). `crates/cvode` is being changed concurrently by another agent, so re-check this after that work lands. The 3D example solves nothing ("Bypass dense CVODE solve … Evaluate analytically"), and its GPU/precision/architecture flags do not change any of its 119 output CSVs (md5 identical). The committed 2D CSVs match the closed form to 2e-9, on an 80×180 grid that the current code cannot write. | `examples/iter_disruption.rs:99-132, :134-154, :141`; `examples/iter_disruption_3d.rs:106-160, :163-182, :249-250` | A §0.1-0.2, B §1-2 and §4, C §3 |
| C4 | **Speedups manufactured with `sleep`, and literal metrics.** `fusion_mhd_benchmark` sleeps 100 ms per step in one path and 2 ms in the other. Its 46-47× "MATHEMATICALLY VERIFIED" speedup drops to a ratio of 1.00 without the sleeps (B control C5). `fusion_sciml_phase5` prints `145x`, `99.9%` and FGMRES residuals as string literals. `exp4` fakes GPU latency with `sleep(1 ms)` and hardcodes `pass_control = true`. `tearing_mode_hero_test` prints literal residuals behind sleeps. | `examples/fusion_mhd_benchmark.rs:48-50, :81, :109-116`; `examples/fusion_sciml_phase5.rs:14, :35, :45, :52, :55, :65, :71-76, :89, :95`; `examples/exp4_ghost_sensitivities.rs:89, :158, :229`; `examples/tearing_mode_hero_test.rs:36, :60, :67, :70-72, :90` | A §2, B §2-3 |
| C5 | **The Lean "sorry-free certificates" contain inconsistent axioms.** Instantiating `flagno_l4_telemetry_oracle` at 0 gives `0 = 6`. Instantiating `gcp_l4_telemetry_oracle` at ε = −1 gives `|0| ≤ −1`. Both derivations are kernel-checked in this branch (§3). The remaining theorems are vacuous. `fusion_sop_monopole.lean` does not parse under Lean 4 (`constant`, :27-28). | `proofs/lean4/fusion_sop_flagno.lean:56-59`; `proofs/lean4/fusion_sop_monopole.lean:97-100` | C §1.3-1.4, A §8 |
| C6 | **Phase-3 "autoresearch" results are string literals.** Examples: 320,000×, 11.4 MW/m², 98.5%, 40 ns, 1,375×. The script runs `time.sleep(1.0)` ("Simulate API execution time"), prints "Integration converged", and writes `"status": "success"` and `"protocols_verified"` unconditionally. | `autoresearch_agent/iter_phase3_serverless.py:21-70, :78-81, :102-108`; `discoveries/phase3_fusion_telemetry.json` | A §8, C §2.2 |
| C7 | **Nothing in the repository produces the fusion paper's Tables 1-6.** The same-named examples solve unrelated toy problems: exp1 a 2-variable Van der Pol (`MU = 100`, :19; classifier discarded, :62), exp3 a 2-D 32×32 diffusion (:21-23), exp4 a pendulum. The named solver, LSODA, is not in the crate. A grep finds 2,448 / 3,133 / 492,096 / 6.37 / 52.5 µs only in documents. | `docs/Fusion_Disruptions_Scientific_Paper.md:53, :59-67, :135-142, :201-215, :278-303, :364-373` | A §0.4, §1, §8; B §4 |

### High

| # | Finding | Evidence | Found by |
|---|---|---|---|
| H1 | The paper's physics contradicts the repository's ITER data. The paper uses B = 6.37 T; the repository has 5.3 T. The paper gives γ_tearing ~ 10³ s⁻¹; the repository CSV has 10.57 s⁻¹. | paper `:11, :209, :23`; `data/fusion/iter_plasma_parameters.csv:2, :10` | A §0.5, §3; B §4 |
| H2 | A peer-review request for a PCIe benchmark and an FP8 residual study was "answered" with closed-form curves: `cpu=(dof/1000)**2*0.05`, and `fp8_res *= 0.78` then `*0.95 + sin(i)*1e-4`. These curves are served as a POC and listed in v17 as "Archived JSON benchmarks". | `scripts/reproduce_v12_poc.py:17, :44-48`; `backend/main.py:322-332`; `manuscript_v17.md:261` | A §6, C §3 |
| H3 | v17 Theorems 1-2 are false as stated. The step ⟨v, AEv⟩ ≤ ε‖v‖² needs ‖AE‖ ≤ ε, i.e. ε‖A‖ < α, but only ‖E‖ ≤ ε is assumed. Both are labelled "✅ Complete". Both Lean bodies are `sorry`. | `manuscript_v17.md:122-129, :150-151`; `proofs/NeuralFGMRES_Convergence.lean:75, :108` | A §4 |
| H4 | The fusion paper's inline Lean contains `sorry` (:342) and `True := trivial` (:254, :332). `field_alignment` relies on `cos(π/4) < 0.707`, which is false (0.70711 > 0.707) (:248). None of these blocks is in any `.lean` file. | paper `:237-254, :328-342` | A §0.6, B §4 (P) |
| H5 | The cvode crate cannot integrate smooth problems at rtol ≤ 1e-8 (`ErrTestFailure`), and convergence stalls near 1e-6. The solver allocates two dense n×n matrices, which makes large n impossible. *(Per report B; owned by another agent.)* | `crates/cvode/src/solver.rs:324, :347, :696-709, :723-728` | B §2-3 |
| H6 | exp5 "FoGNO" uses a diagonal A with the exact inverse as preconditioner, so it takes 1 iteration by construction. α was tuned "to hit <3 iters!" and the run then prints "VALIDATED". | `examples/exp5_fogno_xmhd.rs:25-30, :61-72, :104-105` | A §2, B §2-3 (C6) |
| H7 | The peer reviews are not independent. Each critique and its "publication-ready" rewrite came from the same pass, and nothing records a model, date or code check. The rewrites strengthen claims: "5 (projected)" becomes a measured result. | `docs/Contrarian Peer Review Fusion Articel`; `paper/Fusion Iter - PEER REVIEW REPORT – ROUNd 1 .md` | A §0.9, §6 |
| H8 | The mission-control client intercepts API paths and returns canned data. The Fusion SOP result is a canned "REPRODUCED / 0.00%". A theorem that does not exist is shown as "proved" (CERT-FUS-110). | `mission-control/src/api/client.js:13-56, :32`; `mission-control/src/api/mockData.js:179-185, :256-268` | C §1.5, §4 |

### Medium

| # | Finding | Evidence | Found by |
|---|---|---|---|
| M1 | η = 1e-8 Ω·m is hardcoded, about 44× the NRL Spitzer value at 25 keV (2.26e-10). S and γ are therefore off. | `scripts/fusion_data_integration.py:274, :317` | A §3, B §3 (P) |
| M2 | The FreeGS equilibrium is broken: `psi_norm = 24.0` at row 2, where it must lie in [0, 1]. The script swallows the solver exception ("Continuing with partial solution"). The Te/ne/j profiles are analytic parabolas. | `data/fusion/iter_equilibrium.csv:2`; `fusion_data_integration.py:147-149, :171-175` | A §3, C §3 |
| M3 | The header "Source: PlasmaPy" is contradicted by the file's contents. The `Ti_keV` row is missing, and v_A matches the manual fallback. | `data/fusion/iter_plasma_constants.rs:4` | A §3, C §3 |
| M4 | The viz script allocates 80×180 (`n_rho = 80`, `n_theta = 180`), while the Rust output is 200×400. It raises IndexError on fresh output, so the paper figures cannot have come from current code. | `scripts/iter_disruption_viz.py:50-51` | A §3, B §1 |
| M5 | exp3 prints "VALIDATED" on a regression (6933 vs 4628 RHS evaluations, −49.8%), with pass criterion `diff < 1.0` (:194). exp4 prints "VALIDATED" when Δ\|θ\| = −0.124 rad, i.e. worse. | `examples/exp3_flagno.rs:194`; exp4 output | B §1 |
| M6 | The prescribed trajectory is unphysical. Core Te reaches only ~5% of Te0 by t=1, the edge heats during the "quench", Ip is 6.03 MA (circular) or 10.25 MA (κ = 1.7) rather than 15 MA, and time has no units. | `examples/iter_disruption.rs:47, :109-120` | A §3, B §2 |
| M7 | The paper figures are built from formulas plus `np.random`. | `scripts/generate_paper_figures.py` (e.g. fig1 :51-84) | B §4 |
| M8 | No test covers the fusion physics. The `examples` crate has 0 tests. The only "assertion" in fusion_mhd_benchmark compares t with t. | `examples/fusion_mhd_benchmark.rs:109-113` | B §5 |

### Low

| # | Finding | Evidence | Found by |
|---|---|---|---|
| L1 | Suspected fabricated citations (*not verified*): Halpern et al. 2021 NF, and "ITER Organization … Technical Report 2023". | paper `:401`; `manuscript_v17.md:305` | A §4 |
| L2 | No lake project covers `proofs/lean4/`. The only library root is `theorem test : 1 + 1 = 2 := by sorry`. | `formal_proofs/RustySundialsProofs.lean:1-2`; `formal_proofs/lakefile.toml` | C §1.1 |
| L3 | Artifacts named in the manuscripts do not exist: `data/gnn_weights/` (re-checked: absent) and `vtk_output/iter_midplane.vtk`. | `manuscript_v17.md:258`; manuscript_v13:246-253 | A §5, §8 |

---

## 2. What was fixed in this branch

Nothing was deleted. Every research document and data file is still present.

| File | Change |
|---|---|
| `docs/audit/fusion-2026-09-27/README.md`, `RETRACTION_NOTICE.md` | New. Consolidated findings and the formal retraction. |
| `discoveries/README-RETRACTED.md` | New. Lists `fusion_sop_execution_L4-SERV-88219-FUS.json` and `phase3_fusion_telemetry.json` as retracted. The JSON contents are unchanged. |
| `docs/Fusion_Disruptions_Scientific_Paper.md`, `paper/manuscript_v17.md`, `docs/Standard Operating Procedure (SOP)/Fusion Final Submission v2.md`, `…/Fusion Standard Operating Procedure (SOP).md` | A retraction banner was added at the top (in v17, right after the YAML front matter). The body text is unchanged. |
| `proofs/lean4/fusion_sop_flagno.lean`, `fusion_sop_monopole.lean` | A header comment was added naming the unsound axiom and its line. The original content is unchanged. |
| `proofs/lean4/audit_unsoundness_demo.lean` | New. Restates both oracle axioms verbatim and derives `False` from each (§3). |
| `examples/fusion_mhd_benchmark.rs` | Both sleeps and the "MATHEMATICALLY VERIFIED" / "Exascale Target Met" lines were removed. The t==t assert was replaced by a real check of y(1) against e⁻¹. The printed timing ratio is now measured and expected to be ≈1. |
| `examples/fusion_sciml_phase5.rs` | All sleeps and literal metrics (145×, 99.9%, FGMRES residuals, "3 iterations") were removed. Items with no implementation print "NOT MEASURED — demo placeholder". The real ẏ = −y solves are reported as measured. |
| `examples/exp5_fogno_xmhd.rs` | "RHS evals" relabelled as GMRES iterations. The "VALIDATED" verdict and the α-tuning comment were removed. The run now states that 1 iteration is expected by construction, and it also reports the α = 0.5 case. |
| `examples/iter_disruption.rs` | Added a top-of-file doc comment: the RHS is prescribed, not a physics model, and a pointer to `examples/iter_current_quench_0d.rs`. The FLAGNO/FGMRES/Tensor-Core/FP8/"traversed extreme gradients" prints were removed, and a misleading code comment was corrected. |
| `examples/iter_disruption_3d.rs` | Added a top-of-file doc comment: closed-form evaluation with no solver, and the same pointer. The fake FNO/MPNN/H100/157×/FP8-schedule prints were removed. The env knobs are echoed with their real values and marked "no effect". The unused cvode import was removed. The output CSV format is unchanged. |
| `backend/main.py` | The three auto-research POST endpoints now return `"status": "not_measured"` with the same keys (numbers nulled) and no longer write `storage.json`. `GET /api/auto-research` relabels the three stored fake entries as `not_measured` when serving them; `storage.json` itself is not edited. `/api/benchmarks` keeps its keys, nulls the values and adds `"status": "not_measured"`. `/api/peer_review/poc` adds `"synthetic": true`. `/api/datasets` adds a `provenance` field. The API shape is kept so the UI does not crash (the UI uses optional chaining on these fields). |
| `autoresearch_agent/iter_phase3_serverless.py` | The sleep, "converged" and "success" outputs were removed. The claimed numbers are kept as `claimed_not_measured`. The script now writes `discoveries/phase3_fusion_placeholder_NOT_MEASURED.json` with `"status": "not_measured"`, so it can no longer overwrite the retracted JSON. |
| `scripts/reproduce_v12_poc.py` | Added a docstring explaining that the output is synthetic. The output now carries `"synthetic": true`. The committed JSON was not regenerated. |
| `mission-control/src/api/client.js`, `mockData.js` | The canned `/api/sop/execute` results for Fusion now read "RETRACTED / NOT MEASURED". For SOP-1, SOP-2 and SOP-3 they now read "NOT EXECUTED — canned client response". Those results used to be generated as "PASSED" at click time, and their logs were not audited, so they are marked not executed rather than retracted. The response `status` is `not_executed` except for PSC, which is outside this audit. In `mockData.js`, the Fusion history entry and the nonexistent "proved" theorem CERT-FUS-110 are marked retracted. |

---

## 3. Lean unsoundness demonstration (run in this branch)

Command:

```
cd /home/callensxavier_gmail_com/AutoevolveAI/formal && timeout 1500 lake env lean /home/callensxavier_gmail_com/rusty-SUNDIALS-wt-fusion-audit/proofs/lean4/audit_unsoundness_demo.lean
```

Result: rc = 0. Output:

```
'AuditDemo.FLAGNO.flagno_oracle_proves_false' depends on axioms: [propext,
 Quot.sound,
 AuditDemo.FLAGNO.flagno_l4_telemetry_oracle]
'AuditDemo.Monopole.monopole_oracle_proves_false' depends on axioms: [propext,
 Classical.choice,
 Quot.sound,
 AuditDemo.Monopole.gcp_l4_telemetry_oracle]
```

Both theorems have type `False`. Each footprint contains the corresponding oracle axiom and no `sorryAx`. The axioms are restated with the same binders and statements; only the namespace differs. The original files do not compile in this environment: `fusion_sop_flagno.lean` imports `Mathlib.LinearAlgebra.Matrix.Spectrum`, which is missing from the partial Mathlib build, and `fusion_sop_monopole.lean` uses Lean 3 `constant`. So the demo restates the axioms rather than importing those files (report C §1.2).

---

## 4. What remains (not done in this branch)

- **`.zenodo.json:3` and `:57`, and `NOTICE:63`.** These repeat the retracted claims: "sorry-free with 0 undefined operators", "$0.04996 / 62.45s", "Execution ID L4-SERV-88219-FUS", and the CERT-FUS-* innovations. They are publication metadata, so the owner decides. The claims are quoted in RETRACTION_NOTICE.md.
- **Other examples with the same patterns** (outside the assigned file list): exp1-exp4 and `tearing_mode_hero_test.rs`. They contain sleeps, `pass_control = true`, "VALIDATED" on regressions and literal residuals (C4, M5).
- **Other documents in scope of the retraction but not bannered:**
  - `docs/Iter Autonomoua Phase 3.md`
  - `docs/Contrarian Peer Review Fusion Articel`
  - `paper/Fusion Iter - PEER REVIEW REPORT – ROUNd 1 .md`
  - `docs/verification/PAPER_EXP3_FLAGNO.md`
  - `docs/Fusion_Disruptions_Scientific_Paper.pdf` (a binary copy of the .md)
- **`proofs/lean4/fusion_sop_lss_hdc.lean` (CERT-FUS-LSS-003).** Vacuous or arithmetic-on-literals, but not inconsistent, so it has no header. It is withdrawn in the notice.
- **`mission-control/src/api/mockData.js:334` (SOP-2).** Repeats the fabricated "FLAGNO=6 iters on 128³ … TFLOPs=115.2". The PSC/SOP-1/2/3 logs were outside this audit.
- **Physics/data pipeline:** Spitzer η (M1), fail-hard FreeGS (M2), the PlasmaPy header (M3), viz grid and figure regeneration (M4), `generate_paper_figures.py` (M7), Ip normalisation (M6).
- **The committed data outputs were not regenerated:** `data/fusion/rust_sim_output*`, `poc_output/`. Running `iter_disruption_3d` would rewrite 223 MB of tracked files. Running `iter_disruption` exhausts memory at n=168k (C3).
- **cvode tolerance failure and dense Jacobian (H5).** Owned by another agent.
- **A state-dependent replacement physics model:** `examples/iter_current_quench_0d.rs`, being written by another agent.
- **v17 Theorems 1-2 (H3):** the missing ‖A‖ hypothesis has not been fixed in the manuscript or the Lean file.
- **Citations (L1):** not resolved.

---

## 5. Claim status: `docs/Fusion_Disruptions_Scientific_Paper.md`

| # | Headline claim | Location | Status | Report verdict (verbatim) and basis |
|---|---|---|---|---|
| 1 | Dynamic IMEX cuts Jacobian condition 10,000 → 2,003 (5.0×) | :11, :64-66, :385 | UNSUPPORTED | A#1 "UNSUPPORTED / FABRICATION-PATTERN (the Lean 'proof' of an input constant)". exp1 is a 2-variable Van der Pol with the classifier discarded. |
| 2 | 64 modes, 53 implicit / 11 explicit | :50, :62-63 | UNSUPPORTED | A#2 "UNSUPPORTED" |
| 3 | 2,448 vs 3,133 function evaluations (LSODA) | :53, :61 | UNSUPPORTED | A#3 "UNSUPPORTED". LSODA is not in the crate. |
| 4 | Magnetic energy 0.442; 10 ms of plasma evolution | :52, :67 | UNSUPPORTED | A#4, A#5 "UNSUPPORTED" |
| 5 | LSI²: 2000 → 64 dimensions, 977× Jacobian compression, 30.5 MB → 32 KB | :11, :139-141 | UNSUPPORTED | A#6 "UNSUPPORTED (numbers are arithmetic, not an experiment)" |
| 6 | "Orthogonal neural autoencoder" | :119 | CONTRADICTED | A#8 "CONTRADICTED". exp2 uses an analytic sine basis. |
| 7 | BDF evaluations 108/108 | :142 | UNSUPPORTED | A#7 "UNSUPPORTED" |
| 8 | FLAGNO on an ITER-scale 32×32×32 grid (32,768 cells) | :11, :205-206 | CONTRADICTED | A#9 "CONTRADICTED". exp3 is 2-D 32×32, with no graph and no GNN. |
| 9 | 492,096 field-aligned edges | :208, :387 | UNSUPPORTED | A#10 "UNSUPPORTED" |
| 10 | B = 6.37 T (ITER) | :11, :209 | CONTRADICTED | A#11 "CONTRADICTED (B0)". The repository CSV has 5.3 T. |
| 11 | FGMRES 99 → 5 iterations | :212-213 | UNSUPPORTED | A#12 "UNSUPPORTED (the '5' is a projection)" |
| 12 | FP8 Tensor Core precision | :197, :214 | UNSUPPORTED | A#13 "UNSUPPORTED" |
| 13 | "Lean 4 rejection guarantee ✅" | :215, :237-254 | FABRICATED | A#14 "FABRICATION-PATTERN (verification-washing)". Tautology plus `True := trivial`. |
| 14 | Ghost sensitivities: 128 states × 8 coils = 1,152-dim system | :282-284 | UNSUPPORTED | A#15 "UNSUPPORTED". exp4 is a 2-state pendulum. |
| 15 | CPU 52.5 µs vs GPU FP8 1.1 µs, "50× async speedup" | :286-288, :388 | FABRICATED | A#16 "FABRICATION-PATTERN". No GPU code; `sleep(1 ms)` fakes latency. |
| 16 | Tearing-mode energy 6.39e-5; per-coil sensitivities | :290, :294-303 | UNSUPPORTED | A#17 "UNSUPPORTED" |
| 17 | "Checkpointing required: No" / "zero checkpointing" as a result | :289, :388 | SUPPORTED (trivially; not a finding) | A#18 "Mis-framed (true but not a finding)". Forward sensitivities never need checkpoints. |
| 18 | Timings D1-D4 1.02/8.33/6.64/1.88 s, total 17.87 s, $0.05 | :366-373 | UNSUPPORTED | A#20 "UNSUPPORTED". No run log exists. |
| 19 | Combined budget $0.20 of $100 | :375 | UNSUPPORTED | A#21 "UNSUPPORTED" |
| 20 | "All results are formally specified in Lean 4 … precision bounds" | :390 | CONTRADICTED | A#22 "CONTRADICTED". `sorry` at :342, `True := trivial` at :254 and :332, false lemma at :248. |
| 21 | `compression_ratio`: 2000²/64² = 976 | :178-180 | SUPPORTED (trivial arithmetic) | A#23 "Trivial" |
| 22 | Tearing growth rate γ ~ 10³ s⁻¹ | :23 | CONTRADICTED | A#25 "CONTRADICTED (internal)". The CSV has 10.57 s⁻¹. |
| 23 | Deployed API `POST /fusion/{1-4}` | :406 | UNSUPPORTED | A#26 "UNSUPPORTED". There is no `/fusion/` route. |
| 24 | Background ITER scales (N ~ 10⁹, anisotropy ~ 10⁹, v_A ~ 10⁶ m/s) | :21-29 | not a result | A#24 "Plausible (not a result)" |
| 25 | Ref [6] Halpern et al., Nucl. Fusion 2021 | :401 | UNSUPPORTED (suspected fabricated, not verified) | A#28 "SUSPECTED FABRICATED CITATION (not verified)" |

## 6. Claim status: `paper/manuscript_v17.md`

| # | Headline claim | Location | Status | Report verdict and basis |
|---|---|---|---|---|
| 1 | Neural-FGMRES gives a ~150× speedup over sparse ILU-GMRES | :19, :162, :169, :196, :289 | FABRICATED | A §0.3, §5 "FABRICATION-PATTERN / CONTRADICTED". 157.8× is a literal at `backend/main.py:172, :309`. |
| 2 | 0.9 ms FP8 inference latency | :99 | FABRICATED | Literal at `backend/main.py:172, :298, :306` (A §0.3) |
| 3 | FGMRES preconditioner executed on H100 FP8 Tensor Cores | :19, :34, :54, :81 | UNSUPPORTED | No GPU/FP8 code in the repository (A §1#13, B §2) |
| 4 | Rust/C-SUNDIALS parity at 168K DOF (150 vs 142 ms, Sparse ILU) | :19, :33, :162 | FABRICATED | A §5 "C-vs-Rust '150 vs 142 ms' are constants (main.py:296-297)". B §4 "Unsupported": no ILU and no C comparison in the code. |
| 5 | "Empirically demonstrate execution of a 168,000 DOF 2D reduced-MHD proxy model"; manifest "168K DOF MHD simulation" | :19, :256 | CONTRADICTED | The RHS does not depend on y (A §0.1). The example fails on allocation at that size (B §1). The "2000 time steps" (:97, :101) contradict the 7 output times. |
| 6 | 3-layer MPNN, 45,000 parameters, ~2.5 GPU-h training, frozen weights | :89-97, :101, :196 | FABRICATED | Literals at `backend/main.py:251-255`. No training code, and `data/gnn_weights/` does not exist (A §5). |
| 7 | Figures 2, 3, 4 (C-vs-Rust, PCIe scaling, Newton convergence) | :161-176 | UNSUPPORTED | B §4: "No generator … was found in any repo `.py` file; other file types were not searched" |
| 8 | Figures 5-7 show the reduced-MHD proxy model output | :178-189 | CONTRADICTED | They render the closed form. The committed CSVs match it to 2e-9 on a grid the code cannot produce (B §4, A §3). |
| 9 | Figure 8 relative cost 0.086× / 0.013× | :193-194 | FABRICATED | Literals at `backend/main.py:311-315` (A §7#1) |
| 10 | Break-even ≈ 50 runs | :196 | UNSUPPORTED | Derived from the fabricated speedup and training cost |
| 11 | Theorems 1-2: "✅ Complete" pen-and-paper proofs | :122-151 | CONTRADICTED | A §4: false in general as stated (needs ε‖A‖ < α). The Lean bodies are `sorry`. |
| 12 | Artifact manifest: "GNN weights `data/gnn_weights/`" | :258 | CONTRADICTED | The directory does not exist (re-checked) |
| 13 | Artifact manifest: "Archived JSON benchmarks `data/fusion/poc_output/`" | :261 | FABRICATED | Closed-form formula output from `scripts/reproduce_v12_poc.py` (A §2, C §3) |
| 14 | GPU-native cuSPARSE ablation "planned as future work" | :169, :273 | SUPPORTED (as a statement of future work) | The manuscript is honest here. But `backend/main.py:156-186` serves the same ablation as "completed" with invented numbers (A §6), so the repository contradicts itself. |
| 15 | Ref [5] "ITER Organization … ITER Technical Report, 2023" | :305 | UNSUPPORTED (looks invented; not verified) | A §4 |
| 16 | Validated against 33 canonical ODE benchmarks within machine epsilon of C; crate LOC table | :44-51 | not audited | Outside the scope of the three reports |
