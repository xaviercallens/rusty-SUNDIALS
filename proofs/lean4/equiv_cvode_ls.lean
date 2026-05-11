/-
Lean 4 specification/proof skeleton for behavioral equivalence of the visible
prefix of C `CVodeSetLinearSolver` and Rust `cvode_set_linear_solver`-style API.

Scope modeled from provided snippet:
1) if cvode_mem == NULL  => return CVLS_MEM_NULL
2) else if LS == NULL    => return CVLS_ILL_INPUT
3) else continue with cv_mem cast / normal path (modeled as `ok`)

Modeling choices requested:
- sunrealtype := Float
- indices := Int
- nullable pointers := Option
- preconditions as hypotheses
- postconditions as theorems
- no UB / memory safety captured by total, Option-based semantics
-/

namespace CVLS

abbrev SunRealType := Float
abbrev SunIndexType := Int

/-- C-style integer return codes relevant to the shown prefix. -/
inductive CReturnCode where
  | CVLS_MEM_NULL
  | CVLS_ILL_INPUT
  | OK
deriving DecidableEq, Repr

/-- Rust-style error/result surface corresponding to the same behavior. -/
inductive CvodeError where
  | MemNull (msg : String)
  | IllInput (msg : String)
deriving DecidableEq, Repr

inductive RustResult (α : Type) where
  | ok    (a : α)
  | error (e : CvodeError)
deriving DecidableEq, Repr

/-- Abstract memory objects (opaque). -/
structure CvodeMem where
  id : Int
deriving DecidableEq, Repr

structure LinearSolver where
  id : Int
deriving DecidableEq, Repr

/-- C semantics for the visible prefix of `CVodeSetLinearSolver`. -/
def c_CVodeSetLinearSolver_prefix
    (cvode_mem : Option CvodeMem)
    (ls        : Option LinearSolver) : CReturnCode :=
  match cvode_mem with
  | none      => .CVLS_MEM_NULL
  | some _ =>
    match ls with
    | none      => .CVLS_ILL_INPUT
    | some _    => .OK

/-- Rust semantics for the corresponding visible checks. -/
def rust_CVodeSetLinearSolver_prefix
    (cvode_mem : Option CvodeMem)
    (ls        : Option LinearSolver) : RustResult Unit :=
  match cvode_mem with
  | none      => .error (.MemNull "cvode_mem")
  | some _ =>
    match ls with
    | none      => .error (.IllInput "LS must be non-NULL")
    | some _    => .ok ()

/-- Refinement relation between C return codes and Rust results. -/
def ret_rel : CReturnCode → RustResult Unit → Prop
  | .CVLS_MEM_NULL,  .error (.MemNull _)  => True
  | .CVLS_ILL_INPUT, .error (.IllInput _) => True
  | .OK,             .ok ()               => True
  | _,               _                    => False

/-- Main equivalence theorem for the shown behavior. -/
theorem c_rust_prefix_equiv
    (cvode_mem : Option CvodeMem)
    (ls : Option LinearSolver) :
    ret_rel
      (c_CVodeSetLinearSolver_prefix cvode_mem ls)
      (rust_CVodeSetLinearSolver_prefix cvode_mem ls) := by
  cases cvode_mem with
  | none =>
      simp [c_CVodeSetLinearSolver_prefix, rust_CVodeSetLinearSolver_prefix, ret_rel]
  | some m =>
      cases ls with
      | none =>
          simp [c_CVodeSetLinearSolver_prefix, rust_CVodeSetLinearSolver_prefix, ret_rel]
      | some l =>
          simp [c_CVodeSetLinearSolver_prefix, rust_CVodeSetLinearSolver_prefix, ret_rel]

/- Preconditions as hypotheses, postconditions as theorems -/

/-- If `cvode_mem` is null, both implementations report memory-null class error. -/
theorem post_mem_null
    (ls : Option LinearSolver)
    (hpre : (none : Option CvodeMem) = none) :
    c_CVodeSetLinearSolver_prefix none ls = .CVLS_MEM_NULL ∧
    (∃ msg, rust_CVodeSetLinearSolver_prefix none ls = .error (.MemNull msg)) := by
  constructor
  · simp [c_CVodeSetLinearSolver_prefix]
  · refine ⟨"cvode_mem", ?_⟩
    simp [rust_CVodeSetLinearSolver_prefix]

/-- If `cvode_mem` is non-null and `ls` is null, both report illegal-input class error. -/
theorem post_ls_null
    (m : CvodeMem)
    (hpre1 : (some m : Option CvodeMem) ≠ none)
    (hpre2 : (none : Option LinearSolver) = none) :
    c_CVodeSetLinearSolver_prefix (some m) none = .CVLS_ILL_INPUT ∧
    (∃ msg, rust_CVodeSetLinearSolver_prefix (some m) none = .error (.IllInput msg)) := by
  constructor
  · simp [c_CVodeSetLinearSolver_prefix]
  · refine ⟨"LS must be non-NULL", ?_⟩
    simp [rust_CVodeSetLinearSolver_prefix]

/-- If both pointers are non-null, both proceed successfully on this prefix path. -/
theorem post_nonnull_ok
    (m : CvodeMem) (l : LinearSolver)
    (hpre1 : (some m : Option CvodeMem) ≠ none)
    (hpre2 : (some l : Option LinearSolver) ≠ none) :
    c_CVodeSetLinearSolver_prefix (some m) (some l) = .OK ∧
    rust_CVodeSetLinearSolver_prefix (some m) (some l) = .ok () := by
  constructor <;> simp [c_CVodeSetLinearSolver_prefix, rust_CVodeSetLinearSolver_prefix]

/- No-UB / memory-safety statement (semantic level):
   Because nullable pointers are modeled as Option and functions are total,
   evaluation cannot dereference null and cannot get stuck. -/
theorem no_ub_totality
    (cvode_mem : Option CvodeMem) (ls : Option LinearSolver) :
    ∃ c : CReturnCode, c = c_CVodeSetLinearSolver_prefix cvode_mem ls := by
  exact ⟨_, rfl⟩

theorem rust_memory_safe_totality
    (cvode_mem : Option CvodeMem) (ls : Option LinearSolver) :
    ∃ r : RustResult Unit, r = rust_CVodeSetLinearSolver_prefix cvode_mem ls := by
  exact ⟨_, rfl⟩

end CVLS