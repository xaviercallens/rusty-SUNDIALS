# Fusion audit C: Lean proofs and data provenance

Date: 2026-09-27. Worktree: /home/callensxavier_gmail_com/rusty-SUNDIALS-wt-fusion-audit (branch audit/fusion-2026-09-27, HEAD af4886f). I edited or committed nothing tracked. Scratch copies, scripts and raw tool output are in C_scratch/ (`lean/`, `out/`).

**What ran and what didn't.**
- `C_scratch/run_all.sh` ran: Lean compiles, greps and git log.
- A follow-up script did not run. It held the substituted-import compiles, the in-file False probes, `git log --follow`, the `git ls-tree 9712004` check, and a Python reproduction of `iter_plasma_parameters.csv`. The Claude Code auto-mode classifier denied it ("[Auto-Mode Bypass]"), and I did not retry.
- Items that depended on that script are marked **UNVERIFIED** below.

---

## 1. Lean

### 1.1 Project and toolchain
- No lake project covers `proofs/lean4/`. The only lakefile in the repo is `formal_proofs/lakefile.toml` (from `find`, out/env.txt:11-12).
- That lakefile's `lean_lib` is `RustySundialsProofs` (lakefile.toml:16-17).
  - It requires only `repl`. There is no mathlib `[[require]]` (lakefile.toml:12-14).
  - It pins `leanprover/lean4:v4.30.0-rc2` (formal_proofs/lean-toolchain:1). That toolchain is not installed here: `~/.elan/toolchains` holds v4.31.0 through v4.34.0.
  - `formal_proofs/.lake` does not exist (out/env.txt:10), so it has never been built here.
- The library's only root, `formal_proofs/RustySundialsProofs.lean:1-2`, is `theorem test : 1 + 1 = 2 := by sorry`.
- `formal_proofs/RustySundials.lean` imports Mathlib, which the lakefile does not provide.
- The SOP (`docs/Standard Operating Procedure (SOP)/Fusion Standard Operating Procedure (SOP).md:73-76`) says to run `cd rusty-SUNDIALS/lean_proofs && lake build`. There is no lakefile anywhere except formal_proofs/. Its "expected output" (lines 84-89) is not output from any file in the repo.
- Compiles were done in the AutoevolveAI env: `cd AutoevolveAI/formal && lake env lean <copy>`, with Lean v4.34.0-rc2 and a partial Mathlib build.
  - Two imports used by the fusion files are **missing** from that partial build: `Mathlib.LinearAlgebra.Matrix.Spectrum` and `Mathlib.Dynamics.Ergodic.Basic` (out/env.txt:14,16).

### 1.2 Per-file results (compile rc from out/lean_*.txt)

| File | rc | Cause |
|---|---|---|
| fusion_sop_flagno.lean | 1 | `object file ... Mathlib/LinearAlgebra/Matrix/Spectrum.olean does not exist`. Environment failure, not a proof failure. A substituted-import compile was BLOCKED. |
| fusion_sop_lss_hdc.lean | 1 | `... Mathlib/Dynamics/Ergodic/Basic.olean does not exist`. Environment failure. The same happened with `λ_max` renamed to `lam_max`. Whether `λ_max` even parses in Lean 4 is UNVERIFIED. |
| fusion_sop_monopole.lean (as written) | 1 | `27:0: error: unexpected identifier; expected command`. `constant` is Lean 3 syntax and is not a Lean 4 command. Every use of `div`/`curl` then fails (errors at lines 34, 47, 48, 68, 70, 71). A **genuine file failure.** Separately, `19:22 invalid binder annotation`: `[MeasurableSpace Ω]` is not imported. |
| monopole, `constant`→`opaque` port (scratch only) | 1 | Only the line-19 binder error remains. The theorems elaborate, and the compiler warns `68:5: Variable name h_coulomb is not explicitly referenced`. |
| formal_proofs/RustySundials.lean | 1 | `15:57 unexpected token 'variable'; expected 'lemma'`, because the docstring sits before `variable`. `magnetic_monopole_free_update` depends on `[propext, sorryAx, Classical.choice, Quot.sound]`. |

### 1.3 Axioms and verdict per theorem

| Theorem (file:line) | `#print axioms` | Statement | Verdict |
|---|---|---|---|
| `flagno_o1_weak_scaling` (flagno:26-33) | not obtainable (env) | `∀ n>0, ∃ C ≤ 7, ∀ g ≤ n, C ≤ 7`. The conclusion repeats the hypothesis. No solver, grid or anisotropy appears. | TRIVIAL-ARITHMETIC / VACUOUS; DOES-NOT-COMPILE here (env) |
| `cartesian_amg_fails_under_extreme_anisotropy` (flagno:38-40) | n/a | `∃ b : Bool, b = true`. `h_extreme` is unused. | VACUOUS |
| `flagno_beats_amg` (flagno:45-49) | n/a | `(∃ i:ℕ, i ≤ 7) ∧ (∃ b, b = true)` | VACUOUS |
| axiom `flagno_l4_telemetry_oracle` (flagno:56-59) | n/a | `∀ iters_measured : ℕ, True → iters_measured = 6 ∧ …` | **INCONSISTENT AXIOM**. See 1.4. |
| `lss_shadowing_adjoint_horizon` (lss:16-22) | env | Picks δ := exp(-λT) and proves δ ≤ δ with `le_refl`. `h_window` and `h_lyap` are unused. It says nothing about shadowing or adjoints. | VACUOUS |
| `async_decoupling_correctness` (lss:26-32) | env | `τ_gpu = 1.18e-6 → τ_cpu = 5.18e-5 → τ_gpu < τ_cpu`. A comparison of two literals. | TRIVIAL-ARITHMETIC |
| `hdc_trigger_latency_bound` (lss:36-44) | env | `10000/64·0.25 ≤ 40`, which is 39.0625 ≤ 40. | TRIVIAL-ARITHMETIC |
| `discrete_de_rham_exactness` (mono:45-51) | opaque port: `[propext, Classical.choice, Quot.sound, FusionSOP.Monopole.div_curl_eq_zero]` | div∘curl = 0 over opaque `div`/`curl` (mono:27-28), taken as axiom `div_curl_eq_zero` (mono:34). Nothing about a Yee grid is modelled. | DOES-NOT-COMPILE (as written); DEPENDS-ON-AXIOM (port) |
| `gauge_invariant_latent_bijection` (mono:64-78) | same as above | Identical to the previous theorem after `rw`. `h_coulomb` is unused (compiler warning). The comment at 59-63 says the fix "threads h_coulomb", but it does not. No bijection is stated. | DOES-NOT-COMPILE; DEPENDS-ON-AXIOM + VACUOUS |
| axiom `gcp_l4_telemetry_oracle` (mono:97-100) | n/a | `∀ div_B_sim ε, True → ∀ x, |div_B_sim x| ≤ ε` | **INCONSISTENT AXIOM**. See 1.4. |
| `monopole_suppression_bound` (mono:106-115) | `[propext, Classical.choice, Quot.sound]` (clean) | Hypotheses `h_eps : ε = 1.12e-15` and `h_telemetry : ∀x, |f x| ≤ ε` give `∀x, |f x| ≤ 1.12e-15`. This is P → P. | VACUOUS; the clean axioms do not make it meaningful. DOES-NOT-COMPILE (file) |
| `RustySundials.magnetic_monopole_free_update` (formal_proofs/RustySundials.lean:30-32) | includes `sorryAx` | `is_divergence_free := True` (line 25-27), then proved by `sorry`. | VACUOUS + sorry |
| `FoGNO.fogno_fgmres_convergence` (proofs/lean4/fogno_fgmres_convergence.lean:24-26) | not compiled | Uses `P_fogno := sorry` (line 21), and the theorem is `sorry`. | sorry (not meaningful) |

**Meaningful theorems found: 0.**

### 1.4 Both "oracle" axioms prove False (tool output)
- `C_scratch/probe_false_flagno.lean` (core Lean): the axiom restated, instantiated at 0, then `omega`.
  - Output: `'flagno_oracle_proves_false' depends on axioms: [flagno_l4_telemetry_oracle, propext, Quot.sound]`, rc=0.
- `C_scratch/probe_false_monopole.lean`: the same idea with ε = -1, then `abs_nonneg` + `linarith`.
  - Output: `'monopole_oracle_proves_false' depends on axioms: [gcp_l4_telemetry_oracle, propext, Classical.choice, Quot.sound]`, rc=0.
- These probes restate the axioms with the same binder types. The versions that append the probe to the repo files themselves were BLOCKED.
- No theorem actually uses either oracle, so they are decoration. Still, the docstrings describe them as keeping the certificate "sorry-free" (flagno:12,55; mono:14,95-96). Any file that imported these modules could prove anything.

### 1.5 What the docs claim versus what exists
- `docs/.../Fusion Final Submission v2.md:43-47` lists 5 "mechanized structural truths".
  - `dynamic_imex_truncation_error_bound`, `shadowing_adjoint_sensitivity_horizon` and `fp8_krylov_subspace_convergence` **do not exist in any .lean file**. A grep found them only in docs (out/theorem_name_refs.txt:6-20).
  - The two that do exist are the vacuous or axiom-dependent monopole theorems.
- `.zenodo.json:3` says "All Lean 4 certificates are sorry-free with 0 undefined operators". That is false:
  - `div`/`curl` are opaque constants.
  - There are 3 axioms.
  - fogno and RustySundials use `sorry`.
- `NOTICE:63` and `.zenodo.json:3` cite CERT-FUS-FLAGNO-002 as establishing "FLAGNO O(1) anisotropic preconditioning". The Lean statement contains no preconditioner.
- `docs/Reproducibility_and_Artifact_Evaluation.md:54-56` pins the certificates to commit `7166890`. Not checked (git follow-up BLOCKED).
- `mission-control/src/api/mockData.js:179-185` shows `iter_gyrokinetic_stability` in `iter_phase3.lean` as status "proved", CERT-FUS-110. No such theorem or file turned up in the grep of proofs/, formal_proofs/ or lean/ (out/lean_fusion_files.txt).

---

## 2. Provenance: discoveries/*.json
There are only two fusion files in discoveries/ (out/discoveries_ls.txt).

### 2.1 discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json: **fabricated**
- **No producer.** The id `L4-SERV-88219` and the filename appear only in the JSON itself, the Lean docstrings, `mission-control/src/api/mockData.js:257,291` and `.zenodo.json:3`. No script, Rust or Python, writes this file (out/execid_refs.txt, out/discoveries_producers.txt:1-3).
- **The benchmark binaries do not exist.** The JSON gives the commands `cargo run --release --bin benchmark_monopole_suppression|benchmark_flagno|benchmark_lss_shadowing|benchmark_hdc_trigger` (lines 47, 54, 63, 71).
  - None of those names appears in any `.rs` or `Cargo.toml`.
  - There is no `src/bin/` (out/bins.txt: `ls: cannot access 'src/bin'`).
  - The SOP's `core/` directory and the cargo features `fp8_tensor_cores` and `async_adjoints` are also unverified (check BLOCKED).
- **It was committed before the run it reports had finished.**
  - The file was added in commit `6f0c4ab` at `2026-05-14 18:47:19 +0200`, which is 16:47:19Z (out/discoveries_producers.txt:2).
  - The JSON says `timestamp_end: 2026-05-14T17:10:07Z` (line 8), 23 minutes later.
  - The claimed `git_commit: 9712004` (line 5) exists: "feat: add SOP reproducibility page and execution APIs", 18:32:15 +0200 (out/commit_9712004.txt).
- **Lean verification cannot have happened as described.**
  - `lean4_version v4.16.0`, `errors: 0` (lines 23, 37). But `fusion_sop_monopole.lean` fails to parse under Lean 4 (`constant`, 1.2).
  - It lists 6 theorems "checked", while the docs name 3 theorems that exist nowhere.
- **Internal arithmetic does not add up.**
  - Wall clock 16:45:12 to 17:10:07 is 1495 s, but `total_execution_time_s: 62.45` (line 79).
  - 62.45 × 0.00072 + 0.00011 = 0.045074, but `total_execution_cost_usd: 0.04996` (lines 79-82).
  - "GCP Cloud Run" with 8 vCPU / 32 GB (lines 10-13) conflicts with `instance_type: g2-standard-16` (line 17), which is 16 vCPU / 64 GB.
  - HDC `latency_ns: 38.5` (line 73) versus 39.06 from the Lean formula.
  - CPU step 51.8 µs (line 64) versus 52.5 µs in the article (Final Submission v2.md:37) and the SOP (:141).
- **Implausible claims.** `fp8_tflops: 115.2` and `tensor_core_utilization_pct: 98.4` (lines 58-59) for a 6-iteration FGMRES. Also `deviance_from_baseline: "0.00%"` on every metric.
- `mission-control/src/api/client.js:32` hard-codes the same numbers ("$0.04996 Total, div(B)=1.12e-15, FLAGNO=6", "REPRODUCED", "24m 55s").

### 2.2 discoveries/phase3_fusion_telemetry.json: **hardcoded**
- Written by `autoresearch_agent/iter_phase3_serverless.py:102-108`.
- Each protocol's results are literal strings in the script (lines 21-70), for example "46.2 Megabytes (from 14.8 Terabytes)", "320,000x", "11.4 MW/m²" and "1,375x over TensorRT GPU".
- The script does no computation. `time.sleep(1.0)` sits under the comment "Simulate API execution time" (lines 78-79), then it prints "[✔] Integration converged." (line 81).
- `cost_euros 0.0123244` = (14.2 + 8.5 + 11.3) × 0.000361 + 2.1 × 0.000024, using the hardcoded `sim_time` values. `execution_time_s 36.1` is their sum. Nothing is measured.
- "status": "success" and "protocols_verified" are written unconditionally.

---

## 3. data/fusion/

| Path | Producer | Nature |
|---|---|---|
| `iter_plasma_parameters.csv`, `iter_plasma_constants.rs` | `scripts/fusion_data_integration.py:342-378` | Textbook formulas over round ITER design values: B0 = 5.3, n = 1e20, L = a = 2 m, η = 1e-8 (assumed, `:274,317`). Not a dataset. |
| `iter_equilibrium.csv` | `fusion_data_integration.py:71-209` (FreeGS) | FreeGS ψ with 6 invented coil positions (`:106-113`), plus analytic Te/ne/j parabolas (`:173-175`). |
| `rust_sim_output/` (7 CSV, 7.3 MB) | `examples/iter_disruption.rs:149-171` | CVODE integrates a **prescribed analytic trajectory**. |
| `rust_sim_output_3d/` (119 files, 223 MB) | `examples/iter_disruption_3d.rs:141-208` | **No solver at all**: closed-form evaluation. |
| `vtk_output/`, `vtk_output_3d/` | `scripts/iter_disruption_viz.py:18,64`, `iter_disruption_3d_viz.py:20-21`, `iter_disruption_3d_video*.py`, `export_imas_paraview.py:33` | Renders of the above. |
| `poc_output/v12_poc_results.json` | `scripts/reproduce_v12_poc.py:7-66` | **Synthetic formulas** labelled as benchmarks. |

### iter_plasma_parameters.csv and iter_plasma_constants.rs
- **The "Source: PlasmaPy" header (.rs:4) is contradicted by the file's contents.**
  - The PlasmaPy path returns `Ti_keV` (`:297`), and the CSV and .rs have no `Ti_keV` row.
  - v_A = 8.180836e6 matches the manual fallback `B0/sqrt(mu0·n·3.34e-27)` (`:315-319`).
  - So the values came from `compute_iter_parameters_manual`, the "hardcoded" fallback (`:233-234`).
  - This rests on my own arithmetic. The scripted reproduction was BLOCKED.
- τ_A·v_A = 2.000 m, τ_R = μ0a²/η = 502.65 s, S = τ_R/τ_A, and γ = S^(-3/5)/τ_A are all consistent.
- Te is listed but not used in any derived quantity.

### iter_equilibrium.csv
- ψ rises monotonically from 10.66 to 35.00 across R = 4.2 to 8.2 m (rows 2-101). There is no magnetic axis inside the plasma chord.
- `psi_norm` runs 24.0 down to 1.0 (row 2 to row 101), although it should be in [0, 1]. This suggests the solve failed or the axis/boundary normalisation is wrong.
- The script swallows solver exceptions and writes anyway ("Continuing with partial solution", `:147-149`).
- Te/ne/j_phi are analytic, symmetric in ρ, and independent of ψ.
- `mission-control/src/api/datasetsMockData.js:115` describes this file as "FreeGS solver → ψ(R) midplane profile".

### rust_sim_output/ and rust_sim_output_3d/
- **rust_sim_output/.** The RHS closure never reads `y`.
  - `ydot` is the analytic time derivative of a prescribed trajectory ("Evaluate derivatives analytically to match python model perfectly", `iter_disruption.rs:99-129`). So CVODE only integrates dy/dt = f(t).
  - There is no MHD, no spatial coupling and no tearing dynamics. The "island" is a fixed Gaussian times cos 2θ with width 0.05 + 0.35t (`:43-44,108`).
  - The printed lines "Injecting AI-preconditioned FGMRES (FLAGNO)" and "Offloading to Tensor Cores… FP8" (`:134-137,151-154`) are bare `println!`. The FGMRES option is commented out (`:141`).
- **rust_sim_output_3d/.** "Bypass dense CVODE solve to avoid OOM. Evaluate analytically." (`iter_disruption_3d.rs:163-181`).
  - The "FNO/DeepONet/MPNN weights" and "Offloading SpMV to H100 Tensor Cores (Target: 157x)" messages are prints gated on environment variables (`:106-136`).
  - The "adaptive precision" is `exp(-5t)` compared against thresholds and then printed (`:147-160`).
  - `backend/main.py:51-63,118-127` advertises this output as "ITER 3D Toroidal Disruption (672K DOF)".
- These outputs are deterministic in principle, since everything is closed form. Regeneration was not attempted: `cargo run` would write into tracked `data/fusion`, and bash was restricted.

### poc_output/v12_poc_results.json
- Hand-written scaling formulas: `cpu_time=(dof/1000)^2*0.05`, and an FP8 residual `*=0.78`, switching to `*=0.95 + sin(it)*1e-4` "Add floating point noise" (`reproduce_v12_poc.py:15-54`). No GPU or solver is involved.
- It is deterministic apart from `timestamp`.

### git history
- `git log -- data/fusion` shows commit `89d4efc` of 2026-05-17, "feat(v11): Polish 3D torus video and ITER Education Kit". It adds the poc JSON, the rust_sim_output CSVs and the frames (out/data_fusion.txt:98-217).
- Full per-path history is UNVERIFIED (git follow-up BLOCKED).
- **Real public datasets: none present.** MAST, disruption-py, JOREK and IMAS appear only as catalogue text in `mission-control/src/api/datasetsMockData.js`.

---

## 4. Mission-control UI and backend
- **`mission-control/src/api/client.js:13-56` intercepts API paths before any network call** and returns canned data:
  - `/api/results`, `/api/report`, `/api/verification`, `/api/verify` and `/api/sop` return `MOCK_*`.
  - `/api/sop/execute` returns fixed results (`:29-33`). Fusion gets "REPRODUCED", "0.00%" deviance and "24m 55s", with `execution_id: EXEC-${Math.random()}` and `timestamp: new Date()` (`:35-37`).
  - `/api/datasets*` returns `datasetsMockData`.
- `mockData.js:217-303` is the Fusion SOP history entry (execution id, commit, verdict "REPRODUCED"), pointing at the artifacts above. `mockData.js:179-185` is the nonexistent "proved" theorem.
- **backend/main.py.**
  - `load_db()` default (`:36-68`) and `storage.json` serve static visualization metadata.
  - `/api/datasets` (`:102-129`) counts real CSV files but hard-codes `dof`, `grid` and the names.
  - `/api/peer_review/poc` (`:322-332`) runs `reproduce_v12_poc.py` and returns its synthetic JSON as a peer-review POC.
  - `/api/health` hard-codes `dof_2d/3d` (`:338-346`).
  - The response literals near `:300-315` ("speedup_neural", "gpu_ablation 157.8x", "h100_tensor_core") look hardcoded. I did not trace their full endpoint.

---

## 5. Prioritized top-10 fixes

| # | File | Change | Verification |
|---|---|---|---|
| 1 | `proofs/lean4/fusion_sop_flagno.lean:56-59`, `fusion_sop_monopole.lean:97-100` | Delete both "oracle" axioms. Empirical numbers belong in data, not in Lean axioms. | `grep -n '^axiom' proofs/lean4/fusion_sop_*.lean` is empty; the probes in C_scratch no longer elaborate. |
| 2 | `discoveries/fusion_sop_execution_L4-SERV-88219-FUS.json`, `mission-control/src/api/client.js:32`, `mockData.js:217-303`, `.zenodo.json:3`, `NOTICE:63` | Retract: the run has no producer, its binaries don't exist, and it was committed before its own end timestamp. Remove "REPRODUCED", "sorry-free" and the $0.04996 claims. | `grep -rn 'L4-SERV-88219\|0.04996\|sorry-free' .` returns only a retraction note. |
| 3 | `formal_proofs/lakefile.toml`, `lean-toolchain` | Add `require mathlib` and pin an installed toolchain. Add a `lean_lib` covering `proofs/lean4/Fusion*`. Delete `RustySundialsProofs.lean` (`1+1=2 := sorry`). | `lake build` rc=0, then `#print axioms` for every theorem ⊆ {propext, Classical.choice, Quot.sound}, run through a gate like AutoevolveAI `anse/formal/lean_runner.py`. |
| 4 | `fusion_sop_monopole.lean:27-34` | Port `constant`→`opaque` and import `MeasurableSpace`. Better, define discrete div/curl concretely on a finite Yee grid and **prove** div∘curl = 0 instead of assuming it. | Compile rc=0; `discrete_de_rham_exactness` axioms show no `div_curl_eq_zero`. |
| 5 | `fusion_sop_flagno.lean:26-49`, `fusion_sop_lss_hdc.lean` | Relabel as "numeric constants check" or delete. Restate FLAGNO as a real bound (e.g. an iteration count as a function of κ(P⁻¹A)), or drop the O(1) claim. | Human statement review: every hypothesis is used (no unused-variable warnings) and the conclusion is not a hypothesis. |
| 6 | `docs/.../Fusion Final Submission v2.md:43-47`, `SOP.md:73-89` | Remove the 3 theorem names that don't exist. Replace the invented "expected output" with real `lake build` output. Fix the `lean_proofs/` path. | Every theorem name in the docs is found by `grep` in a .lean file compiled by the lake target. |
| 7 | `examples/iter_disruption.rs:99-154`, `iter_disruption_3d.rs:106-181` | State plainly that these are prescribed-trajectory / analytic proxies. Remove the FLAGNO / Tensor Core / FNO / H100 / FP8 prints. Rename the output from "simulation". | `grep -n 'Tensor Cores\|FLAGNO\|H100\|FP8' examples/iter_disruption*.rs` is empty. |
| 8 | `backend/main.py:36-63,102-129,322-346`, `mission-control/src/api/client.js:13-56` | Remove the client-side mock intercept, or put a visible "DEMO DATA" banner on it. Label POC and dataset responses as synthetic, or derive them from real files. | Network tab shows real backend calls; the UI shows no "REPRODUCED" without a real artifact behind it. |
| 9 | `scripts/fusion_data_integration.py:147-149,233-234`, `data/fusion/iter_plasma_constants.rs:4`, `iter_equilibrium.csv` | Fail hard when FreeGS does not converge; check that psi_norm ∈ [0,1] and that an axis exists. Record which path ran (PlasmaPy or manual) in the header. Use real ITER PF coils, or label the equilibrium a toy. | Rerun into a scratch dir; `min(psi_norm) ≥ 0 and max ≤ 1`; the header matches the path taken; add a positive and a negative control. |
| 10 | `autoresearch_agent/iter_phase3_serverless.py`, `discoveries/phase3_fusion_telemetry.json`, `scripts/reproduce_v12_poc.py` | Delete them, or rename as `*_mock` with `"synthetic": true` in the output. Stop writing `"status": "success"` / "protocols_verified" for hardcoded strings. | `grep -rn 'Simulate API execution time\|Add floating point noise' .` is empty or confined to files marked mock. |

---

## Blocked / unverified
- Substituted-import compiles of flagno and lss_hdc.
- In-file False probes, i.e. against the repo modules rather than restatements.
- Whether `λ_max` parses.
- `git log --follow` for the Lean files and JSON.
- `git ls-tree 9712004` for benchmark binaries at that commit.
- Existence of `lean_proofs/` and `core/`.
- Scripted reproduction of `iter_plasma_parameters.csv`.
- Regeneration of any output directory.

All of these need the user to allow the follow-up script: `bash C_scratch/run_followup.sh; bash C_scratch/run_extra.sh`, plus the removed git lines. The auto-mode classifier denied those.
