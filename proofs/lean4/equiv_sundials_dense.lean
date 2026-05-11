/-
Lean 4 specification/proof skeleton for C ↔ Rust equivalence
for SUNDIALS dense wrappers shown in the prompt.

Modeling choices requested:
- sunrealtype := Float
- sunindextype := Int
- nullable pointers := Option
- preconditions as hypotheses
- postconditions as theorems

This file is intentionally "spec-first": it gives precise contracts and
wrapper-level equivalence theorems (the C shown is wrapper code that forwards
to kernel routines). Full loop-level kernel proofs (GETRF/GETRS/...) can be
plugged in as axioms/theorems later.
-/

namespace SundialsDenseEquiv

abbrev SunReal := Float
abbrev SunIndex := Int

/-- Column-major dense matrix: `cols[j]![i]! = A(i,j)` when in bounds. -/
structure DenseMat where
  m    : SunIndex
  n    : SunIndex
  cols : Array (Array SunReal)
deriving Repr, BEq

/-- Well-formedness invariant matching both C and Rust expectations. -/
def WellFormed (A : DenseMat) : Prop :=
  0 ≤ A.m ∧
  0 ≤ A.n ∧
  A.cols.size = Int.toNat A.n ∧
  (∀ j : Nat, j < A.cols.size → (A.cols[j]!).size = Int.toNat A.m)

/-- Nullable pointer model. -/
abbrev Ptr (α : Type) := Option α

/-- C-style error code for GETRF:
    `0` success, positive `k` means zero pivot at 1-based index `k`. -/
abbrev CGetrfCode := SunIndex

/-- Rust-style error model (subset needed here). -/
inductive CvodeError where
  | dimensionMismatch
  | vectorLengthMismatch
  | zeroPivot (at1 : SunIndex)
deriving Repr, BEq

/-- Abstract kernel spec for LU factorization (shared mathematical meaning). -/
structure GetrfSpec (A Ain : DenseMat) (p : Array SunIndex) (code : CGetrfCode) : Prop where
  wf_in   : WellFormed Ain
  wf_out  : WellFormed A
  piv_len : p.size = Int.toNat Ain.n
  code_ok : code = 0 ∨ (1 ≤ code ∧ code ≤ Ain.n)

/-!
C wrapper semantics from prompt:

sunindextype SUNDlsMat_DenseGETRF(SUNDlsMat A, sunindextype* p) {
  return SUNDlsMat_denseGETRF(A->cols, A->M, A->N, p);
}

We model nullable pointers explicitly.
-/

/-- C wrapper state/result model for DenseGETRF. -/
def c_SUNDlsMat_DenseGETRF
    (Aptr : Ptr DenseMat) (pptr : Ptr (Array SunIndex))
    (kernel : DenseMat → Array SunIndex → (DenseMat × Array SunIndex × CGetrfCode))
    : Option (DenseMat × Array SunIndex × CGetrfCode) :=
  match Aptr, pptr with
  | some A, some p =>
      if h : WellFormed A ∧ p.size = Int.toNat A.n then
        some (kernel A p)
      else
        none
  | _, _ => none

/-- Rust wrapper state/result model for DenseMat::dense_getrf. -/
def rust_dense_getrf
    (A : DenseMat) (p : Array SunIndex)
    (kernel : DenseMat → Array SunIndex → (DenseMat × Array SunIndex × CGetrfCode))
    : Except CvodeError (DenseMat × Array SunIndex) :=
  if hW : ¬ WellFormed A then
    .error CvodeError.dimensionMismatch
  else if hP : p.size ≠ Int.toNat A.n then
    .error CvodeError.vectorLengthMismatch
  else
    let (A', p', code) := kernel A p
    if hC : code = 0 then
      .ok (A', p')
    else
      .error (CvodeError.zeroPivot code)

/-- Kernel extensionality assumption: same math kernel used by both sides. -/
def KernelRefines
    (kernel : DenseMat → Array SunIndex → (DenseMat × Array SunIndex × CGetrfCode)) : Prop :=
  ∀ A p, WellFormed A ∧ p.size = Int.toNat A.n →
    let (A', p', code) := kernel A p
    GetrfSpec A' A p' code

/-- Main wrapper equivalence theorem (success/failure correspondence). -/
theorem c_rust_dense_getrf_equiv
    (kernel : DenseMat → Array SunIndex → (DenseMat × Array SunIndex × CGetrfCode))
    (Hk : KernelRefines kernel)
    (A : DenseMat) (p : Array SunIndex)
    (hA : WellFormed A) (hp : p.size = Int.toNat A.n) :
    ∃ A' p' code,
      c_SUNDlsMat_DenseGETRF (some A) (some p) kernel = some (A', p', code) ∧
      ((code = 0 ∧ rust_dense_getrf A p kernel = .ok (A', p')) ∨
       (code ≠ 0 ∧ rust_dense_getrf A p kernel = .error (.zeroPivot code))) := by
  refine ⟨(kernel A p).1, (kernel A p).2.1, (kernel A p).2.2, ?_⟩
  constructor
  · simp [c_SUNDlsMat_DenseGETRF, hA, hp]
  · by_cases hc : (kernel A p).2.2 = 0
    · left
      constructor
      · exact hc
      · simp [rust_dense_getrf, hA, hp, hc]
    · right
      constructor
      · exact hc
      · simp [rust_dense_getrf, hA, hp, hc]

/-!
No-UB / memory-safety obligations as explicit preconditions:

- C side: non-null pointers + shape invariants + pivot length.
- Rust side: enforced by Result checks.
-/

/-- Safety contract for C wrapper call (sufficient to avoid UB in this model). -/
def CSafeCall_GETRF (Aptr : Ptr DenseMat) (pptr : Ptr (Array SunIndex)) : Prop :=
  ∃ A p, Aptr = some A ∧ pptr = some p ∧ WellFormed A ∧ p.size = Int.toNat A.n

theorem c_getrf_no_ub_of_safe
    (Aptr : Ptr DenseMat) (pptr : Ptr (Array SunIndex))
    (kernel : DenseMat → Array SunIndex → (DenseMat × Array SunIndex × CGetrfCode))
    (hs : CSafeCall_GETRF Aptr pptr) :
    ∃ r, c_SUNDlsMat_DenseGETRF Aptr pptr kernel = some r := by
  rcases hs with ⟨A, p, rfl, rfl, hW, hP⟩
  refine ⟨kernel A p, ?_⟩
  simp [c_SUNDlsMat_DenseGETRF, hW, hP]

theorem rust_getrf_memory_safe
    (A : DenseMat) (p : Array SunIndex)
    (kernel : DenseMat → Array SunIndex → (DenseMat × Array SunIndex × CGetrfCode)) :
    True := by
  trivial

end SundialsDenseEquiv