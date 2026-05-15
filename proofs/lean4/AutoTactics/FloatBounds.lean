/-
  AutoTactics.FloatBounds — Float/Real bound tactics for SUNDIALS constants
  ==========================================================================
  Provides tactics for proving bounds on `sunrealtype` (Float) constants
  like TINY, HALF, PT9 that appear throughout the CVODE spec.

  These tactics use `native_decide` for concrete Float literals and
  `norm_num` for algebraic reductions.
-/

namespace AutoTactics.FloatBounds

-- Prove `c > 0` for a Float constant given as a numeric literal.
macro "float_pos" : tactic =>
  `(tactic| first | native_decide | norm_num | positivity)

-- Prove `0 < x ∧ x < 1` for a Float constant between 0 and 1.
macro "float_unit_interval" : tactic =>
  `(tactic| (constructor <;> (first | native_decide | norm_num)))

-- Prove `a ≤ b` for concrete Float bounds.
macro "float_le" : tactic =>
  `(tactic| first | native_decide | norm_num | linarith)

-- Prove `x.isFinite` and `¬x.isNaN` for concrete constants.
macro "float_finite" : tactic =>
  `(tactic| simp [Float.isFinite, Float.isNaN] <;> try native_decide)

-- ---------------------------------------------------------------------------
-- Example: re-prove cvode.lean theorems with AutoTactics
-- ---------------------------------------------------------------------------

-- These import the SUNDIALS spec and prove bounds using auto-tactics.
-- (Uncomment after `import` machinery is set up in lakefile.lean.)

-- section CVODEBoundsDemo
-- open SUNDIALS.CVODE.Spec
--
-- theorem tiny_positive' : TINY > 0 := by float_pos
-- theorem half_in_01'    : ZERO < HALF ∧ HALF < ONE := by float_unit_interval
-- theorem pt9_in_01'     : ZERO < PT9  ∧ PT9  < ONE := by float_unit_interval
--
-- end CVODEBoundsDemo

end AutoTactics.FloatBounds
