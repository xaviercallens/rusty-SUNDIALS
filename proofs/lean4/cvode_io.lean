/-
  Lean 4 formal specification for selected CVODE optional input setters
  from SUNDIALS C code.

  Modeling choices requested:
  - sunrealtype  -> Float
  - indices      -> Int
  - nullable ptr -> Option ...
-/

namespace SUNDIALS.CVODE.Spec

/-- Return/status codes used by CVODE setters (subset). -/
abbrev CVStatus : Int := Int

def CV_SUCCESS   : CVStatus := 0
def CV_MEM_NULL  : CVStatus := -1
def CV_ILL_INPUT : CVStatus := -2

/-- Constant used by CVodeSetDeltaGammaMaxLSetup when input is negative.
    In C this is a library constant; here we keep it abstract but bounded. -/
def DGMAX_LSETUP_DEFAULT : Float := 0.2

/-- Abstract monitor callback type. -/
abbrev CVMonitorFn := Float → Int → Int

/-- Abstract user data payload. In C this is `void*`; modeled as nullable token. -/
abbrev UserData := Int

/-- CVODE memory record (subset of fields touched by shown functions). -/
structure CVodeMem where
  cv_dgmax_lsetup   : Float
  cv_user_data      : Option UserData
  cv_monitorfun     : Option CVMonitorFn
  cv_monitor_interval : Int
deriving Repr, DecidableEq

/-- Build-time flag for monitoring support (`#ifdef SUNDIALS_ENABLE_MONITORING`). -/
abbrev MonitoringEnabled := Bool

/-- Generic memory-safety invariant for this subset of fields. -/
def MemSafe (m : CVodeMem) : Prop :=
  0 ≤ m.cv_monitor_interval ∧
  0.0 ≤ m.cv_dgmax_lsetup

/-- Numerical stability envelope for gamma setup threshold. -/
def StableDGamma (x : Float) : Prop :=
  0.0 ≤ x ∧ x ≤ 1.0e6

/-! ## CVodeSetDeltaGammaMaxLSetup specification -/

/-- Functional model of `CVodeSetDeltaGammaMaxLSetup`. -/
def CVodeSetDeltaGammaMaxLSetup
    (cvode_mem : Option CVodeMem) (dgmax_lsetup : Float) :
    CVStatus × Option CVodeMem :=
  match cvode_mem with
  | none =>
      (CV_MEM_NULL, none)
  | some m =>
      let newVal := if dgmax_lsetup < 0.0 then DGMAX_LSETUP_DEFAULT else dgmax_lsetup
      (CV_SUCCESS, some { m with cv_dgmax_lsetup := newVal })

/-- Postcondition theorem: null pointer case. -/
theorem CVodeSetDeltaGammaMaxLSetup_null_post
    (dg : Float) :
    CVodeSetDeltaGammaMaxLSetup none dg = (CV_MEM_NULL, none) := by
  rfl

/-- Postcondition theorem: non-null case updates exactly one field by C semantics. -/
theorem CVodeSetDeltaGammaMaxLSetup_nonnull_post
    (m : CVodeMem) (dg : Float) :
    let (rc, out) := CVodeSetDeltaGammaMaxLSetup (some m) dg
    rc = CV_SUCCESS ∧
    out = some { m with cv_dgmax_lsetup := (if dg < 0.0 then DGMAX_LSETUP_DEFAULT else dg) } := by
  simp [CVodeSetDeltaGammaMaxLSetup]

/-- Memory safety preservation under standard preconditions. -/
theorem CVodeSetDeltaGammaMaxLSetup_preserves_memsafe
    (m : CVodeMem) (dg : Float)
    (hSafe : MemSafe m)
    (hDef : 0.0 ≤ DGMAX_LSETUP_DEFAULT)
    (hIn  : dg < 0.0 ∨ 0.0 ≤ dg) :
    let (_, out) := CVodeSetDeltaGammaMaxLSetup (some m) dg
    match out with
    | none => False
    | some m' => MemSafe m' := by
  rcases hSafe with ⟨hInt, hGamma⟩
  simp [CVodeSetDeltaGammaMaxLSetup, MemSafe]
  rcases hIn with hneg | hnonneg
  · simp [hneg, hInt, hDef]
  · have : ¬ dg < 0.0 := by exact not_lt.mpr hnonneg
    simp [this, hInt, hnonneg]

/-- Numerical stability bound: output threshold remains in stable range
    if default and nonnegative input are stable. -/
theorem CVodeSetDeltaGammaMaxLSetup_stability
    (m : CVodeMem) (dg : Float)
    (hDefStable : StableDGamma DGMAX_LSETUP_DEFAULT)
    (hInStable  : 0.0 ≤ dg → dg ≤ 1.0e6) :
    let (_, out) := CVodeSetDeltaGammaMaxLSetup (some m) dg
    match out with
    | none => False
    | some m' => StableDGamma m'.cv_dgmax_lsetup := by
  simp [CVodeSetDeltaGammaMaxLSetup, StableDGamma]
  by_cases h : dg < 0.0
  · simp [h, hDefStable]
  · have hnonneg : 0.0 ≤ dg := le_of_not_gt h
    have hub : dg ≤ 1.0e6 := hInStable hnonneg
    simp [h, hnonneg, hub]

/-! ## CVodeSetUserData specification -/

/-- Functional model of `CVodeSetUserData`. -/
def CVodeSetUserData
    (cvode_mem : Option CVodeMem) (user_data : Option UserData) :
    CVStatus × Option CVodeMem :=
  match cvode_mem with
  | none => (CV_MEM_NULL, none)
  | some m => (CV_SUCCESS, some { m with cv_user_data := user_data })

theorem CVodeSetUserData_null_post
    (u : Option UserData) :
    CVodeSetUserData none u = (CV_MEM_NULL, none) := by
  rfl

theorem CVodeSetUserData_nonnull_post
    (m : CVodeMem) (u : Option UserData) :
    let (rc, out) := CVodeSetUserData (some m) u
    rc = CV_SUCCESS ∧ out = some { m with cv_user_data := u } := by
  simp [CVodeSetUserData]

theorem CVodeSetUserData_preserves_memsafe
    (m : CVodeMem) (u : Option UserData)
    (hSafe : MemSafe m) :
    let (_, out) := CVodeSetUserData (some m) u
    match out with
    | none => False
    | some m' => MemSafe m' := by
  simpa [CVodeSetUserData, MemSafe] using hSafe

/-
  ## CVodeSetMonitorFn specification
  C behavior depends on compile-time macro SUNDIALS_ENABLE_MONITORING.
-/

/-- Functional model of `CVodeSetMonitorFn`. -/
def CVodeSetMonitorFn
    (monitoringEnabled : MonitoringEnabled)
    (cvode_mem : Option CVodeMem)
    (fn : CVMonitorFn) :
    CVStatus × Option CVodeMem :=
  match cvode_mem with
  | none => (CV_MEM_NULL, none)
  | some m =>
      if monitoringEnabled then
        (CV_SUCCESS, some { m with cv_monitorfun := some fn })
      else
        (CV_ILL_INPUT, some m)

theorem CVodeSetMonitorFn_null_post
    (en : MonitoringEnabled) (fn : CVMonitorFn) :
    CVodeSetMonitorFn en none fn = (CV_MEM_NULL, none) := by
  rfl

theorem CVodeSetMonitorFn_enabled_post
    (m : CVodeMem) (fn : CVMonitorFn) :
    let (rc, out) := CVodeSetMonitorFn true (some m) fn
    rc = CV_SUCCESS ∧ out = some { m with cv_monitorfun := some fn } := by
  simp [CVodeSetMonitorFn]

theorem CVodeSetMonitorFn_disabled_post
    (m : CVodeMem) (fn : CVMonitorFn) :
    let (rc, out) := CVodeSetMonitorFn false (some m) fn
    rc = CV_ILL_INPUT ∧ out = some m := by
  simp [CVodeSetMonitorFn]

theorem CVodeSetMonitorFn_preserves_memsafe
    (en : MonitoringEnabled) (m : CVodeMem) (fn : CVMonitorFn)
    (hSafe : MemSafe m) :
    let (_, out) := CVodeSetMonitorFn en (some m) fn
    match out with
    | none => False
    | some m' => MemSafe m' := by
  by_cases h : en
  · simp [CVodeSetMonitorFn, h, MemSafe] at *
    exact hSafe
  · simp [CVodeSetMonitorFn, h, MemSafe] at *
    exact hSafe

/-
  ## CVodeSetMonitorFrequency specification
  C precondition: nst >= 0, else CV_ILL_INPUT.
-/

/-- Functional model of `CVodeSetMonitorFrequency`. -/
def CVodeSetMonitorFrequency
    (monitoringEnabled : MonitoringEnabled)
    (cvode_mem : Option CVodeMem)
    (nst : Int) :
    CVStatus × Option CVodeMem :=
  match cvode_mem with
  | none => (CV_MEM_NULL, none)
  | some m =>
      if nst < 0 then
        (CV_ILL_INPUT, some m)
      else if monitoringEnabled then
        (CV_SUCCESS, some { m with cv_monitor_interval := nst })
      else
        (CV_ILL_INPUT, some m)

/-- Preconditions as hypotheses: valid memory and nonnegative step interval. -/
theorem CVodeSetMonitorFrequency_success_post
    (m : CVodeMem) (nst : Int)
    (hNst : 0 ≤ nst) :
    let (rc, out) := CVodeSetMonitorFrequency true (some m) nst
    rc = CV_SUCCESS ∧ out = some { m with cv_monitor_interval := nst } := by
  have hNotLt : ¬ nst < 0 := not_lt.mpr hNst
  simp [CVodeSetMonitorFrequency, hNotLt]

theorem CVodeSetMonitorFrequency_negative_input_post
    (en : MonitoringEnabled) (m : CVodeMem) (nst : Int)
    (hNeg : nst < 0) :
    let (rc, out) := CVodeSetMonitorFrequency en (some m) nst
    rc = CV_ILL_INPUT ∧ out = some m := by
  simp [CVodeSetMonitorFrequency, hNeg]

theorem CVodeSetMonitorFrequency_null_post
    (en : MonitoringEnabled) (nst : Int) :
    CVodeSetMonitorFrequency en none nst = (CV_MEM_NULL, none) := by
  rfl

theorem CVodeSetMonitorFrequency_preserves_memsafe
    (en : MonitoringEnabled) (m : CVodeMem) (nst : Int)
    (hSafe : MemSafe m)
    (hNst : 0 ≤ nst) :
    let (_, out) := CVodeSetMonitorFrequency en (some m) nst
    match out with
    | none => False
    | some m' => MemSafe m' := by
  rcases hSafe with ⟨hInt, hGamma⟩
  by_cases hEn : en
  · have hNotLt : ¬ nst < 0 := not_lt.mpr hNst
    simp [CVodeSetMonitorFrequency, hEn, hNotLt, MemSafe, hNst, hGamma]
  · have hNotLt : ¬ nst < 0 := not_lt.mpr hNst
    simp [CVodeSetMonitorFrequency, hEn, hNotLt, MemSafe, hInt, hGamma]

end SUNDIALS.CVODE.Spec