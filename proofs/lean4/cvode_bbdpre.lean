/-
  Lean 4 formal specification skeleton for a fragment of SUNDIALS CVBBDPrecInit.

  Modeling choices requested by user:
  - sunrealtype  -> Float
  - indices      -> Int
  - nullable ptr -> Option α

  This is a *specification* (not executable C binding code).  It captures:
  - key data structures and nullable-pointer semantics
  - function signature and return/error behavior
  - preconditions as hypotheses
  - postconditions as theorems
  - memory-safety style properties
  - basic numerical-stability bounds relevant to dqrely
-/

namespace SUNDIALS
namespace CVODE
namespace BBDPRE

abbrev sunrealtype   := Float
abbrev sunindextype  := Int
abbrev sunbooleantype := Bool

/-- Error/status codes used by CVLS/CVBBDPrecInit (modeled). -/
inductive CVStatus where
  | CVLS_SUCCESS
  | CVLS_MEM_NULL
  | CVLS_LMEM_NULL
  | CVLS_ILL_INPUT
  | CVLS_MEM_FAIL
deriving DecidableEq, Repr

/-- Minimal NVECTOR ops needed by this code fragment. -/
structure NVectorOps where
  nvgetarraypointer : Option Unit
deriving Repr

/-- Minimal N_Vector model. -/
structure NVector where
  ops : NVectorOps
deriving Repr

/-- Placeholder callback types from C API. -/
abbrev CVLocalFn := Unit
abbrev CVCommFn  := Unit

/-- Linear solver memory block (opaque in this spec). -/
structure CVLsMem where
  alive : Bool := true
deriving Repr

/-- CVODE memory block fields used by CVBBDPrecInit fragment. -/
structure CVodeMem where
  cv_lmem : Option CVLsMem
  cv_tempv : NVector
deriving Repr

/-- Preconditioner private data (partial, from shown fragment). -/
structure CVBBDPrecData where
  cvode_mem : Option CVodeMem
  gloc      : CVLocalFn
  cfn       : CVCommFn
deriving Repr

/-- Result of initialization: status + optional allocated pdata. -/
structure InitResult where
  flag  : CVStatus
  pdata : Option CVBBDPrecData
deriving Repr

/-- Constants from C macros. -/
def MIN_INC_MULT : sunrealtype := 1000.0
def ZERO : sunrealtype := 0.0
def ONE  : sunrealtype := 1.0
def TWO  : sunrealtype := 2.0

/--
  Abstract allocator success predicate.
  In C this corresponds to `malloc(sizeof *pdata) != NULL`.
-/
def mallocOK (canAlloc : Bool) : Bool := canAlloc

/--
  Specification-level model of the shown prefix of `CVBBDPrecInit`.

  Inputs:
  - `cvode_mem` is nullable (`Option CVodeMem`) to model `void*` possibly NULL.
  - integer bandwidth/index arguments are `Int`.
  - `dqrely` is `Float`.
  - callbacks are abstract values.

  Extra param:
  - `canAlloc` models whether malloc succeeds.
-/
def CVBBDPrecInit_spec
    (cvode_mem : Option CVodeMem)
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype)
    (gloc : CVLocalFn)
    (cfn : CVCommFn)
    (canAlloc : Bool)
    : InitResult :=
  match cvode_mem with
  | none =>
      { flag := .CVLS_MEM_NULL, pdata := none }
  | some cmem =>
      match cmem.cv_lmem with
      | none =>
          { flag := .CVLS_LMEM_NULL, pdata := none }
      | some _ =>
          match cmem.cv_tempv.ops.nvgetarraypointer with
          | none =>
              { flag := .CVLS_ILL_INPUT, pdata := none }
          | some _ =>
              if mallocOK canAlloc then
                { flag := .CVLS_SUCCESS
                , pdata := some
                    { cvode_mem := cvode_mem
                    , gloc := gloc
                    , cfn := cfn } }
              else
                { flag := .CVLS_MEM_FAIL, pdata := none }

/-! ## Preconditions (as hypotheses) -/

/-- Basic domain assumptions typically required by BBD preconditioner setup. -/
def BBDInitPre
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype) : Prop :=
  0 ≤ Nlocal ∧
  0 ≤ mudq ∧ 0 ≤ mldq ∧
  0 ≤ mukeep ∧ 0 ≤ mlkeep ∧
  (0.0 ≤ dqrely)

/-- Optional stronger numerical precondition for stable finite-difference increments. -/
def DQRelyStable (dqrely : sunrealtype) : Prop :=
  0.0 ≤ dqrely ∧ dqrely ≤ 1.0

/-- Memory-safety precondition for successful path. -/
def InitSuccessPre (cvode_mem : Option CVodeMem) (canAlloc : Bool) : Prop :=
  (∃ cmem, cvode_mem = some cmem ∧
    cmem.cv_lmem.isSome ∧
    cmem.cv_tempv.ops.nvgetarraypointer.isSome) ∧
  canAlloc = true

/-! ## Postcondition theorems -/

theorem CVBBDPrecInit_null_mem_post
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype) (gloc : CVLocalFn) (cfn : CVCommFn) (canAlloc : Bool) :
    (CVBBDPrecInit_spec none Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc).flag
      = .CVLS_MEM_NULL ∧
    (CVBBDPrecInit_spec none Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc).pdata
      = none := by
  simp [CVBBDPrecInit_spec]

theorem CVBBDPrecInit_lmem_null_post
    (cmem : CVodeMem)
    (h_lmem : cmem.cv_lmem = none)
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype) (gloc : CVLocalFn) (cfn : CVCommFn) (canAlloc : Bool) :
    (CVBBDPrecInit_spec (some cmem) Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc).flag
      = .CVLS_LMEM_NULL := by
  simp [CVBBDPrecInit_spec, h_lmem]

theorem CVBBDPrecInit_bad_nvector_post
    (cmem : CVodeMem)
    (h_lmem : cmem.cv_lmem.isSome)
    (h_bad : cmem.cv_tempv.ops.nvgetarraypointer = none)
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype) (gloc : CVLocalFn) (cfn : CVCommFn) (canAlloc : Bool) :
    (CVBBDPrecInit_spec (some cmem) Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc).flag
      = .CVLS_ILL_INPUT := by
  cases hL : cmem.cv_lmem <;> simp [CVBBDPrecInit_spec] at h_lmem
  simp [CVBBDPrecInit_spec, hL, h_bad]

theorem CVBBDPrecInit_mem_fail_post
    (cmem : CVodeMem)
    (h_lmem : cmem.cv_lmem.isSome)
    (h_nv   : cmem.cv_tempv.ops.nvgetarraypointer.isSome)
    (h_alloc : canAlloc = false)
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype) (gloc : CVLocalFn) (cfn : CVCommFn) :
    (CVBBDPrecInit_spec (some cmem) Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc).flag
      = .CVLS_MEM_FAIL ∧
    (CVBBDPrecInit_spec (some cmem) Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc).pdata
      = none := by
  cases hL : cmem.cv_lmem <;> simp [Option.isSome] at h_lmem
  cases hN : cmem.cv_tempv.ops.nvgetarraypointer <;> simp [Option.isSome] at h_nv
  simp [CVBBDPrecInit_spec, hL, hN, h_alloc, mallocOK]

theorem CVBBDPrecInit_success_post
    (cmem : CVodeMem)
    (h_lmem : cmem.cv_lmem.isSome)
    (h_nv   : cmem.cv_tempv.ops.nvgetarraypointer.isSome)
    (h_alloc : canAlloc = true)
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype) (gloc : CVLocalFn) (cfn : CVCommFn) :
    let r := CVBBDPrecInit_spec (some cmem) Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc
    in r.flag = .CVLS_SUCCESS ∧
       r.pdata.isSome ∧
       (match r.pdata with
        | some pd => pd.cvode_mem = some cmem ∧ pd.gloc = gloc ∧ pd.cfn = cfn
        | none => False) := by
  cases hL : cmem.cv_lmem <;> simp [Option.isSome] at h_lmem
  cases hN : cmem.cv_tempv.ops.nvgetarraypointer <;> simp [Option.isSome] at h_nv
  simp [CVBBDPrecInit_spec, hL, hN, h_alloc, mallocOK]

/-! ## Memory safety properties -/

/-- No dangling pdata on any error return. -/
theorem CVBBDPrecInit_error_has_no_pdata
    (cvode_mem : Option CVodeMem)
    (Nlocal mudq mldq mukeep mlkeep : sunindextype)
    (dqrely : sunrealtype) (gloc : CVLocalFn) (cfn : CVCommFn) (canAlloc : Bool) :
    let r := CVBBDPrecInit_spec cvode_mem Nlocal mudq mldq mukeep mlkeep dqrely gloc cfn canAlloc
    in r.flag ≠ .CVLS_SUCCESS → r.pdata = none := by
  intro r hne
  cases cvode_mem <;> simp [CVBBDPrecInit_spec] at *
  rename_i cmem
  cases hL : cmem.cv_lmem <;> simp [CVBBDPrecInit_spec, hL] at *
  rename_i lmem
  cases hN : cmem.cv_tempv.ops.nvgetarraypointer <;> simp [CVBBDPrecInit_spec, hL, hN, mallocOK] at *
  split <;> simp at *

/-! ## Numerical stability bounds (spec-level) -/

/--
  If `dqrely` is in [0,1], then scaled increment factor `MIN_INC_MULT * dqrely`
  is nonnegative and bounded by `MIN_INC_MULT`.
-/
theorem dq_increment_bound
    (dqrely : sunrealtype)
    (h : DQRelyStable dqrely) :
    0.0 ≤ MIN_INC_MULT * dqrely ∧ MIN_INC_MULT * dqrely ≤ MIN_INC_MULT := by
  rcases h with ⟨h0, h1⟩
  constructor
  · nlinarith [h0]
  · nlinarith [h1]

end BBDPRE
end CVODE
end SUNDIALS