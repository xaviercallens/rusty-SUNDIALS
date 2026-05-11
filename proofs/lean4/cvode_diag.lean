/-
  Lean 4 formal specification skeleton for the shown SUNDIALS CVDiag code fragment.

  Modeling choices requested by user:
  - sunrealtype  -> Float
  - indices      -> Int
  - nullable ptr -> Option α

  This is a *specification* (not executable C binding code).
-/

namespace SUNDIALS.CVDiagSpec

--------------------------------------------------------------------------------
-- Basic constants and return codes
--------------------------------------------------------------------------------

abbrev sunrealtype := Float
abbrev Index := Int

def FRACT : sunrealtype := 0.1
def ONE   : sunrealtype := 1.0

/-- Return/status codes used by CVDiag. -/
inductive CVDiagFlag where
  | success      -- 0
  | memNull      -- CVDIAG_MEM_NULL
  | illInput     -- CVDIAG_ILL_INPUT
  | memFail      -- CVDIAG_MEM_FAIL
deriving DecidableEq, Repr

def CVDiagFlag.toInt : CVDiagFlag → Int
  | .success  => 0
  | .memNull  => -1
  | .illInput => -2
  | .memFail  => -3

--------------------------------------------------------------------------------
-- N_Vector and ops model
--------------------------------------------------------------------------------

/-- Minimal model of N_Vector operations needed by this code fragment. -/
structure NVectorOps where
  nvcompare : Option (sunrealtype → sunrealtype → Bool)
  nvinvtest : Option (sunrealtype → Option sunrealtype)

/-- Minimal model of N_Vector. -/
structure NVector where
  ops  : NVectorOps
  data : List sunrealtype

/-- Abstract clone operation (nullable pointer semantics via Option). -/
abbrev N_VClone := NVector → Option NVector

--------------------------------------------------------------------------------
-- CVDiag memory and CVode memory model
--------------------------------------------------------------------------------

structure CVDiagMem where
  di_last_flag : CVDiagFlag
  di_M         : Option NVector
  di_bit       : Option NVector
  di_bitcomp   : Option NVector

/-- Function-pointer slots are modeled abstractly by Bool "is set". -/
structure CVodeMem where
  cv_tempv      : NVector
  cv_lfree      : Option (CVodeMem → CVodeMem)   -- old lfree callback
  cv_linit_set  : Bool
  cv_lreinit_set: Bool
  cv_lsetup_set : Bool
  cv_lsolve_set : Bool
  cv_lfree_set  : Bool
  cv_lmem       : Option CVDiagMem
  setupNonNull  : Bool

--------------------------------------------------------------------------------
-- Preconditions used by CVDiag
--------------------------------------------------------------------------------

def hasRequiredNVectorOps (cv : CVodeMem) : Prop :=
  cv.cv_tempv.ops.nvcompare.isSome ∧ cv.cv_tempv.ops.nvinvtest.isSome

--------------------------------------------------------------------------------
-- State transition spec for CVDiag (fragment shown)
--------------------------------------------------------------------------------

/--
  Abstract result of CVDiag call:
  - flag: returned status code
  - out : resulting memory state if input pointer was non-null
-/
structure CVDiagResult where
  flag : CVDiagFlag
  out  : Option CVodeMem

/--
  High-level specification relation for the shown C code fragment.
  `mallocOK` models allocation success for CVDiagMemRec.
  `cloneM` models success/failure of `N_VClone(cv_tempv)` for `di_M`.
-/
def CVDiagSpec
  (cvode_mem : Option CVodeMem)
  (mallocOK : Bool)
  (cloneM : Option NVector)
  : CVDiagResult :=
  match cvode_mem with
  | none =>
      { flag := .memNull, out := none }
  | some cv =>
      if hOps : hasRequiredNVectorOps cv then
        if mallocOK then
          let cv' : CVodeMem :=
            { cv with
              cv_linit_set   := true
              cv_lreinit_set := true
              cv_lsetup_set  := true
              cv_lsolve_set  := true
              cv_lfree_set   := true
              setupNonNull   := true
              cv_lmem        := some {
                di_last_flag := .success
                di_M         := cloneM
                di_bit       := none
                di_bitcomp   := none
              }
            }
          -- In the shown fragment, failure of di_M clone would later trigger memFail.
          if cloneM.isSome then
            { flag := .success, out := some cv' }
          else
            { flag := .memFail, out := some cv' }
        else
          { flag := .memFail, out := some cv }
      else
        { flag := .illInput, out := some cv }

--------------------------------------------------------------------------------
-- Postcondition theorems
--------------------------------------------------------------------------------

theorem CVDiag_null_ptr_post
  (mallocOK : Bool) (cloneM : Option NVector) :
  (CVDiagSpec none mallocOK cloneM).flag = .memNull ∧
  (CVDiagSpec none mallocOK cloneM).out = none := by
  simp [CVDiagSpec]

theorem CVDiag_ill_input_post
  (cv : CVodeMem) (mallocOK : Bool) (cloneM : Option NVector)
  (hbad : ¬ hasRequiredNVectorOps cv) :
  (CVDiagSpec (some cv) mallocOK cloneM).flag = .illInput := by
  simp [CVDiagSpec, hbad]

theorem CVDiag_mem_fail_on_malloc_post
  (cv : CVodeMem) (cloneM : Option NVector)
  (hops : hasRequiredNVectorOps cv) :
  (CVDiagSpec (some cv) false cloneM).flag = .memFail := by
  simp [CVDiagSpec, hops]

theorem CVDiag_success_sets_function_fields
  (cv : CVodeMem) (m : NVector)
  (hops : hasRequiredNVectorOps cv) :
  let r := CVDiagSpec (some cv) true (some m)
  r.flag = .success →
  ∃ cv', r.out = some cv' ∧
    cv'.cv_linit_set = true ∧
    cv'.cv_lreinit_set = true ∧
    cv'.cv_lsetup_set = true ∧
    cv'.cv_lsolve_set = true ∧
    cv'.cv_lfree_set = true ∧
    cv'.setupNonNull = true := by
  intro r hflag
  simp [CVDiagSpec, hops] at hflag
  refine ⟨{
    cv with
    cv_linit_set := true, cv_lreinit_set := true, cv_lsetup_set := true,
    cv_lsolve_set := true, cv_lfree_set := true, setupNonNull := true,
    cv_lmem := some { di_last_flag := .success, di_M := some m, di_bit := none, di_bitcomp := none }
  }, ?_⟩
  simp [CVDiagSpec, hops]

--------------------------------------------------------------------------------
-- Memory safety properties
--------------------------------------------------------------------------------

/-- No dereference of null `cvode_mem` occurs in null branch (modeled by no output state). -/
theorem memory_safety_no_deref_on_null
  (mallocOK : Bool) (cloneM : Option NVector) :
  (CVDiagSpec none mallocOK cloneM).out = none := by
  simp [CVDiagSpec]

/-- If required ops are missing, solver memory (`cv_lmem`) is not newly allocated in this spec. -/
theorem memory_safety_no_alloc_on_bad_ops
  (cv : CVodeMem) (mallocOK : Bool) (cloneM : Option NVector)
  (hbad : ¬ hasRequiredNVectorOps cv) :
  (CVDiagSpec (some cv) mallocOK cloneM).out = some cv := by
  simp [CVDiagSpec, hbad]

--------------------------------------------------------------------------------
-- Numerical stability bounds (for constants and inverse-test style ops)
--------------------------------------------------------------------------------

theorem FRACT_nonneg : 0.0 ≤ FRACT := by native_decide
theorem FRACT_lt_ONE : FRACT < ONE := by native_decide

/--
  Generic stability-style contract for inverse test:
  if inverse exists, multiplying by original is approximately 1.
  (Abstract tolerance-based postcondition for floating-point semantics.)
-/
def InvTestStable (x : sunrealtype) (invf : sunrealtype → Option sunrealtype) (eps : sunrealtype) : Prop :=
  x ≠ 0.0 →
  match invf x with
  | none => False
  | some y => Float.abs (x * y - 1.0) ≤ eps

/-- Example boundedness requirement used by diagonal scaling logic. -/
def DiagEntryWellConditioned (d : sunrealtype) : Prop :=
  Float.abs d ≥ FRACT

end SUNDIALS.CVDiagSpec