/-
  Lean 4 formal specification for a fragment of SUNDIALS CVODE nonlinear solver
  interface (CVodeSetNonlinearSolver).

  Notes:
  * This is a specification model, not an executable implementation.
  * We model:
      - sunrealtype as Float
      - indices as Int
      - nullable pointers as Option
  * Preconditions are expressed as hypotheses in theorem statements.
  * Postconditions include return-code behavior, state updates, memory safety,
    and simple numerical-stability bounds for constants used in this file.
-/

namespace SUNDIALS.CVODE

/-- SUNDIALS scalar real type. -/
abbrev SunRealType := Float

/-- Index type used in specs. -/
abbrev Index := Int

/-- Boolean type used by SUNDIALS C API. -/
abbrev SunBool := Bool

/-- Return/status codes (subset needed for this spec). -/
inductive CVRet : Type
  | CV_SUCCESS
  | CV_MEM_NULL
  | CV_ILL_INPUT
  deriving DecidableEq, Repr

/-- Nonlinear solver kind. -/
inductive NLSKind : Type
  | rootfind
  | fixedpoint
  | invalid
  deriving DecidableEq, Repr

/-- Required nonlinear-solver operations in this C fragment. -/
structure NLSOps where
  gettype  : Bool
  solve    : Bool
  setsysfn : Bool
  deriving DecidableEq, Repr

/-- Abstract nonlinear solver object. -/
structure SUNNonlinearSolver where
  ops      : NLSOps
  kind     : NLSKind
  sysFnSet : Bool
  deriving DecidableEq, Repr

/-- CVODE memory object (subset relevant to this function). -/
structure CVodeMem where
  NLS    : Option SUNNonlinearSolver
  ownNLS : SunBool
  deriving DecidableEq, Repr

/-- Abstract "free" operation result model. -/
def SUNNonlinSolFree (_ : SUNNonlinearSolver) : CVRet := CVRet.CV_SUCCESS

/-- Abstract "set system function" model. -/
def SUNNonlinSolSetSysFn (nls : SUNNonlinearSolver) : CVRet :=
  if nls.ops.setsysfn then CVRet.CV_SUCCESS else CVRet.CV_ILL_INPUT

/-- Required-ops predicate from C checks:
    gettype != NULL && solve != NULL && setsysfn != NULL. -/
def hasRequiredOps (nls : SUNNonlinearSolver) : Prop :=
  nls.ops.gettype = true ∧ nls.ops.solve = true ∧ nls.ops.setsysfn = true

/-- Numerical constants from the file. -/
def ONE : SunRealType := 1.0
def NLS_MAXCOR : Int := 3
def CRDOWN : SunRealType := 0.3
def RDIV : SunRealType := 2.0

/-- Numerical-stability sanity bounds for nonlinear iteration constants. -/
def StableNLSConstants : Prop :=
  (0.0 < CRDOWN) ∧ (CRDOWN < 1.0) ∧ (1.0 < RDIV) ∧ (0 < NLS_MAXCOR)

/-- State transition result for specification. -/
structure SetNLSResult where
  ret   : CVRet
  mem'  : Option CVodeMem
  deriving DecidableEq, Repr

/--
Specification-level model of `CVodeSetNonlinearSolver`.

C signature:
`int CVodeSetNonlinearSolver(void* cvode_mem, SUNNonlinearSolver NLS)`

Modeled with nullable pointers as `Option`.
-/
def CVodeSetNonlinearSolver_spec
  (cvode_mem : Option CVodeMem)
  (NLS       : Option SUNNonlinearSolver) : SetNLSResult :=
  match cvode_mem with
  | none =>
      { ret := CVRet.CV_MEM_NULL, mem' := none }
  | some mem =>
      match NLS with
      | none =>
          { ret := CVRet.CV_ILL_INPUT, mem' := some mem }
      | some nls =>
          if hreq : hasRequiredOps nls then
            let _freeret :=
              match mem.NLS, mem.ownNLS with
              | some old, true  => SUNNonlinSolFree old
              | _, _            => CVRet.CV_SUCCESS
            let mem1 : CVodeMem := { mem with NLS := some nls, ownNLS := false }
            match nls.kind with
            | NLSKind.rootfind =>
                let r := SUNNonlinSolSetSysFn nls
                if r = CVRet.CV_SUCCESS then
                  { ret := CVRet.CV_SUCCESS, mem' := some mem1 }
                else
                  { ret := CVRet.CV_ILL_INPUT, mem' := some mem1 }
            | NLSKind.fixedpoint =>
                let r := SUNNonlinSolSetSysFn nls
                if r = CVRet.CV_SUCCESS then
                  { ret := CVRet.CV_SUCCESS, mem' := some mem1 }
                else
                  { ret := CVRet.CV_ILL_INPUT, mem' := some mem1 }
            | NLSKind.invalid =>
                { ret := CVRet.CV_ILL_INPUT, mem' := some mem }
          else
            { ret := CVRet.CV_ILL_INPUT, mem' := some mem }

/-! ### Postcondition theorems (preconditions as hypotheses) -/

/-- Null CVODE memory pointer returns `CV_MEM_NULL` and preserves null state. -/
theorem CVodeSetNonlinearSolver_null_mem
  (nls : Option SUNNonlinearSolver) :
  (CVodeSetNonlinearSolver_spec none nls).ret = CVRet.CV_MEM_NULL ∧
  (CVodeSetNonlinearSolver_spec none nls).mem' = none := by
  simp [CVodeSetNonlinearSolver_spec]

/-- Non-null memory but null NLS pointer returns `CV_ILL_INPUT` and leaves memory unchanged. -/
theorem CVodeSetNonlinearSolver_null_nls
  (mem : CVodeMem) :
  (CVodeSetNonlinearSolver_spec (some mem) none).ret = CVRet.CV_ILL_INPUT ∧
  (CVodeSetNonlinearSolver_spec (some mem) none).mem' = some mem := by
  simp [CVodeSetNonlinearSolver_spec]

/-- Missing required NLS ops returns `CV_ILL_INPUT` and leaves memory unchanged. -/
theorem CVodeSetNonlinearSolver_missing_required_ops
  (mem : CVodeMem) (nls : SUNNonlinearSolver)
  (h : ¬ hasRequiredOps nls) :
  (CVodeSetNonlinearSolver_spec (some mem) (some nls)).ret = CVRet.CV_ILL_INPUT ∧
  (CVodeSetNonlinearSolver_spec (some mem) (some nls)).mem' = some mem := by
  simp [CVodeSetNonlinearSolver_spec, h]

/-- Invalid nonlinear solver type returns `CV_ILL_INPUT` and leaves memory unchanged. -/
theorem CVodeSetNonlinearSolver_invalid_type
  (mem : CVodeMem) (nls : SUNNonlinearSolver)
  (hreq : hasRequiredOps nls)
  (ht : nls.kind = NLSKind.invalid) :
  (CVodeSetNonlinearSolver_spec (some mem) (some nls)).ret = CVRet.CV_ILL_INPUT ∧
  (CVodeSetNonlinearSolver_spec (some mem) (some nls)).mem' = some mem := by
  simp [CVodeSetNonlinearSolver_spec, hreq, ht]

/--
Successful attach for valid type and required ops:
- return `CV_SUCCESS`
- `NLS` pointer updated
- ownership flag set to false
- memory remains allocated (non-null)
-/
theorem CVodeSetNonlinearSolver_success_rootfind
  (mem : CVodeMem) (nls : SUNNonlinearSolver)
  (hreq : hasRequiredOps nls)
  (ht : nls.kind = NLSKind.rootfind) :
  (CVodeSetNonlinearSolver_spec (some mem) (some nls)).ret = CVRet.CV_SUCCESS →
  ∃ mem',
    (CVodeSetNonlinearSolver_spec (some mem) (some nls)).mem' = some mem' ∧
    mem'.NLS = some nls ∧
    mem'.ownNLS = false := by
  intro hsucc
  simp [CVodeSetNonlinearSolver_spec, hreq, ht] at hsucc ⊢
  refine ⟨{ mem with NLS := some nls, ownNLS := false }, ?_⟩
  simp

/-- Same success shape for fixed-point solver type. -/
theorem CVodeSetNonlinearSolver_success_fixedpoint
  (mem : CVodeMem) (nls : SUNNonlinearSolver)
  (hreq : hasRequiredOps nls)
  (ht : nls.kind = NLSKind.fixedpoint) :
  (CVodeSetNonlinearSolver_spec (some mem) (some nls)).ret = CVRet.CV_SUCCESS →
  ∃ mem',
    (CVodeSetNonlinearSolver_spec (some mem) (some nls)).mem' = some mem' ∧
    mem'.NLS = some nls ∧
    mem'.ownNLS = false := by
  intro hsucc
  simp [CVodeSetNonlinearSolver_spec, hreq, ht] at hsucc ⊢
  refine ⟨{ mem with NLS := some nls, ownNLS := false }, ?_⟩
  simp

/-- Memory safety: function never dereferences null memory in the model. -/
theorem CVodeSetNonlinearSolver_memory_safety
  (cvode_mem : Option CVodeMem) (nls : Option SUNNonlinearSolver) :
  cvode_mem = none →
  (CVodeSetNonlinearSolver_spec cvode_mem nls).ret = CVRet.CV_MEM_NULL := by
  intro h
  simp [h, CVodeSetNonlinearSolver_spec]

/-- Numerical constants satisfy expected stability bounds. -/
theorem nonlinear_constants_stable : StableNLSConstants := by
  native_decide

end SUNDIALS.CVODE