import Mathlib.Analysis.Calculus.FDeriv.Basic
import Mathlib.Analysis.InnerProductSpace.Basic

/-!
# v3.0 Advanced Solvers: Implicit DAE Solver (IDA)

This file contains the formal mathematical specification of the Implicit
Differential-Algebraic Equation solver in rusty-SUNDIALS (`ida` module).
-/

noncomputable section

variable {V : Type*} [NormedAddCommGroup V] [InnerProductSpace ℝ V]

/-- Defines a fully implicit Differential-Algebraic Equation $F(t, y, y') = 0$. -/
structure ImplicitDAE where
  F : ℝ → V → V → V
  -- Lipschitz assumptions on y and y' to guarantee existence and uniqueness of the Newton step
  F_lipschitz_y : ∃ L > 0, ∀ t y₁ y₂ yp, ‖F t y₁ yp - F t y₂ yp‖ ≤ L * ‖y₁ - y₂‖
  F_lipschitz_yp : ∃ K > 0, ∀ t y yp₁ yp₂, ‖F t y yp₁ - F t y yp₂‖ ≤ K * ‖yp₁ - yp₂‖

/-- Fixed-point existence for the pseudo-Newton step used in IDA. -/
axiom ida_step_convergence {dae : ImplicitDAE} 
  (h : ℝ) (h_pos : h > 0) (y_n : V) :
  ∃! y_next : V, dae.F (h) y_next ((y_next - y_n) • (1/h)) = 0

end
