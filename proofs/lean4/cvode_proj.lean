/-
  Lean 4 formal specification for selected SUNDIALS CVODE projection APIs.

  Scope modeled from C snippet:
  - CVodeSetProjFn
  - CVodeSetProjErrEst
  - (signature only) CVodeSetProjFrequency

  Modeling choices requested:
  - sunrealtype -> Float
  - indices -> Int
  - nullable pointers -> Option
-/

namespace SUNDIALS.CVODE.ProjSpec

/-- C `sunrealtype` modeled as Lean `Float`. -/
abbrev SunReal := Float

/-- C integer-like indices modeled as Lean `Int`. -/
abbrev Index := Int

/-- Boolean type used in SUNDIALS. -/
abbrev SunBool := Bool

/-- Return/status codes (subset used by the snippet). -/
inductive CVRet : Type
  | CV_SUCCESS
  | CV_MEM_NULL
  | CV_ILL_INPUT
  | CV_MEM_FAIL
deriving DecidableEq, Repr

/-- Linear multistep method tag (subset). -/
inductive LMM : Type
  | CV_BDF
  | CV_ADAMS
deriving DecidableEq, Repr

/--
  Projection callback type.
  In C this is a function pointer (`CVProjFn`); here we abstract as a pure function.
-/
abbrev CVProjFn := SunReal → SunReal

/-- Projection-memory block (`CVodeProjMem`). -/
structure CVodeProjMem where
  internal_proj : SunBool
  pfun          : Option CVProjFn
  err_proj      : SunBool
  deriving Repr

/-- Main CVODE memory block (`CVodeMem`). -/
structure CVodeMem where
  cv_lmm       : LMM
  proj_mem     : Option CVodeProjMem
  proj_enabled : SunBool
  deriving Repr

/-- Default projection-memory values (spec-level). -/
def cvProjDefaults : CVodeProjMem :=
  { internal_proj := true
  , pfun          := none
  , err_proj      := false
  }

/--
  `cvProjCreate` spec model:
  - if projection memory exists, keep it;
  - otherwise allocate defaults.
  - never fails in this pure model (returns `CV_SUCCESS`).
-/
def cvProjCreate (pm : Option CVodeProjMem) : CVRet × Option CVodeProjMem :=
  match pm with
  | some m => (CVRet.CV_SUCCESS, some m)
  | none   => (CVRet.CV_SUCCESS, some cvProjDefaults)

/--
  Access helper corresponding to `cvAccessProjMem`.
  Requires non-null cvode memory and existing projection memory.
-/
def cvAccessProjMem (cvode_mem : Option CVodeMem) : CVRet × Option CVodeMem × Option CVodeProjMem :=
  match cvode_mem with
  | none => (CVRet.CV_MEM_NULL, none, none)
  | some cm =>
    match cm.proj_mem with
    | none      => (CVRet.CV_ILL_INPUT, some cm, none)
    | some pmem => (CVRet.CV_SUCCESS, some cm, some pmem)

/--
  Spec model of C function:

  `int CVodeSetProjFn(void* cvode_mem, CVProjFn pfun)`
-/
def CVodeSetProjFn (cvode_mem : Option CVodeMem) (pfun : Option CVProjFn) :
    CVRet × Option CVodeMem :=
  match cvode_mem with
  | none => (CVRet.CV_MEM_NULL, none)
  | some cm =>
    match pfun with
    | none => (CVRet.CV_ILL_INPUT, some cm)
    | some f =>
      if hBDF : cm.cv_lmm = LMM.CV_BDF then
        let (rCreate, pm') := cvProjCreate cm.proj_mem
        match rCreate, pm' with
        | CVRet.CV_SUCCESS, some pm =>
          let pm2 : CVodeProjMem :=
            { pm with internal_proj := false, pfun := some f }
          let cm2 : CVodeMem :=
            { cm with proj_mem := some pm2, proj_enabled := true }
          (CVRet.CV_SUCCESS, some cm2)
        | _, _ => (CVRet.CV_MEM_FAIL, some cm)
      else
        (CVRet.CV_ILL_INPUT, some cm)

/--
  Spec model of C function:

  `int CVodeSetProjErrEst(void* cvode_mem, sunbooleantype onoff)`
-/
def CVodeSetProjErrEst (cvode_mem : Option CVodeMem) (onoff : SunBool) :
    CVRet × Option CVodeMem :=
  let (ra, cm?, pm?) := cvAccessProjMem cvode_mem
  match ra, cm?, pm? with
  | CVRet.CV_SUCCESS, some cm, some pm =>
    let pm2 : CVodeProjMem := { pm with err_proj := onoff }
    let cm2 : CVodeMem := { cm with proj_mem := some pm2 }
    (CVRet.CV_SUCCESS, some cm2)
  | r, cm, _ => (r, cm)

/--
  Signature-level spec for:

  `int CVodeSetProjFrequency(void* cvode_mem, long int freq)`

  (Body not present in snippet; we provide a conservative contract placeholder.)
-/
def CVodeSetProjFrequency (cvode_mem : Option CVodeMem) (freq : Int) :
    CVRet × Option CVodeMem :=
  -- Placeholder: require valid memory and positive frequency.
  match cvode_mem with
  | none => (CVRet.CV_MEM_NULL, none)
  | some cm =>
    if freq ≤ 0 then (CVRet.CV_ILL_INPUT, some cm)
    else
      -- No state field for frequency in this snippet model.
      (CVRet.CV_SUCCESS, some cm)

/-! ## Preconditions as hypotheses and postcondition theorems -/

/-- Precondition bundle for successful `CVodeSetProjFn`. -/
structure Pre_CVodeSetProjFn (cvode_mem : Option CVodeMem) (pfun : Option CVProjFn) : Prop where
  h_mem_nonnull : ∃ cm, cvode_mem = some cm
  h_pfun_nonnull : ∃ f, pfun = some f
  h_bdf : ∀ cm, cvode_mem = some cm → cm.cv_lmm = LMM.CV_BDF

/-- Postcondition: success code under valid preconditions. -/
theorem CVodeSetProjFn_success_code
    (cvode_mem : Option CVodeMem) (pfun : Option CVProjFn)
    (hpre : Pre_CVodeSetProjFn cvode_mem pfun) :
    (CVodeSetProjFn cvode_mem pfun).fst = CVRet.CV_SUCCESS := by
  rcases hpre.h_mem_nonnull with ⟨cm, rfl⟩
  rcases hpre.h_pfun_nonnull with ⟨f, rfl⟩
  simp [CVodeSetProjFn, hpre.h_bdf cm rfl, cvProjCreate]

/-- Postcondition: projection gets enabled and function stored on success. -/
theorem CVodeSetProjFn_sets_fields
    (cm : CVodeMem) (f : CVProjFn)
    (h_bdf : cm.cv_lmm = LMM.CV_BDF) :
    ∃ cm',
      CVodeSetProjFn (some cm) (some f) = (CVRet.CV_SUCCESS, some cm') ∧
      cm'.proj_enabled = true ∧
      (∃ pm, cm'.proj_mem = some pm ∧ pm.internal_proj = false ∧ pm.pfun = some f) := by
  simp [CVodeSetProjFn, h_bdf, cvProjCreate]
  refine ⟨{ cm with
      proj_mem := some { (match cm.proj_mem with | some m => m | none => cvProjDefaults) with
        internal_proj := false, pfun := some f },
      proj_enabled := true }, ?_⟩
  constructor
  · cases hpm : cm.proj_mem <;> simp [hpm, cvProjCreate]
  · constructor
    · simp
    · refine ⟨{ (match cm.proj_mem with | some m => m | none => cvProjDefaults) with
          internal_proj := false, pfun := some f }, ?_⟩
      simp

/-- Memory-safety: null input pointer yields `CV_MEM_NULL` and no dereference state. -/
theorem CVodeSetProjFn_null_mem_safe (pfun : Option CVProjFn) :
    CVodeSetProjFn none pfun = (CVRet.CV_MEM_NULL, none) := by
  simp [CVodeSetProjFn]

/-- Precondition for successful `CVodeSetProjErrEst`. -/
structure Pre_CVodeSetProjErrEst (cvode_mem : Option CVodeMem) : Prop where
  h_mem_nonnull : ∃ cm, cvode_mem = some cm
  h_projmem_exists : ∀ cm, cvode_mem = some cm → ∃ pm, cm.proj_mem = some pm

/-- Postcondition: `err_proj` is updated exactly to `onoff` on success. -/
theorem CVodeSetProjErrEst_sets_flag
    (cvode_mem : Option CVodeMem) (onoff : SunBool)
    (hpre : Pre_CVodeSetProjErrEst cvode_mem) :
    ∃ cm',
      CVodeSetProjErrEst cvode_mem onoff = (CVRet.CV_SUCCESS, some cm') ∧
      ∃ pm, cm'.proj_mem = some pm ∧ pm.err_proj = onoff := by
  rcases hpre.h_mem_nonnull with ⟨cm, rfl⟩
  rcases hpre.h_projmem_exists cm rfl with ⟨pm, hpm⟩
  subst hpm
  simp [CVodeSetProjErrEst, cvAccessProjMem]
  refine ⟨{ cm with proj_mem := some { pm with err_proj := onoff } }, ?_⟩
  simp

/-- Memory-safety for `CVodeSetProjErrEst`: null pointer is rejected. -/
theorem CVodeSetProjErrEst_null_mem_safe (onoff : SunBool) :
    CVodeSetProjErrEst none onoff = (CVRet.CV_MEM_NULL, none) := by
  simp [CVodeSetProjErrEst, cvAccessProjMem]

/-! ## Numerical stability bounds (spec-level invariants)

These APIs mostly manipulate pointers/flags, not floating arithmetic.
Still, we state non-interference bounds over stored floating callbacks.
-/

/-- If a projection function was finite-bounded before, setting `err_proj` preserves it. -/
theorem CVodeSetProjErrEst_preserves_pfun_bound
    (cm : CVodeMem) (onoff : SunBool) (B x : Float)
    (hpm : ∃ pm f, cm.proj_mem = some pm ∧ pm.pfun = some f ∧ Float.abs (f x) ≤ B) :
    ∃ cm', (CVodeSetProjErrEst (some cm) onoff).2 = some cm' ∧
      ∃ pm' f', cm'.proj_mem = some pm' ∧ pm'.pfun = some f' ∧ Float.abs (f' x) ≤ B := by
  rcases hpm with ⟨pm, f, h1, h2, hb⟩
  subst h1; subst h2
  simp [CVodeSetProjErrEst, cvAccessProjMem]
  refine ⟨{ cm with proj_mem := some { pm with err_proj := onoff } }, ?_⟩
  constructor <;> simp [hb]

end SUNDIALS.CVODE.ProjSpec