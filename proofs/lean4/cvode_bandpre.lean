/-
  Lean 4 formal specification skeleton for a fragment of SUNDIALS:
  CVBandPrecInit (banded preconditioner initialization).

  Modeling choices requested by user:
  - sunrealtype  -> Float
  - indices      -> Int
  - nullable ptr -> Option α

  This is a *specification* (not executable C), with:
  - type definitions approximating C structs/pointers
  - function signature with explicit preconditions
  - postcondition theorems
  - memory-safety properties
  - numerical-stability style bounds
-/

namespace SUNDIALS.CVODE.BandPrec

/-- C `sunrealtype` -/
abbrev SunRealType := Float

/-- C `sunindextype` (modeled as mathematical integer) -/
abbrev SunIndexType := Int

/-- C `sunbooleantype` -/
abbrev SunBoolType := Bool

/-- Representative CVLS return/error codes used in this snippet. -/
inductive CVLSError : Type
  | CVLS_SUCCESS
  | CVLS_MEM_NULL
  | CVLS_LMEM_NULL
  | CVLS_ILL_INPUT
  | CVLS_MEM_FAIL
deriving DecidableEq, Repr

/-- NVECTOR ops subset needed by this code path. -/
structure NVectorOps where
  /-- Presence of `N_VGetArrayPointer`; `none` means unsupported. -/
  nvgetarraypointer : Option Unit
deriving Repr

/-- Minimal NVECTOR model used by CVBandPrecInit checks. -/
structure NVector where
  ops : NVectorOps
deriving Repr

/-- CVLS linear-solver memory block (opaque here). -/
structure CVLsMem where
  dummy : Unit := ()
deriving Repr

/-- CVODE memory block subset used by CVBandPrecInit. -/
structure CVodeMem where
  /-- Attached linear solver memory (`cv_lmem`), nullable in C. -/
  cv_lmem : Option CVLsMem
  /-- Temporary vector used for NVECTOR compatibility check. -/
  cv_tempv : NVector
deriving Repr

/-- Preconditioner private data block subset. -/
structure CVBandPrecData where
  cvode_mem : CVodeMem
  N  : SunIndexType
  mu : SunIndexType
  ml : SunIndexType
deriving Repr

/-- Abstract heap/memory model for allocation reasoning. -/
structure MemState where
  /-- Whether allocator can provide one `CVBandPrecData` object. -/
  canAllocCVBandPrecData : Bool
deriving Repr

/-- Result of initialization: status code + optional allocated pdata + memory state. -/
structure InitResult where
  flag  : CVLSError
  pdata : Option CVBandPrecData
  mem   : MemState
deriving Repr

/-- Constants from C macros (modeled in Float). -/
def MIN_INC_MULT : SunRealType := 1000.0
def ZERO : SunRealType := 0.0
def ONE  : SunRealType := 1.0
def TWO  : SunRealType := 2.0

/--
  Executable spec function for the visible C control-flow fragment of `CVBandPrecInit`.
  We model `cvode_mem == NULL` as `none`.
-/
def CVBandPrecInitSpec
    (cvode_mem : Option CVodeMem)
    (N mu ml : SunIndexType)
    (σ : MemState) : InitResult :=
  match cvode_mem with
  | none =>
      { flag := .CVLS_MEM_NULL, pdata := none, mem := σ }
  | some cmem =>
      match cmem.cv_lmem with
      | none =>
          { flag := .CVLS_LMEM_NULL, pdata := none, mem := σ }
      | some _ =>
          if cmem.cv_tempv.ops.nvgetarraypointer = none then
            { flag := .CVLS_ILL_INPUT, pdata := none, mem := σ }
          else if σ.canAllocCVBandPrecData then
            { flag := .CVLS_SUCCESS
            , pdata := some { cvode_mem := cmem, N := N, mu := mu, ml := ml }
            , mem := σ
            }
          else
            { flag := .CVLS_MEM_FAIL, pdata := none, mem := σ }

/-! ## Preconditions (as hypotheses) -/

/-- Basic index-domain sanity often required by banded preconditioners. -/
def ValidBandwidthInputs (N mu ml : SunIndexType) : Prop :=
  0 ≤ N ∧ 0 ≤ mu ∧ 0 ≤ ml ∧ mu < N ∧ ml < N

/-- NVECTOR compatibility precondition for successful initialization. -/
def HasArrayPointerOp (cmem : CVodeMem) : Prop :=
  cmem.cv_tempv.ops.nvgetarraypointer ≠ none

/-- Linear solver interface attached precondition. -/
def HasLSMem (cmem : CVodeMem) : Prop :=
  cmem.cv_lmem ≠ none

/-- Allocation precondition for success path. -/
def CanAllocate (σ : MemState) : Prop :=
  σ.canAllocCVBandPrecData = true

/-- Non-null CVODE memory precondition. -/
def NonNullCVodeMem (cvode_mem : Option CVodeMem) : Prop :=
  cvode_mem ≠ none

/-- Numerical finiteness helper. -/
def IsFinite (x : Float) : Prop := x.isFinite = true

/-- Stability-style bound for perturbation increments (abstract form). -/
def StableIncrementBound (h yscale : SunRealType) : Prop :=
  IsFinite h ∧ IsFinite yscale ∧ (0.0 ≤ h) ∧ (MIN_INC_MULT * h ≤ yscale)

/-! ## Postcondition theorems -/

theorem CVBandPrecInit_null_mem_post
    (N mu ml : SunIndexType) (σ : MemState) :
    (CVBandPrecInitSpec none N mu ml σ).flag = .CVLS_MEM_NULL ∧
    (CVBandPrecInitSpec none N mu ml σ).pdata = none := by
  simp [CVBandPrecInitSpec]

theorem CVBandPrecInit_no_lmem_post
    (cmem : CVodeMem) (h : cmem.cv_lmem = none)
    (N mu ml : SunIndexType) (σ : MemState) :
    (CVBandPrecInitSpec (some cmem) N mu ml σ).flag = .CVLS_LMEM_NULL ∧
    (CVBandPrecInitSpec (some cmem) N mu ml σ).pdata = none := by
  simp [CVBandPrecInitSpec, h]

theorem CVBandPrecInit_bad_nvector_post
    (cmem : CVodeMem)
    (hls : cmem.cv_lmem ≠ none)
    (hop : cmem.cv_tempv.ops.nvgetarraypointer = none)
    (N mu ml : SunIndexType) (σ : MemState) :
    (CVBandPrecInitSpec (some cmem) N mu ml σ).flag = .CVLS_ILL_INPUT ∧
    (CVBandPrecInitSpec (some cmem) N mu ml σ).pdata = none := by
  cases hls' : cmem.cv_lmem <;> simp [CVBandPrecInitSpec, hls'] at hls
  simp [CVBandPrecInitSpec, hls', hop]

theorem CVBandPrecInit_alloc_fail_post
    (cmem : CVodeMem)
    (hls : HasLSMem cmem)
    (hop : HasArrayPointerOp cmem)
    (N mu ml : SunIndexType) (σ : MemState)
    (halloc : σ.canAllocCVBandPrecData = false) :
    (CVBandPrecInitSpec (some cmem) N mu ml σ).flag = .CVLS_MEM_FAIL ∧
    (CVBandPrecInitSpec (some cmem) N mu ml σ).pdata = none := by
  cases hlsm : cmem.cv_lmem <;> simp [HasLSMem, hlsm] at hls
  simp [CVBandPrecInitSpec, hlsm, halloc] at *
  cases hptr : cmem.cv_tempv.ops.nvgetarraypointer <;> simp [HasArrayPointerOp, hptr] at hop
  simp [CVBandPrecInitSpec, hlsm, hptr, halloc]

theorem CVBandPrecInit_success_post
    (cmem : CVodeMem)
    (hls : HasLSMem cmem)
    (hop : HasArrayPointerOp cmem)
    (hidx : ValidBandwidthInputs N mu ml)
    (σ : MemState)
    (halloc : CanAllocate σ)
    (N mu ml : SunIndexType) :
    let r := CVBandPrecInitSpec (some cmem) N mu ml σ
    r.flag = .CVLS_SUCCESS ∧
    r.pdata = some { cvode_mem := cmem, N := N, mu := mu, ml := ml } := by
  cases hlsm : cmem.cv_lmem <;> simp [HasLSMem, hlsm] at hls
  cases hptr : cmem.cv_tempv.ops.nvgetarraypointer <;> simp [HasArrayPointerOp, hptr] at hop
  simp [CVBandPrecInitSpec, hlsm, hptr, CanAllocate, halloc]

/-! ## Memory-safety properties -/

/-- On all error returns in this fragment, no pdata is produced (no dangling pointer exposure). -/
theorem CVBandPrecInit_error_implies_no_pdata
    (cvode_mem : Option CVodeMem) (N mu ml : SunIndexType) (σ : MemState) :
    let r := CVBandPrecInitSpec cvode_mem N mu ml σ
    r.flag ≠ .CVLS_SUCCESS → r.pdata = none := by
  intro r hne
  cases cvode_mem with
  | none =>
      simp [CVBandPrecInitSpec] at *
  | some cmem =>
      cases hls : cmem.cv_lmem <;> simp [CVBandPrecInitSpec, hls] at *
      cases hptr : cmem.cv_tempv.ops.nvgetarraypointer <;> simp [CVBandPrecInitSpec, hls, hptr] at *
      by_cases halloc : σ.canAllocCVBandPrecData
      · simp [CVBandPrecInitSpec, hls, hptr, halloc] at hne
      · simp [CVBandPrecInitSpec, hls, hptr, halloc]

/-- Success implies pdata fields are initialized exactly from inputs (no uninitialized read). -/
theorem CVBandPrecInit_success_fields_exact
    (cmem : CVodeMem) (N mu ml : SunIndexType) (σ : MemState)
    (h : (CVBandPrecInitSpec (some cmem) N mu ml σ).flag = .CVLS_SUCCESS) :
    (CVBandPrecInitSpec (some cmem) N mu ml σ).pdata =
      some { cvode_mem := cmem, N := N, mu := mu, ml := ml } := by
  unfold CVBandPrecInitSpec at h ⊢
  cases hls : cmem.cv_lmem <;> simp [hls] at h
  cases hptr : cmem.cv_tempv.ops.nvgetarraypointer <;> simp [hptr] at h
  cases halloc : σ.canAllocCVBandPrecData <;> simp [halloc] at h ⊢

/-! ## Numerical stability bounds (spec-level obligations) -/

/--
  Example theorem schema: if caller provides finite/nonnegative step and scale
  satisfying the MIN_INC_MULT lower-bound relation, the bound is preserved as a
  reusable postcondition fact for downstream DQ-Jacobian routines.
-/
theorem stable_increment_bound_preserved
    (h yscale : SunRealType)
    (hs : StableIncrementBound h yscale) :
    MIN_INC_MULT * h ≤ yscale := by
  exact hs.2.2

end SUNDIALS.CVODE.BandPrec