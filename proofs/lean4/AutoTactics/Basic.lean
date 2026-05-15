/-
  AutoTactics.Basic — Core tactic combinators for rusty-SUNDIALS v11
  ==================================================================
  Provides `auto_bound`, `decide_eq`, and `simp_arith` macros that
  eliminate the most common `sorry` patterns in the proof suite.
-/

namespace AutoTactics

-- ---------------------------------------------------------------------------
-- `auto_bound` — tries a cascade of arithmetic decision procedures.
-- Covers: omega (integer/linear), norm_num (numeric literals),
--         linarith (linear arithmetic over ordered fields), native_decide.
-- ---------------------------------------------------------------------------
macro "auto_bound" : tactic =>
  `(tactic| first
    | omega
    | norm_num
    | linarith
    | native_decide
    | decide
    | simp_arith)

-- ---------------------------------------------------------------------------
-- `auto_eq` — proves equalities by normalization then reflection.
-- ---------------------------------------------------------------------------
macro "auto_eq" : tactic =>
  `(tactic| first
    | rfl
    | norm_num
    | simp [*]
    | omega)

-- ---------------------------------------------------------------------------
-- `auto_cases` — destructs an Option/Bool/Decidable and closes each branch.
-- ---------------------------------------------------------------------------
macro "auto_cases" x:term : tactic =>
  `(tactic| (cases $x <;> simp_all <;> try auto_bound))

-- ---------------------------------------------------------------------------
-- `prove_pos` — proves `x > 0` for concrete numeric constants.
-- ---------------------------------------------------------------------------
macro "prove_pos" : tactic =>
  `(tactic| first | norm_num | linarith | native_decide)

-- ---------------------------------------------------------------------------
-- `prove_bounded` — proves `a < x ∧ x < b` goals.
-- ---------------------------------------------------------------------------
macro "prove_bounded" : tactic =>
  `(tactic| constructor <;> auto_bound)

end AutoTactics
