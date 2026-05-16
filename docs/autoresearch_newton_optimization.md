# Auto-Research Plan: Newton Solver Optimization
# rusty-SUNDIALS v11.3.0 — Reducing RHS Evaluation Gap vs LLNL C

## Objective
Reduce RHS evaluations from 2,707 → ~1,537 (LLNL C ref) without increasing step count.

## Current State (v11.2.0, CI run 25953795792)
- Steps:     960   (0.9× C ref = GOOD)
- RHS evals: 2,707 (1.76× C ref = TARGET FOR IMPROVEMENT)
- RHS/step:  2.82  (C ref: 1.44) ← gap is here

## Root Cause Analysis

Per-step RHS breakdown:
  1x  — f(y_pred) at prediction point (line 380) [unavoidable]
  Nx  — f(y_new) per Newton iteration (line 452) [optimization target]

Rust: 2.82 RHS/step → ~1.82 Newton RHS/step → ~1.82 Newton iters/step average
C:   1.44 RHS/step → ~0.44 Newton RHS/step → ~0.44 Newton iters/step average

## Hypotheses (3 targeted fixes)

### H1: LLNL convergence threshold (tol=0.1 → tol=0.33)
File: solver.rs line 489, 496
LLNL uses tol = min(0.1, ||acor|| * CRDOWN) where CRDOWN = 0.3.
Our flat tol=0.1 is too tight — forces extra iterations.
Fix: Use LLNL's CRDOWN=0.3 factor: tol = 0.1 on first iter, relax to 0.33 after.
Expected: ~30% reduction in Newton iters per step.

### H2: First-iteration direct convergence (m=0 early exit)
File: solver.rs line 481 (rho check only applies for m>0)
On m=0, we have no rho to check — we just check del<0.1 (too strict).
LLNL exits on m=0 if del < tol immediately (no rho needed).
Fix: Apply the same tol check from the start, no special m==0 handling.

### H3: Correct del_old initialization
File: solver.rs line 443
del_old = 1.0 means rho=del/1.0=del on first comparison — always < 0.9
so the divergence guard never fires on iter 1.
LLNL: del_old = del from first iteration (rho computed only for m>=2).
Fix: Initialize del_old to del from m=0, compute rho from m=1 onwards.

## Expected Result
- Newton iters/step: 1.82 → ~1.0-1.2
- RHS evals: 2,707 → ~1,400-1,800
- Steps: 960 (unchanged — step control is independent of Newton iter count)
- Conservation: unchanged

## Validation Protocol
1. CI benchmark must show Steps within 20% of current (960±200)
2. RHS evals must decrease from 2,707
3. Conservation error must remain < 1e-12
4. Peer review via Gwen (Mistral AI) on benchmark report delta

## Revert Criteria
If steps increase by >50% (>1,440) → revert immediately (solver instability)
