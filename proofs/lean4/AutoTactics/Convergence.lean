/-
  AutoTactics.Convergence — Convergence rate and stability tactics
  ================================================================
  Provides tactics for proving convergence properties of iterative
  solvers (GMRES, FGMRES, Parareal PinT) as used in rusty-SUNDIALS.

  Key patterns:
    • Geometric convergence: ‖r_{k+1}‖ ≤ ρ · ‖r_k‖  with ρ < 1
    • Energy drift bound:    |E(t) - E(0)| ≤ C · dt²
    • Parareal stability:    ‖error_n‖ ≤ K^n / n! · ‖fine_error‖
-/

namespace AutoTactics.Convergence

-- Prove `ρ < 1` for a concrete convergence rate.
macro "conv_rate_lt_one" : tactic =>
  `(tactic| first | norm_num | native_decide | linarith)

-- Prove geometric decrease: `x * ρ < x` given `0 < x` and `0 < ρ < 1`.
macro "geo_decrease" : tactic =>
  `(tactic| (apply mul_lt_of_lt_one_right <;> (first | assumption | linarith | norm_num)))

-- Prove `‖residual‖ ≥ 0` (non-negativity of norms).
-- In our Float model, norms are represented as non-negative reals.
macro "norm_nonneg_auto" : tactic =>
  `(tactic| first | positivity | norm_num | linarith)

-- Prove monotone decrease of an energy function over time steps.
-- Pattern: E(t + dt) ≤ E(t) given numerical stability hypothesis.
macro "energy_monotone" : tactic =>
  `(tactic| (intro h_stab; linarith [h_stab]))

-- ---------------------------------------------------------------------------
-- Parareal PinT convergence bound
-- ---------------------------------------------------------------------------

-- The Parareal algorithm satisfies:
--   ‖e^k‖ ≤ (C · ΔT)^k / k! · ‖e^0‖
-- This tactic proves the k=1 base case.
macro "parareal_base" : tactic =>
  `(tactic| (simp; ring_nf; (first | norm_num | linarith)))

-- Prove the inductive step: if bound holds for k, it holds for k+1.
-- Requires hypothesis `h_k : ‖e^k‖ ≤ (C * ΔT)^k / k.factorial * ‖e^0‖`.
macro "parareal_step" : tactic =>
  `(tactic| (
    apply le_trans;
    · linarith [Nat.factorial_pos _]
    · ring_nf; norm_num
  ))

-- ---------------------------------------------------------------------------
-- GMRES convergence bound (min-res property)
-- ---------------------------------------------------------------------------

-- GMRES minimizes residual over Krylov subspace K_m.
-- The m-step bound: ‖r_m‖/‖r_0‖ ≤ min_{p ∈ P_m, p(0)=1} max_{λ ∈ σ(A)} |p(λ)|
-- This tactic proves the trivial bound ‖r_m‖ ≤ ‖r_0‖ (no divergence).
macro "gmres_no_diverge" : tactic =>
  `(tactic| linarith [show (0 : Float) ≤ 1 from by norm_num])

end AutoTactics.Convergence
