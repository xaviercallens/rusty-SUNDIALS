import Mathlib.Analysis.Calculus.FDeriv.Basic
import Mathlib.Analysis.InnerProductSpace.Basic

/-!
# v3.0 Advanced Solvers: IMEX Splitting

This file contains the formal specification of the Implicit-Explicit (IMEX) 
Runge-Kutta method implemented in rusty-SUNDIALS (`arkode` module).
-/

noncomputable section

variable {V : Type*} [NormedAddCommGroup V] [InnerProductSpace ℝ V]

/-- Defines a generic partitioned ODE $y' = f^E(t,y) + f^I(t,y)$ where 
    $f^E$ is non-stiff (advection) and $f^I$ is stiff (diffusion). -/
structure SplitODE where
  fE : ℝ → V → V
  fI : ℝ → V → V
  -- Lipschitz condition on the explicit part
  fE_lipschitz : ∃ L > 0, ∀ t y₁ y₂, ‖fE t y₁ - fE t y₂‖ ≤ L * ‖y₁ - y₂‖
  -- Monotonicity condition on the implicit part (coercive/dissipative)
  fI_monotonic : ∀ t y₁ y₂, ⟪fI t y₁ - fI t y₂, y₁ - y₂⟫_ℝ ≤ 0

/-- Butcher Tableau for an IMEX method. -/
structure ImexTableau where
  stages : ℕ
  c : Fin stages → ℝ
  A_E : Fin stages → Fin stages → ℝ  -- Strictly lower triangular
  A_I : Fin stages → Fin stages → ℝ  -- Lower triangular
  b_E : Fin stages → ℝ
  b_I : Fin stages → ℝ
  -- Explicit constraint
  explicit : ∀ i j, i ≤ j → A_E i j = 0
  -- Implicit constraint (diagonally implicit)
  implicit : ∀ i j, i < j → A_I i j = 0

/-- Specifies that the fixed-point iteration for the implicit stage converges 
    uniquely given the monotonicity of fI and a sufficiently small step size h. -/
axiom imex_stage_convergence {ode : SplitODE} {tab : ImexTableau} 
  (h : ℝ) (h_pos : h > 0) (i : Fin tab.stages) (y_pred : V) :
  ∃! z : V, z = y_pred + h • (tab.A_I i i) • (ode.fI (h * tab.c i) z)

end
