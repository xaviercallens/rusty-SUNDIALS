/-
Lean 4 specification/proof skeleton for C ↔ Rust equivalence
for the provided CVODE constants/types fragment.

Modeling choices requested:
- sunrealtype  ↦ Float
- indices      ↦ Int
- nullable ptr ↦ Option α

This file is intentionally self-contained and focuses on the
semantics visible in the snippet: constants, status space, and
basic safety invariants (no UB / memory safety at this abstraction).
-/

namespace CvodeEquiv

/-- C `sunrealtype` and Rust `Real = f64` are modeled as Lean `Float`. -/
abbrev SunReal := Float

/-- C/Rust index-like quantities modeled as mathematical integers. -/
abbrev Index := Int

/-- Nullable pointer model. `none` = NULL, `some x` = valid pointer to x. -/
abbrev Ptr (α : Type) := Option α

/-- C-style status codes (subset used in the snippet). -/
inductive CStatus where
  | success
  | memNull
  | illInput
  | memFail
  | rhsFuncFail
  | rootFuncFail
  | nonlinearSolverFail
  | other
deriving DecidableEq, Repr

/-- Rust error enum (subset mirroring snippet). -/
inductive RustErr where
  | memNull
  | illInput
  | memFail
  | rhsFuncFail
  | rootFuncFail
  | nonlinearSolverFail
  | other
deriving DecidableEq, Repr

/-- Rust `Result`-style status abstraction. -/
abbrev RustStatus := Except RustErr Unit

/-- C→Rust status translation. -/
def cToRustStatus : CStatus → RustStatus
  | .success            => .ok ()
  | .memNull            => .error .memNull
  | .illInput           => .error .illInput
  | .memFail            => .error .memFail
  | .rhsFuncFail        => .error .rhsFuncFail
  | .rootFuncFail       => .error .rootFuncFail
  | .nonlinearSolverFail=> .error .nonlinearSolverFail
  | .other              => .error .other

/-- Rust→C status translation. -/
def rustToCStatus : RustStatus → CStatus
  | .ok _                        => .success
  | .error .memNull              => .memNull
  | .error .illInput             => .illInput
  | .error .memFail              => .memFail
  | .error .rhsFuncFail          => .rhsFuncFail
  | .error .rootFuncFail         => .rootFuncFail
  | .error .nonlinearSolverFail  => .nonlinearSolverFail
  | .error .other                => .other

/-- C constants from snippet (modeled). -/
namespace C
def ZERO    : SunReal := 0.0
def TINY    : SunReal := 1.0e-10
def HALF    : SunReal := 0.5
def ONE     : SunReal := 1.0
def HUNDRED : SunReal := 100.0
end C

/-- Rust constants from snippet (modeled). -/
namespace Rust
def ZERO    : SunReal := 0.0
def TINY    : SunReal := 1.0e-10
def HALF    : SunReal := 0.5
def ONE     : SunReal := 1.0
def HUNDRED : SunReal := 100.0
end Rust

/-- CVODE memory object (abstract). -/
structure CvMem where
  initialized : Bool
  deriving Repr

/-- A tiny abstract operation representative of init/check behavior. -/
def c_checkMem (m : Ptr CvMem) : CStatus :=
  match m with
  | none   => .memNull
  | some _ => .success

/-- Rust analogue returning `Result<(), CvodeError>`. -/
def rust_checkMem (m : Ptr CvMem) : RustStatus :=
  match m with
  | none   => .error .memNull
  | some _ => .ok ()

/-!
Preconditions as hypotheses, postconditions as theorems.
-/

/-- Precondition: pointer is non-null. -/
def Pre_NonNull (m : Ptr CvMem) : Prop := m ≠ none

/-- Postcondition: check returns success. -/
def Post_CheckSuccessC (m : Ptr CvMem) : Prop := c_checkMem m = .success
def Post_CheckSuccessR (m : Ptr CvMem) : Prop := rust_checkMem m = .ok ()

/-- C-side correctness under precondition. -/
theorem c_checkMem_correct
    (m : Ptr CvMem)
    (hpre : Pre_NonNull m) :
    Post_CheckSuccessC m := by
  cases m with
  | none =>
      contradiction
  | some v =>
      rfl

/-- Rust-side correctness under precondition. -/
theorem rust_checkMem_correct
    (m : Ptr CvMem)
    (hpre : Pre_NonNull m) :
    Post_CheckSuccessR m := by
  cases m with
  | none =>
      contradiction
  | some v =>
      rfl

/-- Constant-level semantic equivalence (C vs Rust). -/
theorem const_ZERO_equiv : C.ZERO = Rust.ZERO := rfl
theorem const_TINY_equiv : C.TINY = Rust.TINY := rfl
theorem const_HALF_equiv : C.HALF = Rust.HALF := rfl
theorem const_ONE_equiv : C.ONE = Rust.ONE := rfl
theorem const_HUNDRED_equiv : C.HUNDRED = Rust.HUNDRED := rfl

/-- Status translation round-trip (C -> Rust -> C). -/
theorem status_roundtrip_c (s : CStatus) :
    rustToCStatus (cToRustStatus s) = s := by
  cases s <;> rfl

/-- Status translation round-trip (Rust -> C -> Rust). -/
theorem status_roundtrip_r (r : RustStatus) :
    cToRustStatus (rustToCStatus r) = r := by
  cases r with
  | ok u => rfl
  | error e =>
      cases e <;> rfl

/-- Behavioral equivalence of the representative memory check. -/
theorem checkMem_equiv (m : Ptr CvMem) :
    cToRustStatus (c_checkMem m) = rust_checkMem m := by
  cases m <;> rfl

/-
No-UB / memory-safety statement at this abstraction:
- all pointer dereference is guarded by pattern match on `Option`;
- therefore null is handled explicitly and cannot be dereferenced.
-/
theorem no_null_deref_c_checkMem (m : Ptr CvMem) :
    (m = none → c_checkMem m = .memNull) ∧
    (∀ v, m = some v → c_checkMem m = .success) := by
  constructor
  · intro h; simpa [h, c_checkMem]
  · intro v h; simpa [h, c_checkMem]

theorem no_null_deref_rust_checkMem (m : Ptr CvMem) :
    (m = none → rust_checkMem m = .error .memNull) ∧
    (∀ v, m = some v → rust_checkMem m = .ok ()) := by
  constructor
  · intro h; simpa [h, rust_checkMem]
  · intro v h; simpa [h, rust_checkMem]

/-- Final packaged equivalence theorem (behavior + safety). -/
theorem c_rust_equiv_main (m : Ptr CvMem) :
    (cToRustStatus (c_checkMem m) = rust_checkMem m) ∧
    (m = none → c_checkMem m = .memNull ∧ rust_checkMem m = .error .memNull) := by
  constructor
  · exact checkMem_equiv m
  · intro hnull
    constructor <;> simpa [hnull, c_checkMem, rust_checkMem]

end CvodeEquiv