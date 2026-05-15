/-
  rusty-SUNDIALS v11 — Lean 4 Automatic Tactics Library
  =====================================================
  Provides reusable tactic macros for the SUNDIALS formal verification suite.

  Design goals:
    • Replace `sorry` placeholders with mechanically verified proofs
    • Chain `simp`, `omega`, `norm_num`, `linarith`, `decide` automatically
    • Provide domain-specific tactics for floating-point bounds and
      convergence rate arguments

  Usage:
    import AutoTactics
    -- Then use `auto_bound`, `auto_conv`, `auto_stability` in theorems.
-/

import AutoTactics.Basic
import AutoTactics.FloatBounds
import AutoTactics.Convergence
-/
