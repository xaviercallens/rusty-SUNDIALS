# Auto-Research V2: Corrected NLS Convergence
# Based on falsification of tq4 experiment (PR #51, Gwen: REJECT)

## Lessons from tq4 Failure

tq4 experiment regressed: 903 steps → 2,410 (+2.67×)

Root cause of failure: **wrong formula**.
My implementation: tq4 = (q+1)/(l[0]*NLSCOEF) → ~137 at BDF-5
SUNDIALS actual:    tq4 = NLSCOEF / tq[2]  → ~0.6 at BDF-5

Where tq[2] ≈ BDF_ERR_COEFF[q]:
  q=1: tq[2]=0.500, tq4=0.20
  q=2: tq[2]=0.333, tq4=0.30
  q=3: tq[2]=0.250, tq4=0.40
  q=4: tq[2]=0.200, tq4=0.50
  q=5: tq[2]=0.167, tq4=0.60

tq4=0.6 means dcon = del*crate/0.6 ≤ 1 → del ≤ 0.6 (vs our del<0.1).
6× more lenient than H1-H3, but NOT the catastrophic 137× that destabilized steps.

## Current State (v11.3.0)
  Steps: 903, RHS: 2,536, RHS/step: 2.81
  Newton iters/step ≈ 2.81 (all RHS are Newton evals)
  LLNL: Steps: 1070, RHS: 1537, Newton iters/step ≈ 1.28

## New Hypotheses

### H4: Corrected tq4 = NLSCOEF / BDF_ERR_COEFF[q]
File: solver.rs Newton loop
Formula: tq4 = NLS_COEF / BDF_ERR_COEFF[q]  (0.2–0.6 range)
Test: dcon = del * crate.min(1) / tq4 ≤ 1.0
Expected: Newton iters/step 2.81 → ~1.5-1.8 (20-40% reduction)
Safety: Since tq4 ≤ 0.6, convergence implies del ≤ 0.6 < 1.0 (no destabilization)

### H5: MAX_NLS_ITERS: 3 → 4
File: constants.rs
A conv failure costs 0.25× step shrink (very expensive). One extra Newton
iter costs only 1 RHS eval. Net benefit: fewer conv failures → fewer wasted steps.
Expected: Conv failures ↓ → steps ↓ slightly, RHS ↓ slightly

### H6: Adaptive m=0 tolerance from previous acor norm
File: solver.rs Newton loop + Cvode struct
Track prev_acor_norm across steps.
tol_m0 = max(NLS_MIN_TOL, min(NLS_TOL, NLS_CRDOWN * prev_acor_norm))
When prev step was accurate: tighter tol → converge in 1 iter.
When prev step was hard: standard tol.
Expected: Another 10-20% reduction in Newton iters.

## Validation Protocol
1. Steps: must stay within 30% of 903 (630–1175 acceptable)
2. RHS evals: must DECREASE from 2,536
3. Conservation: < 1e-12
4. BDF order: must reach 5 (not regress to 4)

## Revert Criteria
Steps > 1,440 OR BDF order < 5 → immediate revert (learned from tq4 failure)
