# Auto-Research V3: Persistent Convergence Rate & Newton Instrumentation
# For scientific paper: "rusty-SUNDIALS — A Rust Implementation of CVODE"

## The Smoking Gun: crate_nls Reset

### Discovery
LLNL cvode.c:cvNlsNewton persists `cv_crate` as a STRUCT FIELD.
Our V2 code resets `crate_nls = 1.0` EVERY STEP (line 464).

### Why this matters
LLNL convergence test: dcon = del * min(1, crate) / tq4 ≤ 1.0

When crate is persisted from previous step:
  After initial transient: crate ≈ 0.01–0.1 (Newton converges fast)
  dcon = del * 0.01 / 0.6 = del * 0.017
  → converges in 1 iter for ANY del < 60

When crate is reset to 1.0 every step (our bug):
  dcon = del * 1.0 / 0.6 = del * 1.67
  → converges in 1 iter only when del < 0.6

This explains the entire 1.69× RHS gap:
  Rust: 2.42 Newton iters/step (crate=1.0 → needs del<0.6)
  C:    1.44 Newton iters/step (crate≈0.01 → needs del<60)

## Hypotheses

### H7: Persist crate_nls across steps (THE FIX)
File: solver.rs struct field + Newton loop
Change: Add `nls_crate: Real` to Cvode struct, init to 1.0.
        Use self.nls_crate instead of local crate_nls.
        Update in Newton loop (already happens via crate formula).
Expected: Newton iters/step 2.42 → ~1.4 → RHS 2603 → ~1500

### H8: Newton iteration instrumentation
File: solver.rs struct + solve output
Add `nni: usize` counter (total Newton iterations across all steps).
Print NI/step ratio for paper comparison.
C ref: nni=1537, nni/nst=1.44
Expected: After H7, nni/nst ≈ 1.3–1.5

### H9: Jacobian gamma-ratio scaling (LLNL gamrat)
File: solver.rs Jacobian setup
LLNL tracks gamrat = γ_new/γ_old and scales existing J:
  When |gamrat - 1| < DGMAX: reuse M with scaled γ
  When |gamrat - 1| ≥ DGMAX: recompute J
Our code already does this via jac_age + last_gamma.
LOW PRIORITY — verify correctness only.

## Validation Protocol
1. Steps: must stay within [700, 1400] (was 1076)
2. RHS evals: must DECREASE from 2603 — target < 1600
3. NI/step ratio: target < 1.6 (C ref: 1.44)
4. Conservation: < 1e-12
5. BDF order: must reach 5

## Revert Criteria
Steps > 1800 OR BDF order < 5 OR Conservation > 1e-10 → immediate revert
