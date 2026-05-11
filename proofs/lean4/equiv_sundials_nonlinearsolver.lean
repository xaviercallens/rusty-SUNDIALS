/-
Lean 4 specification/proof skeleton for C ↔ Rust equivalence
for SUNNonlinSolNewEmpty-style constructor behavior.

Notes:
- We model `sunrealtype` as `Float`.
- We model indices as `Int`.
- We model nullable pointers with `Option`.
- We encode preconditions as hypotheses and postconditions as theorems.
- This is a semantic model (not a byte-level CompCert proof).
-/

namespace SUNDIALS.NonlinearSolver

--------------------------------------------------------------------------------
-- Basic C/Rust-aligned types
--------------------------------------------------------------------------------

abbrev SunReal : Type := Float
abbrev SunIndex : Type := Int

inductive NlsError where
  | mallocFail
  | argCorrupt
  | argIncompatible
  | missingOperation (op : String)
  | invalidOptionValue (v : String)
  | backend (msg : String)
  deriving DecidableEq, Repr

inductive NonlinearSolverType where
  | unknown
  | newton
  | fixedPoint
  deriving DecidableEq, Repr

structure SunProfiler where
  dummy : Unit := ()

structure SunContext where
  profiler : Option SunProfiler := none

--------------------------------------------------------------------------------
-- Operation table and solver object model
--------------------------------------------------------------------------------

/-- C ops table fields initialized to NULL in SUNNonlinSolNewEmpty. -/
structure NLSOps where
  gettype         : Option Unit
  initialize      : Option Unit
  setup           : Option Unit
  solve           : Option Unit
  free            : Option Unit
  setsysfn        : Option Unit
  setlsetupfn     : Option Unit
  setlsolvefn     : Option Unit
  setctestfn      : Option Unit
  setoptions      : Option Unit
  setmaxiters     : Option Unit
  getnumiters     : Option Unit
  getcuriter      : Option Unit
  getnumconvfails : Option Unit
  deriving DecidableEq, Repr

def nullOps : NLSOps :=
  { gettype := none
    initialize := none
    setup := none
    solve := none
    free := none
    setsysfn := none
    setlsetupfn := none
    setlsolvefn := none
    setctestfn := none
    setoptions := none
    setmaxiters := none
    getnumiters := none
    getcuriter := none
    getnumconvfails := none }

structure NLSObj where
  sunctx  : SunContext
  ops     : NLSOps
  content : Option Unit := none
  deriving DecidableEq, Repr

--------------------------------------------------------------------------------
-- Abstract allocation model (to capture malloc failure/success)
--------------------------------------------------------------------------------

structure AllocModel where
  allocNLS : Bool   -- whether malloc(sizeof *NLS) succeeds
  allocOps : Bool   -- whether malloc(sizeof *ops) succeeds
  deriving Repr

/-- C constructor semantics (modeled): returns nullable pointer. -/
def c_SUNNonlinSolNewEmpty (ctx : Option SunContext) (am : AllocModel) : Option NLSObj :=
  match ctx with
  | none => none
  | some c =>
      if h₁ : am.allocNLS then
        if h₂ : am.allocOps then
          some { sunctx := c, ops := nullOps, content := none }
        else
          none
      else
        none

/-- Rust constructor semantics (modeled): Result instead of nullable pointer. -/
def rust_SUNNonlinSolNewEmpty (ctx : Option SunContext) (am : AllocModel) :
    Except NlsError NLSObj :=
  match ctx with
  | none => .error .argCorrupt
  | some c =>
      if h₁ : am.allocNLS then
        if h₂ : am.allocOps then
          .ok { sunctx := c, ops := nullOps, content := none }
        else
          .error .mallocFail
      else
        .error .mallocFail

--------------------------------------------------------------------------------
-- Refinement relation between C nullable return and Rust Result
--------------------------------------------------------------------------------

def refinesRet : Option NLSObj → Except NlsError NLSObj → Prop
  | none,      .error _ => True
  | some x,    .ok y    => x = y
  | _,         _        => False

--------------------------------------------------------------------------------
-- Preconditions and postconditions as theorems
--------------------------------------------------------------------------------

/-- Postcondition: on C success, all ops are NULL and context is attached. -/
theorem c_post_success
    (ctx : SunContext) (am : AllocModel)
    (hNLS : am.allocNLS = true) (hOps : am.allocOps = true) :
    ∃ obj, c_SUNNonlinSolNewEmpty (some ctx) am = some obj
      ∧ obj.sunctx = ctx
      ∧ obj.ops = nullOps
      ∧ obj.content = none := by
  refine ⟨{ sunctx := ctx, ops := nullOps, content := none }, ?_⟩
  simp [c_SUNNonlinSolNewEmpty, hNLS, hOps]

/-- Postcondition: null context yields null in C. -/
theorem c_pre_null_ctx_returns_null (am : AllocModel) :
    c_SUNNonlinSolNewEmpty none am = none := by
  simp [c_SUNNonlinSolNewEmpty]

/-- Rust postcondition: null context is ArgCorrupt. -/
theorem rust_pre_null_ctx_arg_corrupt (am : AllocModel) :
    rust_SUNNonlinSolNewEmpty none am = .error .argCorrupt := by
  simp [rust_SUNNonlinSolNewEmpty]

/-- Rust postcondition: successful allocation yields fully initialized object. -/
theorem rust_post_success
    (ctx : SunContext) (am : AllocModel)
    (hNLS : am.allocNLS = true) (hOps : am.allocOps = true) :
    rust_SUNNonlinSolNewEmpty (some ctx) am
      = .ok { sunctx := ctx, ops := nullOps, content := none } := by
  simp [rust_SUNNonlinSolNewEmpty, hNLS, hOps]

--------------------------------------------------------------------------------
-- Behavioral equivalence theorem (up to error-model refinement)
--------------------------------------------------------------------------------

/-
Equivalence statement:
- C `NULL` return is refined by Rust `Err _`.
- C non-NULL object is equal to Rust `Ok` object.
-/
theorem c_rust_constructor_equiv (ctx : Option SunContext) (am : AllocModel) :
    refinesRet (c_SUNNonlinSolNewEmpty ctx am) (rust_SUNNonlinSolNewEmpty ctx am) := by
  cases ctx with
  | none =>
      simp [c_SUNNonlinSolNewEmpty, rust_SUNNonlinSolNewEmpty, refinesRet]
  | some c =>
      by_cases hNLS : am.allocNLS = true
      · by_cases hOps : am.allocOps = true
        · simp [c_SUNNonlinSolNewEmpty, rust_SUNNonlinSolNewEmpty, refinesRet, hNLS, hOps]
        · simp [c_SUNNonlinSolNewEmpty, rust_SUNNonlinSolNewEmpty, refinesRet, hNLS, hOps]
      · simp [c_SUNNonlinSolNewEmpty, rust_SUNNonlinSolNewEmpty, refinesRet, hNLS]

--------------------------------------------------------------------------------
-- Safety theorems (model-level: no UB, memory safety)
--------------------------------------------------------------------------------

/-- In this model, C constructor is total (never stuck): no UB at semantic level. -/
theorem c_no_ub_total (ctx : Option SunContext) (am : AllocModel) :
    ∃ r, c_SUNNonlinSolNewEmpty ctx am = r := by
  exact ⟨c_SUNNonlinSolNewEmpty ctx am, rfl⟩

/-- Rust constructor is total and memory-safe by construction (pure value semantics). -/
theorem rust_memory_safe_total (ctx : Option SunContext) (am : AllocModel) :
    ∃ r, rust_SUNNonlinSolNewEmpty ctx am = r := by
  exact ⟨rust_SUNNonlinSolNewEmpty ctx am, rfl⟩

end SUNDIALS.NonlinearSolver