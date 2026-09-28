import Mathlib.Data.Real.Basic
import Mathlib.MeasureTheory.MeasurableSpace.Instances
import Mathlib.Tactic.Linarith

/-!
# Audit 2026-09-27: the fusion SOP "oracle" axioms are inconsistent

Added by the fusion audit (docs/audit/fusion-2026-09-27/README.md, report C §1.4).

The two fusion SOP certificates add "telemetry oracle" axioms and describe
them as keeping the certificates "sorry-free":

* `proofs/lean4/fusion_sop_flagno.lean:56-59`   — `flagno_l4_telemetry_oracle`
* `proofs/lean4/fusion_sop_monopole.lean:97-100` — `gcp_l4_telemetry_oracle`

Each axiom is restated below **verbatim** (same binders, same statement; only
the namespace differs so the two restatements do not clash), and `False` is
derived from each one. An environment that contains either axiom can prove
every proposition, so any "certificate" that depends on it certifies nothing.
An inconsistent axiom is strictly worse than `sorry`: `sorry` is at least
reported as `sorryAx` by `#print axioms`.

This file is a demonstration of unsoundness. It is not a certificate and it
does not import the original files (they do not compile in the audit
environment; see report C §1.2).
-/

namespace AuditDemo.FLAGNO

/-- Verbatim restatement of `fusion_sop_flagno.lean:56-59`. -/
axiom flagno_l4_telemetry_oracle
    (iters_measured : ℕ)
    (h_exec : True) -- Execution ID: L4-SERV-88219-FUS
    : iters_measured = 6 ∧ iters_measured ≤ 7

/-- Instantiating the oracle at `iters_measured := 0` gives `0 = 6`. -/
theorem flagno_oracle_proves_false : False := by
  have h := (flagno_l4_telemetry_oracle 0 trivial).1
  omega

end AuditDemo.FLAGNO

namespace AuditDemo.Monopole

variable {Ω : Type*} [MeasurableSpace Ω]

/-- Verbatim restatement of `fusion_sop_monopole.lean:97-100`. -/
axiom gcp_l4_telemetry_oracle
    (div_B_sim : Ω → ℝ) (ε : ℝ)
    (h_exec_id  : True) -- Execution ID: L4-SERV-88219-FUS (witnesses provenance)
    : ∀ x, |div_B_sim x| ≤ ε

/-- Instantiating the oracle on the one-point space with `ε := -1` gives
    `|0| ≤ -1`, contradicting `abs_nonneg`. -/
theorem monopole_oracle_proves_false : False := by
  have h := gcp_l4_telemetry_oracle (Ω := Unit) (fun _ => (0 : ℝ)) (-1) trivial ()
  have h0 : (0 : ℝ) ≤ |(0 : ℝ)| := abs_nonneg 0
  linarith

end AuditDemo.Monopole

-- Expected: each footprint contains the corresponding oracle axiom.
#print axioms AuditDemo.FLAGNO.flagno_oracle_proves_false
#print axioms AuditDemo.Monopole.monopole_oracle_proves_false
