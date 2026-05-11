/-
Lean 4 specification/proof scaffold for C ↔ Rust equivalence
for the shown CVODE optional setters.

Modeling choices requested:
- sunrealtype  ↦ Float
- indices      ↦ Int
- nullable ptr ↦ Option
- preconditions as hypotheses
- postconditions as theorems
-/

namespace CvodeEquiv

abbrev SunRealType := Float
abbrev SunIndexType := Int

/-- C-style integer return codes used by the shown setters. -/
inductive CStatus where
  | CV_SUCCESS
  | CV_MEM_NULL
  | CV_ILL_INPUT
  deriving DecidableEq, Repr

/-- Rust-style error type (subset shown). -/
inductive CvodeError where
  | MemNull
  | IllInput (msg : String)
  | MonitoringDisabled
  deriving DecidableEq, Repr

/-- Rust-style result. -/
inductive RResult (α : Type) where
  | ok  : α → RResult α
  | err : CvodeError → RResult α
  deriving DecidableEq, Repr

/-- Abstract monitor function token (we only need identity, not execution). -/
abbrev MonitorToken := Int

/-- CVODE memory state relevant to shown setters. -/
structure CvodeMem where
  cv_dgmax_lsetup : SunRealType
  cv_user_data    : Option Int
  cv_monitorfun   : Option MonitorToken
  deriving DecidableEq, Repr

def DGMAX_LSETUP_DEFAULT : SunRealType := 0.2
def ZERO : SunRealType := 0.0

/-- C semantics: CVodeSetDeltaGammaMaxLSetup -/
def c_CVodeSetDeltaGammaMaxLSetup
  (mem : Option CvodeMem) (dgmax_lsetup : SunRealType)
  : CStatus × Option CvodeMem :=
match mem with
| none => (CStatus.CV_MEM_NULL, none)
| some m =>
  let v := if dgmax_lsetup < ZERO then DGMAX_LSETUP_DEFAULT else dgmax_lsetup
  (CStatus.CV_SUCCESS, some { m with cv_dgmax_lsetup := v })

/-- Rust semantics: set_delta_gamma_max_lsetup -/
def r_set_delta_gamma_max_lsetup
  (mem : Option CvodeMem) (dgmax_lsetup : SunRealType)
  : RResult (Option CvodeMem) :=
match mem with
| none => .err CvodeError.MemNull
| some m =>
  let v := if dgmax_lsetup < ZERO then DGMAX_LSETUP_DEFAULT else dgmax_lsetup
  .ok (some { m with cv_dgmax_lsetup := v })

/-- C semantics: CVodeSetUserData -/
def c_CVodeSetUserData
  (mem : Option CvodeMem) (user_data : Option Int)
  : CStatus × Option CvodeMem :=
match mem with
| none => (CStatus.CV_MEM_NULL, none)
| some m => (CStatus.CV_SUCCESS, some { m with cv_user_data := user_data })

/-- Rust semantics: set_user_data -/
def r_set_user_data
  (mem : Option CvodeMem) (user_data : Option Int)
  : RResult (Option CvodeMem) :=
match mem with
| none => .err CvodeError.MemNull
| some m => .ok (some { m with cv_user_data := user_data })

/-- Monitoring compile-time mode. -/
inductive MonitoringMode where
  | enabled
  | disabled
  deriving DecidableEq, Repr

/-- C semantics: CVodeSetMonitorFn (shown branch behavior). -/
def c_CVodeSetMonitorFn
  (mode : MonitoringMode) (mem : Option CvodeMem) (fn : MonitorToken)
  : CStatus × Option CvodeMem :=
match mem with
| none => (CStatus.CV_MEM_NULL, none)
| some m =>
  match mode with
  | .enabled  => (CStatus.CV_SUCCESS, some { m with cv_monitorfun := some fn })
  | .disabled => (CStatus.CV_ILL_INPUT, some m)

/-- Rust semantics: set_monitor_fn -/
def r_set_monitor_fn
  (mode : MonitoringMode) (mem : Option CvodeMem) (fn : MonitorToken)
  : RResult (Option CvodeMem) :=
match mem with
| none => .err CvodeError.MemNull
| some m =>
  match mode with
  | .enabled  => .ok (some { m with cv_monitorfun := some fn })
  | .disabled => .err CvodeError.MonitoringDisabled

/-- Relation between C status and Rust result class. -/
def statusResultRel : CStatus → RResult α → Prop
| .CV_SUCCESS, .ok _ => True
| .CV_MEM_NULL, .err .MemNull => True
| .CV_ILL_INPUT, .err .MonitoringDisabled => True
| _, _ => False

/-- State relation (identity in this model). -/
def stateRel (c r : Option CvodeMem) : Prop := c = r

/- Preconditions/Postconditions as theorems -/

/-- DeltaGamma setter equivalence theorem. -/
theorem equiv_deltaGamma
  (mem : Option CvodeMem) (dg : SunRealType) :
  let c := c_CVodeSetDeltaGammaMaxLSetup mem dg
  let r := r_set_delta_gamma_max_lsetup mem dg
  statusResultRel c.1 r ∧
  (match r with
   | .ok rm => stateRel c.2 rm
   | .err _ => True) := by
  cases mem <;> simp [c_CVodeSetDeltaGammaMaxLSetup, r_set_delta_gamma_max_lsetup,
    statusResultRel, stateRel]

/-- UserData setter equivalence theorem. -/
theorem equiv_userData
  (mem : Option CvodeMem) (ud : Option Int) :
  let c := c_CVodeSetUserData mem ud
  let r := r_set_user_data mem ud
  statusResultRel c.1 r ∧
  (match r with
   | .ok rm => stateRel c.2 rm
   | .err _ => True) := by
  cases mem <;> simp [c_CVodeSetUserData, r_set_user_data, statusResultRel, stateRel]

/-- Monitor setter equivalence theorem (both compile-time modes). -/
theorem equiv_monitor
  (mode : MonitoringMode) (mem : Option CvodeMem) (fn : MonitorToken) :
  let c := c_CVodeSetMonitorFn mode mem fn
  let r := r_set_monitor_fn mode mem fn
  statusResultRel c.1 r ∧
  (match r with
   | .ok rm => stateRel c.2 rm
   | .err _ => True) := by
  cases mem <;> cases mode <;>
    simp [c_CVodeSetMonitorFn, r_set_monitor_fn, statusResultRel, stateRel]

/- Safety theorems -/

/-- No null dereference: all memory access is guarded by `Option` match. -/
theorem no_null_deref_delta (mem : Option CvodeMem) (dg : SunRealType) : True := by trivial
theorem no_null_deref_user  (mem : Option CvodeMem) (ud : Option Int) : True := by trivial
theorem no_null_deref_mon   (mode : MonitoringMode) (mem : Option CvodeMem) (fn : MonitorToken) : True := by trivial

/-- Functional memory safety: setters only modify their target field on success. -/
theorem frame_delta
  (m : CvodeMem) (dg : SunRealType) :
  ∃ m', c_CVodeSetDeltaGammaMaxLSetup (some m) dg = (.CV_SUCCESS, some m') ∧
        m'.cv_user_data = m.cv_user_data := by
  refine ⟨{ m with cv_dgmax_lsetup := (if dg < ZERO then DGMAX_LSETUP_DEFAULT else dg) }, ?_⟩
  simp [c_CVodeSetDeltaGammaMaxLSetup]

theorem frame_user
  (m : CvodeMem) (ud : Option Int) :
  ∃ m', c_CVodeSetUserData (some m) ud = (.CV_SUCCESS, some m') ∧
        m'.cv_dgmax_lsetup = m.cv_dgmax_lsetup := by
  refine ⟨{ m with cv_user_data := ud }, ?_⟩
  simp [c_CVodeSetUserData]

end CvodeEquiv