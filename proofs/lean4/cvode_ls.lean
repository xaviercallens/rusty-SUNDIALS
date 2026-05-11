/-
  Lean 4 formal specification skeleton for (partial) SUNDIALS CVODE LS interface code.

  Scope:
  * Models the visible semantics of CVodeSetLinearSolver from the provided C snippet.
  * Uses:
      - Float for sunrealtype
      - Int for indices
      - Option for nullable pointers
  * Encodes preconditions as hypotheses and postconditions as theorems.
  * Includes memory-safety style predicates and numerical-stability bounds.

  Note:
  This is a specification model, not an implementation proof of the C code.
-/

namespace SUNDIALS.CVODE.LS

/- Basic aliases requested by user -/
abbrev sunrealtype := Float
abbrev Index := Int

/-- C-style booleans. -/
abbrev sunbooleantype := Bool

/-- Return/error codes (modeled as Int, like C enums/macros). -/
abbrev RetCode := Int

/-- Selected CVLS return codes used in this snippet. -/
def CVLS_SUCCESS    : RetCode := 0
def CVLS_MEM_NULL   : RetCode := -1
def CVLS_ILL_INPUT  : RetCode := -2

/-- Linear solver type tags (modeled from SUNLinearSolver_Type). -/
inductive SUNLinearSolverType where
  | direct
  | iterative
  | matrixEmbedded
  | other
deriving DecidableEq, Repr

/-- N_Vector operations required by this function. -/
structure NVectorOps where
  nvconst    : Option Unit
  nvwrmsnorm : Option Unit
deriving Repr

/-- N_Vector object. -/
structure NVector where
  ops : NVectorOps
deriving Repr

/-- SUNLinearSolver operations required by this function. -/
structure SUNLinearSolverOps where
  gettype : Option Unit
  solve   : Option Unit
deriving Repr

/-- SUNLinearSolver object. -/
structure SUNLinearSolver where
  ops    : SUNLinearSolverOps
  lsType : SUNLinearSolverType
deriving Repr

/-- Opaque SUNMatrix placeholder. -/
structure SUNMatrix where
  tag : Int := 0
deriving Repr

/-- CVODE memory object (only fields needed by snippet). -/
structure CVodeMem where
  cv_tempv : NVector
deriving Repr

/-- Derived flags from LS type in the C code. -/
def iterativeFlag (t : SUNLinearSolverType) : Bool :=
  t != SUNLinearSolverType.direct

def matrixBasedFlag (t : SUNLinearSolverType) : Bool :=
  (t != SUNLinearSolverType.iterative) && (t != SUNLinearSolverType.matrixEmbedded)

/-- Required LS interface compatibility check. -/
def lsHasRequiredOps (ls : SUNLinearSolver) : Prop :=
  ls.ops.gettype.isSome ∧ ls.ops.solve.isSome

/-- Required N_Vector interface compatibility check. -/
def nvecHasRequiredOps (v : NVector) : Prop :=
  v.ops.nvconst.isSome ∧ v.ops.nvwrmsnorm.isSome

/-- Matrix-embedded consistency condition from snippet:
    if LS is matrix-embedded, A must be NULL (none). -/
def matrixEmbeddedAConsistent (ls : SUNLinearSolver) (A : Option SUNMatrix) : Prop :=
  ls.lsType = SUNLinearSolverType.matrixEmbedded → A = none

/-- Memory-safety predicate: all dereferenced nested fields are present. -/
def memorySafeForCVodeSetLinearSolver
    (cvode_mem : Option CVodeMem) (LS : Option SUNLinearSolver) : Prop :=
  match cvode_mem, LS with
  | some cv, some ls =>
      lsHasRequiredOps ls ∧ nvecHasRequiredOps cv.cv_tempv
  | _, _ => True

/-- Numerical constants from file (modeled as Float). -/
def MIN_INC_MULT : sunrealtype := 1000.0
def MAX_DQITERS  : Int := 3
def ZERO         : sunrealtype := 0.0
def PT25         : sunrealtype := 0.25
def ONE          : sunrealtype := 1.0
def TWO          : sunrealtype := 2.0

/-- Basic numerical-stability sanity bounds for constants. -/
def numericalConstantBounds : Prop :=
  (ZERO ≤ PT25) ∧ (PT25 ≤ ONE) ∧ (ONE ≤ TWO) ∧ (0.0 < MIN_INC_MULT) ∧ (0 < MAX_DQITERS)

/-- Abstract state transition result for CVodeSetLinearSolver. -/
structure CVodeSetLinearSolverResult where
  ret         : RetCode
  iterative   : Bool
  matrixbased : Bool
deriving Repr

/--
  Specification-level model of CVodeSetLinearSolver behavior for the shown snippet.
  This is a pure model returning derived flags and return code.
-/
def CVodeSetLinearSolver_spec
    (cvode_mem : Option CVodeMem)
    (LS        : Option SUNLinearSolver)
    (A         : Option SUNMatrix) : CVodeSetLinearSolverResult :=
  match cvode_mem, LS with
  | none, _ =>
      { ret := CVLS_MEM_NULL, iterative := false, matrixbased := false }
  | some _, none =>
      { ret := CVLS_ILL_INPUT, iterative := false, matrixbased := false }
  | some cv, some ls =>
      if hls : ¬ lsHasRequiredOps ls then
        { ret := CVLS_ILL_INPUT, iterative := false, matrixbased := false }
      else
        let it := iterativeFlag ls.lsType
        let mb := matrixBasedFlag ls.lsType
        if hv : ¬ nvecHasRequiredOps cv.cv_tempv then
          { ret := CVLS_ILL_INPUT, iterative := it, matrixbased := mb }
        else if hm : ¬ matrixEmbeddedAConsistent ls A then
          -- In full C source this path is also ill-input.
          { ret := CVLS_ILL_INPUT, iterative := it, matrixbased := mb }
        else
          { ret := CVLS_SUCCESS, iterative := it, matrixbased := mb }

/-! ## Postcondition theorems -/

/-- If cvode_mem is NULL, function returns CVLS_MEM_NULL. -/
theorem post_cvode_mem_null
    (LS : Option SUNLinearSolver) (A : Option SUNMatrix) :
    (CVodeSetLinearSolver_spec none LS A).ret = CVLS_MEM_NULL := by
  simp [CVodeSetLinearSolver_spec, CVLS_MEM_NULL]

/-- If LS is NULL and cvode_mem is non-NULL, function returns CVLS_ILL_INPUT. -/
theorem post_ls_null
    (cv : CVodeMem) (A : Option SUNMatrix) :
    (CVodeSetLinearSolver_spec (some cv) none A).ret = CVLS_ILL_INPUT := by
  simp [CVodeSetLinearSolver_spec, CVLS_ILL_INPUT]

/-- Missing required LS ops implies ill-input. -/
theorem post_missing_ls_ops
    (cv : CVodeMem) (ls : SUNLinearSolver) (A : Option SUNMatrix)
    (h : ¬ lsHasRequiredOps ls) :
    (CVodeSetLinearSolver_spec (some cv) (some ls) A).ret = CVLS_ILL_INPUT := by
  simp [CVodeSetLinearSolver_spec, h, CVLS_ILL_INPUT]

/-- Missing required NVector ops implies ill-input (assuming LS ops are present). -/
theorem post_bad_nvector_ops
    (cv : CVodeMem) (ls : SUNLinearSolver) (A : Option SUNMatrix)
    (hls : lsHasRequiredOps ls)
    (hv  : ¬ nvecHasRequiredOps cv.cv_tempv) :
    (CVodeSetLinearSolver_spec (some cv) (some ls) A).ret = CVLS_ILL_INPUT := by
  simp [CVodeSetLinearSolver_spec, hls, hv, CVLS_ILL_INPUT]

/-- Matrix-embedded LS with non-NULL A implies ill-input. -/
theorem post_matrix_embedded_requires_null_A
    (cv : CVodeMem) (ls : SUNLinearSolver) (A : Option SUNMatrix)
    (hls : lsHasRequiredOps ls)
    (hv  : nvecHasRequiredOps cv.cv_tempv)
    (hm  : ¬ matrixEmbeddedAConsistent ls A) :
    (CVodeSetLinearSolver_spec (some cv) (some ls) A).ret = CVLS_ILL_INPUT := by
  simp [CVodeSetLinearSolver_spec, hls, hv, hm, CVLS_ILL_INPUT]

/-- Success characterization for the modeled checks. -/
theorem post_success_of_all_preconditions
    (cv : CVodeMem) (ls : SUNLinearSolver) (A : Option SUNMatrix)
    (hls : lsHasRequiredOps ls)
    (hv  : nvecHasRequiredOps cv.cv_tempv)
    (hm  : matrixEmbeddedAConsistent ls A) :
    (CVodeSetLinearSolver_spec (some cv) (some ls) A).ret = CVLS_SUCCESS := by
  simp [CVodeSetLinearSolver_spec, hls, hv, hm, CVLS_SUCCESS]

/-- On successful path, iterative flag equals C-derived condition. -/
theorem post_iterative_flag_on_success
    (cv : CVodeMem) (ls : SUNLinearSolver) (A : Option SUNMatrix)
    (hls : lsHasRequiredOps ls)
    (hv  : nvecHasRequiredOps cv.cv_tempv)
    (hm  : matrixEmbeddedAConsistent ls A) :
    (CVodeSetLinearSolver_spec (some cv) (some ls) A).iterative = iterativeFlag ls.lsType := by
  simp [CVodeSetLinearSolver_spec, hls, hv, hm]

/-- On successful path, matrixbased flag equals C-derived condition. -/
theorem post_matrixbased_flag_on_success
    (cv : CVodeMem) (ls : SUNLinearSolver) (A : Option SUNMatrix)
    (hls : lsHasRequiredOps ls)
    (hv  : nvecHasRequiredOps cv.cv_tempv)
    (hm  : matrixEmbeddedAConsistent ls A) :
    (CVodeSetLinearSolver_spec (some cv) (some ls) A).matrixbased = matrixBasedFlag ls.lsType := by
  simp [CVodeSetLinearSolver_spec, hls, hv, hm]

/-- Memory safety theorem: if preconditions hold, all dereferences are safe in model. -/
theorem memory_safety_under_preconditions
    (cv : CVodeMem) (ls : SUNLinearSolver) :
    lsHasRequiredOps ls →
    nvecHasRequiredOps cv.cv_tempv →
    memorySafeForCVodeSetLinearSolver (some cv) (some ls) := by
  intro hls hv
  simp [memorySafeForCVodeSetLinearSolver, hls, hv]

/-- Numerical stability bounds for constants hold. -/
theorem numerical_bounds_hold : numericalConstantBounds := by
  native_decide

end SUNDIALS.CVODE.LS