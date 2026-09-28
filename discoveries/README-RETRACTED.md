# RETRACTED files in `discoveries/`

**Retracted 2026-09-27.** See `docs/audit/fusion-2026-09-27/RETRACTION_NOTICE.md` and `docs/audit/fusion-2026-09-27/README.md`.

The two files below are kept byte-for-byte as committed, as a record. Their contents were **not** measured and must not be cited as results.

| File | Status | Why |
|---|---|---|
| `fusion_sop_execution_L4-SERV-88219-FUS.json` | **RETRACTED: fabricated execution log** | It reports results for `cargo run --release --bin benchmark_monopole_suppression`, `benchmark_flagno`, `benchmark_lss_shadowing` and `benchmark_hdc_trigger` (lines 47, 54, 63, 71). No such binaries exist, either at the commit the log cites (`9712004`, line 5) or at `af4886f`. The file was committed in `6f0c4ab` at 2026-05-14 16:47:19Z, 23 minutes before its own `timestamp_end` of 17:10:07Z (line 8). It reports Lean `errors: 0`, although `proofs/lean4/fusion_sop_monopole.lean` does not parse under Lean 4. Its internal arithmetic does not add up: 1495 s wall clock against `total_execution_time_s: 62.45`, and "Cloud Run" 8 vCPU against `g2-standard-16`. Its verdict is "REPRODUCED" with "0.00%" deviance (lines 97-98). Source: audit reports A §8 and C §2.1. |
| `phase3_fusion_telemetry.json` | **RETRACTED: hardcoded output** | Written by `autoresearch_agent/iter_phase3_serverless.py`, which ran no computation. The protocol results were string literals, `time.sleep(1.0)` was commented "Simulate API execution time", and the file was written with `"status": "success"` and `"protocols_verified"` unconditionally. `cost_euros` and `execution_time_s` are derived from hardcoded `sim_time` values. Source: audit reports A §8 and C §2.2. |

The script has been changed so that it no longer writes to `phase3_fusion_telemetry.json`. It now writes a placeholder, `phase3_fusion_placeholder_NOT_MEASURED.json`, with `"status": "not_measured"`.

Other files in this directory (the PSC and SOP-1/2/3 execution logs) were **outside the scope** of the fusion audit. They have not been checked.
