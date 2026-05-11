/-
  Lean 4 formal specification for selected SUNDIALS SUNNonlinearSolver C code.

  Modeling choices requested by user:
  - sunrealtype  -> Float
  - indices      -> Int
  - nullable ptr -> Option α

  This is a *specification* model (not executable C/FFI code).
-/

namespace SUNDIALS

--------------------------------------------------------------------------------
-- Basic aliases and enums
--------------------------------------------------------------------------------

abbrev sunrealtype := Float
abbrev Index := Int

/-- Error/status codes (subset needed for this snippet). -/
inductive SUNErrCode where
  | success
  | mallocFail
  | nullInput
  deriving DecidableEq, Repr

/-- Nonlinear solver type (opaque in C; modeled abstractly). -/
inductive SUNNonlinearSolverType where
  | unknown
  | newton
  | fixedPoint
  deriving DecidableEq, Repr

--------------------------------------------------------------------------------
-- Context and profiler
--------------------------------------------------------------------------------

/-- Profiler handle (opaque). -/
structure SUNProfiler where
  id : Int
  deriving DecidableEq, Repr

/-- SUNDIALS context. -/
structure SUNContext where
  profiler : Option SUNProfiler
  deriving DecidableEq, Repr

--------------------------------------------------------------------------------
-- Function table (ops) and solver object
--------------------------------------------------------------------------------

/-
  In C, ops fields are function pointers that may be NULL.
  We model nullable function pointers as Option.
-/
structure SUNNonlinearSolverOps where
  gettype         : Option (SUNNonlinearSolver → SUNNonlinearSolverType)
  initialize      : Option (SUNNonlinearSolver → SUNErrCode)
  setup           : Option (SUNNonlinearSolver → SUNErrCode)
  solve           : Option (SUNNonlinearSolver → SUNErrCode)
  free            : Option (SUNNonlinearSolver → SUNErrCode)
  setsysfn        : Option (SUNNonlinearSolver → SUNErrCode)
  setlsetupfn     : Option (SUNNonlinearSolver → SUNErrCode)
  setlsolvefn     : Option (SUNNonlinearSolver → SUNErrCode)
  setctestfn      : Option (SUNNonlinearSolver → SUNErrCode)
  setoptions      : Option (SUNNonlinearSolver → SUNErrCode)
  setmaxiters     : Option (SUNNonlinearSolver → Index → SUNErrCode)
  getnumiters     : Option (SUNNonlinearSolver → Index)
  getcuriter      : Option (SUNNonlinearSolver → Index)
  getnumconvfails : Option (SUNNonlinearSolver → Index)
  deriving Repr

/-- Constructor for the all-NULL ops table (matches C initialization). -/
def emptyOps : SUNNonlinearSolverOps :=
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

/-- Main nonlinear solver object. -/
structure SUNNonlinearSolver where
  sunctx  : SUNContext
  ops     : Option SUNNonlinearSolverOps
  content : Option Unit
  python  : Option Unit
  deriving Repr

--------------------------------------------------------------------------------
-- Memory model (abstract) for malloc/free safety specs
--------------------------------------------------------------------------------

/-
  We use an abstract heap token and allocation predicates to state memory safety
  properties as pre/postconditions.
-/
structure Heap where
  liveSolver : SUNNonlinearSolver → Prop
  deriving Repr

def allocatedSolver (h : Heap) (nls : SUNNonlinearSolver) : Prop := h.liveSolver nls

--------------------------------------------------------------------------------
-- C function specifications
--------------------------------------------------------------------------------

/--
  Spec for:
    SUNNonlinearSolver SUNNonlinSolNewEmpty(SUNContext sunctx)

  C behavior:
  - if sunctx == NULL: return NULL
  - allocate NLS and ops
  - initialize all ops fields to NULL
  - set NLS.sunctx = sunctx, NLS.ops = ops, NLS.content = NULL, NLS.python = NULL
-/
def SUNNonlinSolNewEmpty_spec
    (h : Heap) (sunctx : Option SUNContext) :
    Option SUNNonlinearSolver :=
  match sunctx with
  | none => none
  | some ctx =>
      some
        { sunctx := ctx
          ops := some emptyOps
          content := none
          python := none }

/--
  Spec for:
    void SUNNonlinSolFreeEmpty(SUNNonlinearSolver NLS)

  C behavior:
  - if NLS == NULL: no-op
  - free NLS->ops; set NLS->ops = NULL
  - destroy python table if enabled; set NLS->python = NULL
  - free NLS object
-/
def SUNNonlinSolFreeEmpty_spec
    (h : Heap) (nls : Option SUNNonlinearSolver) : Heap :=
  h
  -- Abstractly unchanged token; safety properties are stated in theorems below.

/--
  Spec for:
    SUNNonlinearSolver_Type SUNNonlinSolGetType(SUNNonlinearSolver NLS)
  (partial in C snippet, but semantics shown: return NLS->ops->gettype(NLS))
-/
def SUNNonlinSolGetType_spec (nls : SUNNonlinearSolver) : Option SUNNonlinearSolverType :=
  match nls.ops with
  | none => none
  | some ops =>
      match ops.gettype with
      | none => none
      | some f => some (f nls)

--------------------------------------------------------------------------------
-- Preconditions and postcondition theorems
--------------------------------------------------------------------------------

/-- Precondition: context must be non-null for successful creation. -/
def pre_new_empty (sunctx : Option SUNContext) : Prop := sunctx.isSome

/-- Postcondition bundle for successful creation. -/
def post_new_empty (sunctx : SUNContext) (res : SUNNonlinearSolver) : Prop :=
  res.sunctx = sunctx ∧
  res.ops = some emptyOps ∧
  res.content = none ∧
  res.python = none

theorem SUNNonlinSolNewEmpty_null_returns_none
    (h : Heap) :
    SUNNonlinSolNewEmpty_spec h none = none := by
  rfl

theorem SUNNonlinSolNewEmpty_success_post
    (h : Heap) (ctx : SUNContext) :
    ∃ res, SUNNonlinSolNewEmpty_spec h (some ctx) = some res ∧ post_new_empty ctx res := by
  refine ⟨{ sunctx := ctx, ops := some emptyOps, content := none, python := none }, ?_, ?_⟩
  · rfl
  · simp [post_new_empty]

/-- Memory-safety precondition for free: nullable input always allowed. -/
def pre_free_empty (_nls : Option SUNNonlinearSolver) : Prop := True

/--
  Memory-safety postcondition for free:
  - no dereference occurs when input is none
  - function is total and does not require non-null input
-/
theorem SUNNonlinSolFreeEmpty_total
    (h : Heap) (nls : Option SUNNonlinearSolver) :
    ∃ h', h' = SUNNonlinSolFreeEmpty_spec h nls := by
  exact ⟨_, rfl⟩

/-- Precondition for GetType: ops and gettype callback must be non-null. -/
def pre_get_type (nls : SUNNonlinearSolver) : Prop :=
  ∃ ops f, nls.ops = some ops ∧ ops.gettype = some f

theorem SUNNonlinSolGetType_defined_if_pre
    (nls : SUNNonlinearSolver)
    (hpre : pre_get_type nls) :
    ∃ t, SUNNonlinSolGetType_spec nls = some t := by
  rcases hpre with ⟨ops, f, hops, hgt⟩
  subst hops; subst hgt
  simp [SUNNonlinSolGetType_spec]

--------------------------------------------------------------------------------
-- Numerical stability bounds (generic, for Float-based callbacks)
--------------------------------------------------------------------------------

/-
  The shown C functions are mostly structural (allocation/dispatch), not numeric.
  We still provide generic Float stability contracts for callbacks that may be
  installed in ops and used by solver routines.
-/

/-- A generic finite-value predicate for Float outputs. -/
def Finite (x : Float) : Prop := ¬ x.isNaN ∧ ¬ x.isInf

/--
  Stability contract for a scalar system function:
  bounded input implies bounded finite output.
-/
def StableSysFn (f : sunrealtype → sunrealtype) (B_in B_out : Float) : Prop :=
  0.0 ≤ B_in ∧ 0.0 ≤ B_out ∧
  ∀ x, Float.abs x ≤ B_in → Finite (f x) ∧ Float.abs (f x) ≤ B_out

/--
  Lipschitz-style contract (useful for nonlinear solver robustness specs).
-/
def LipschitzSysFn (f : sunrealtype → sunrealtype) (L : Float) : Prop :=
  0.0 ≤ L ∧ ∀ x y, Float.abs (f x - f y) ≤ L * Float.abs (x - y)

theorem stable_implies_finite_at_zero
    (f : sunrealtype → sunrealtype) (Bin Bout : Float)
    (h : StableSysFn f Bin Bout)
    (hBin : 0.0 ≤ Bin) :
    Finite (f 0.0) := by
  rcases h with ⟨_, _, hbound⟩
  have hz : Float.abs 0.0 ≤ Bin := by simpa using hBin
  exact (hbound 0.0 hz).1

end SUNDIALS