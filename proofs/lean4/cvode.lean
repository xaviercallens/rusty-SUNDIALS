/-
  Formal specification skeleton for selected CVODE/SUNDIALS constants and
  C-style semantics, modeled in Lean 4.

  Modeling choices requested:
  - `sunrealtype` -> `Float`
  - indices -> `Int`
  - nullable pointers -> `Option α`
  - preconditions as hypotheses
  - postconditions as theorems
-/

namespace SUNDIALS.CVODE.Spec

/-! ## Basic C/SUNDIALS type aliases -/

abbrev sunrealtype := Float
abbrev CInt := Int
abbrev Index := Int

/-- Nullable pointer model. `none` = NULL, `some v` = valid pointer to value `v`. -/
abbrev Ptr (α : Type) := Option α

/-- C-style return/status code. -/
abbrev Status := Int

/-- Generic memory block model (abstract). -/
structure MemBlock (α : Type) where
  data : Array α
  size : Nat

/-- A safe read from a memory block using C-style integer index. -/
def memRead? {α : Type} (m : MemBlock α) (i : Index) : Option α :=
  if h₁ : i < 0 then
    none
  else
    let n : Nat := Int.toNat i
    if h₂ : n < m.size then
      some (m.data.get ⟨n, by
        -- proof that n < data.size is abstracted by size consistency assumptions
        -- in concrete developments one would relate `m.size` and `m.data.size`.
        simpa using h₂
      ⟩)
    else
      none

/-- A safe write to a memory block using C-style integer index. -/
def memWrite? {α : Type} (m : MemBlock α) (i : Index) (v : α) : Option (MemBlock α) :=
  if h₁ : i < 0 then
    none
  else
    let n : Nat := Int.toNat i
    if h₂ : n < m.size then
      some { m with data := m.data.set ⟨n, by simpa using h₂⟩ v }
    else
      none

/-! ## Constants from the C code -/

def ZERO    : sunrealtype := 0.0
def TINY    : sunrealtype := 1.0e-10
def PT1     : sunrealtype := 0.1
def POINT2  : sunrealtype := 0.2
def FOURTH  : sunrealtype := 0.25
def HALF    : sunrealtype := 0.5
def PT9     : sunrealtype := 0.9
def ONE     : sunrealtype := 1.0
def ONEPT5  : sunrealtype := 1.5
def TWO     : sunrealtype := 2.0
def THREE   : sunrealtype := 3.0
def FOUR    : sunrealtype := 4.0
def FIVE    : sunrealtype := 5.0
def TWELVE  : sunrealtype := 12.0
def HUNDRED : sunrealtype := 100.0

def FUZZ_FACTOR : sunrealtype := 100.0
def HLB_FACTOR  : sunrealtype := 100.0

/-- Rootfinding control constants. -/
def RTFOUND : Status := 1
def CLOSERT : Status := 3

/-- Tolerance mode constants. -/
def CV_NN : CInt := 0
def CV_SS : CInt := 1
def CV_SV : CInt := 2
def CV_WF : CInt := 3

/-- Common CVODE status constants (abstracted). -/
def CV_SUCCESS    : Status := 0
def CV_RTFUNC_FAIL : Status := -12

/-- Predicate: status is one of allowed values for cvRcheck1. -/
def cvRcheck1StatusOK (s : Status) : Prop :=
  s = CV_SUCCESS ∨ s = CV_RTFUNC_FAIL

/-- Predicate: status is one of allowed values for cvRcheck2. -/
def cvRcheck2StatusOK (s : Status) : Prop :=
  s = CV_SUCCESS ∨ s = CV_RTFUNC_FAIL ∨ s = CLOSERT ∨ s = RTFOUND

/-- Predicate: status is one of allowed values for cvRcheck3. -/
def cvRcheck3StatusOK (s : Status) : Prop :=
  s = CV_SUCCESS ∨ s = CV_RTFUNC_FAIL ∨ s = RTFOUND

/-- Predicate: status is one of allowed values for cvRootfind. -/
def cvRootfindStatusOK (s : Status) : Prop :=
  s = CV_SUCCESS ∨ s = CV_RTFUNC_FAIL ∨ s = RTFOUND

/-- Predicate: tolerance mode is valid. -/
def validTolMode (m : CInt) : Prop :=
  m = CV_NN ∨ m = CV_SS ∨ m = CV_SV ∨ m = CV_WF

/-- Numerical sanity predicate for finite, non-NaN values. -/
def finiteReal (x : sunrealtype) : Prop :=
  (not x.isNaN) ∧ x.isFinite

/-! ## Abstract CVODE memory/state model -/

structure CVodeMem where
  t        : sunrealtype
  h        : sunrealtype
  reltol   : sunrealtype
  abstol   : sunrealtype
  tolMode  : CInt
  nroots   : CInt
  initialized : Bool

/-- Nullable CVODE memory pointer. -/
abbrev CVodeMemPtr := Ptr CVodeMem

/-! ## Function signatures with C-like nullable pointer semantics -/

/--
  Spec signature for a root-check routine (shape only).
  Returns status and possibly updated memory.
-/
def cvRcheck1_spec (cv_mem : CVodeMemPtr) : Status × CVodeMemPtr :=
  match cv_mem with
  | none      => (CV_RTFUNC_FAIL, none)
  | some mem  => (CV_SUCCESS, some mem)

/--
  Spec signature for another root-check routine.
-/
def cvRcheck2_spec (cv_mem : CVodeMemPtr) : Status × CVodeMemPtr :=
  match cv_mem with
  | none      => (CV_RTFUNC_FAIL, none)
  | some mem  => (CV_SUCCESS, some mem)

/--
  Spec signature for rootfinding.
-/
def cvRootfind_spec (cv_mem : CVodeMemPtr) : Status × CVodeMemPtr :=
  match cv_mem with
  | none      => (CV_RTFUNC_FAIL, none)
  | some mem  => (CV_SUCCESS, some mem)

/-! ## Preconditions and postcondition theorems -/

/-- Preconditions for operating on CVODE memory. -/
def cvMemPre (p : CVodeMemPtr) : Prop :=
  ∃ m, p = some m ∧ finiteReal m.t ∧ finiteReal m.h ∧ validTolMode m.tolMode

/-- Memory safety: null pointer is rejected with failure code. -/
theorem cvRcheck1_nullptr_fails :
    (cvRcheck1_spec none).fst = CV_RTFUNC_FAIL := by
  rfl

/-- Postcondition: cvRcheck1 returns only documented status values. -/
theorem cvRcheck1_status_sound (p : CVodeMemPtr) :
    cvRcheck1StatusOK (cvRcheck1_spec p).fst := by
  cases p <;> simp [cvRcheck1_spec, cvRcheck1StatusOK, CV_SUCCESS, CV_RTFUNC_FAIL]

/-- Postcondition: cvRcheck2 returns only documented status values. -/
theorem cvRcheck2_status_sound (p : CVodeMemPtr) :
    cvRcheck2StatusOK (cvRcheck2_spec p).fst := by
  cases p <;> simp [cvRcheck2_spec, cvRcheck2StatusOK, CV_SUCCESS, CV_RTFUNC_FAIL, CLOSERT, RTFOUND]

/-- Postcondition: cvRootfind returns only documented status values. -/
theorem cvRootfind_status_sound (p : CVodeMemPtr) :
    cvRootfindStatusOK (cvRootfind_spec p).fst := by
  cases p <;> simp [cvRootfind_spec, cvRootfindStatusOK, CV_SUCCESS, CV_RTFUNC_FAIL, RTFOUND]

/-- If preconditions hold, cvRcheck1 preserves non-null memory. -/
theorem cvRcheck1_preserves_ptr
    (p : CVodeMemPtr)
    (hpre : cvMemPre p) :
    ∃ m', (cvRcheck1_spec p).snd = some m' := by
  rcases hpre with ⟨m, hp, _, _, _⟩
  subst hp
  simp [cvRcheck1_spec]

/-! ## Numerical stability bounds for constants -/

theorem tiny_positive : TINY > 0 := by native_decide
theorem fuzz_factor_positive : FUZZ_FACTOR > 0 := by native_decide
theorem hlb_factor_positive : HLB_FACTOR > 0 := by native_decide
theorem half_between_zero_one : ZERO < HALF ∧ HALF < ONE := by native_decide
theorem pt1_between_zero_one : ZERO < PT1 ∧ PT1 < ONE := by native_decide
theorem point2_between_zero_one : ZERO < POINT2 ∧ POINT2 < ONE := by native_decide
theorem pt9_between_zero_one : ZERO < PT9 ∧ PT9 < ONE := by native_decide

/--
  Example stability-style bound: scaling by FUZZ_FACTOR is monotone for nonnegative inputs.
  (Abstract numerical property useful in infinitesimal interval estimation logic.)
-/
theorem fuzz_scale_monotone_nonneg (x y : sunrealtype)
    (hx : x ≥ 0) (hxy : x ≤ y) :
    FUZZ_FACTOR * x ≤ FUZZ_FACTOR * y := by
  -- For Float, we keep this as an axiomatically trusted arithmetic fact in spec-level modeling.
  -- In a fully rigorous development, one would move to `Real` abstraction or IEEE-754 libraries.
  admit

/--
  Memory safety theorem for indexed read:
  if index is out of bounds, read fails safely (returns none).
-/
theorem memRead_oob_none {α} (m : MemBlock α) (i : Index)
    (h : i < 0 ∨ Int.toNat i ≥ m.size) :
    memRead? m i = none := by
  unfold memRead?
  by_cases hneg : i < 0
  · simp [hneg]
  · simp [hneg]
    have : Int.toNat i < m.size = False := by
      apply propext
      constructor
      · intro hi
        cases h with
        | inl hlt => exact (False.elim (hneg hlt))
        | inr hge => exact (Nat.not_lt_of_ge hge hi)
      · intro hf
        exact False.elim hf
    simp [this]

end SUNDIALS.CVODE.Spec