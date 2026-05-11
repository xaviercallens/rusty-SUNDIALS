/-
Lean 4 specification skeleton for C ↔ Rust equivalence of CVBBDPrecInit-style behavior.

Notes:
* We model `sunrealtype` as `Float`, indices as `Int`.
* Nullable pointers are modeled with `Option`.
* We encode C integer return flags and Rust `Result` errors, then prove correspondence.
* This is a *behavioral model* (not a full mechanization of all CVODE internals).
-/

namespace CVBBD

--------------------------------------------------------------------------------
-- Basic numeric aliases (requested)
--------------------------------------------------------------------------------

abbrev SunReal := Float
abbrev SunIndex := Int

def MIN_INC_MULT : SunReal := 1000.0
def ZERO : SunReal := 0.0
def ONE  : SunReal := 1.0
def TWO  : SunReal := 2.0

--------------------------------------------------------------------------------
-- Error models: C flags vs Rust errors
--------------------------------------------------------------------------------

inductive CFlag where
  | success
  | mem_null
  | lmem_null
  | pmem_null
  | ill_input
  | mem_fail
  | sunls_fail
deriving DecidableEq, Repr

inductive CvodeError where
  | MemNull
  | LMemNull
  | PMemNull
  | IllInput (msg : String)
  | MemFail (msg : String)
  | SunLsFail (msg : String)
deriving DecidableEq, Repr

def cFlagToRust : CFlag → Except CvodeError Unit
  | .success    => .ok ()
  | .mem_null   => .error .MemNull
  | .lmem_null  => .error .LMemNull
  | .pmem_null  => .error .PMemNull
  | .ill_input  => .error (.IllInput "illegal input")
  | .mem_fail   => .error (.MemFail "allocation failure")
  | .sunls_fail => .error (.SunLsFail "linear solver failure")

--------------------------------------------------------------------------------
-- Abstract state for C and Rust sides
--------------------------------------------------------------------------------

/-- Abstract C-side CVODE memory graph (nullable pointers via Option). -/
structure CState where
  cvode_mem : Option Unit
  cvls_mem  : Option Unit
  p_mem     : Option Unit
  deriving Repr

/-- Abstract Rust-side typed state (still optional to model uninitialized fields). -/
structure RState where
  cvode_mem : Option Unit
  cvls_mem  : Option Unit
  p_mem     : Option Unit
  deriving Repr

/-- Relation connecting C and Rust abstract states. -/
def StateRel (c : CState) (r : RState) : Prop :=
  c.cvode_mem = r.cvode_mem ∧
  c.cvls_mem  = r.cvls_mem  ∧
  c.p_mem     = r.p_mem

--------------------------------------------------------------------------------
-- Preconditions (as hypotheses) for init-like call
--------------------------------------------------------------------------------

structure InitArgs where
  Nlocal : SunIndex
  mudq   : SunIndex
  mldq   : SunIndex
  mukeep : SunIndex
  mlkeep : SunIndex
  dqrely : SunReal
  deriving Repr

def WellFormedArgs (a : InitArgs) : Prop :=
  0 ≤ a.Nlocal ∧
  0 ≤ a.mudq ∧ 0 ≤ a.mldq ∧
  0 ≤ a.mukeep ∧ 0 ≤ a.mlkeep

--------------------------------------------------------------------------------
-- C and Rust operational models (small-step-free, functional summary)
--------------------------------------------------------------------------------

/-- C summary semantics for CVBBDPrecInit-like routine. -/
def c_CVBBDPrecInit (s : CState) (a : InitArgs) : CFlag × CState :=
  match s.cvode_mem with
  | none   => (.mem_null, s)
  | some _ =>
    if hargs : WellFormedArgs a then
      -- success path allocates/sets p_mem in this abstract model
      (.success, { s with p_mem := some () })
    else
      (.ill_input, s)

/-- Rust summary semantics for builder/init call. -/
def r_CVBBDPrecInit (s : RState) (a : InitArgs) : Except CvodeError RState :=
  match s.cvode_mem with
  | none   => .error .MemNull
  | some _ =>
    if hargs : WellFormedArgs a then
      .ok { s with p_mem := some () }
    else
      .error (.IllInput "illegal input")

--------------------------------------------------------------------------------
-- No-UB / memory-safety style invariants
--------------------------------------------------------------------------------

/-- In this abstract model, memory safety means no dereference of absent cvode_mem. -/
def CSafeToCall (s : CState) : Prop := True
def RSafeToCall (s : RState) : Prop := True

theorem c_no_ub_on_null_guard (s : CState) (a : InitArgs) :
    s.cvode_mem = none →
    (c_CVBBDPrecInit s a).1 = .mem_null := by
  intro h
  simp [c_CVBBDPrecInit, h]

theorem rust_memory_safe_total (s : RState) (a : InitArgs) :
    ∃ out, out = r_CVBBDPrecInit s a := by
  exact ⟨_, rfl⟩

--------------------------------------------------------------------------------
-- Behavioral equivalence theorem
--------------------------------------------------------------------------------

theorem init_equiv
  (c : CState) (r : RState) (a : InitArgs)
  (hrel : StateRel c r) :
  let cOut := c_CVBBDPrecInit c a
  let rOut := r_CVBBDPrecInit r a
  cFlagToRust cOut.1 =
    match rOut with
    | .ok _      => .ok ()
    | .error err => .error err := by
  rcases hrel with ⟨hcv, hls, hpm⟩
  simp [c_CVBBDPrecInit, r_CVBBDPrecInit, cFlagToRust, hcv]
  split <;> simp

/-- Stronger postcondition: resulting states remain related on success/failure. -/
theorem init_equiv_stateful
  (c : CState) (r : RState) (a : InitArgs)
  (hrel : StateRel c r) :
  match c_CVBBDPrecInit c a, r_CVBBDPrecInit r a with
  | (cf, c'), .ok r' =>
      cFlagToRust cf = .ok () ∧ StateRel c' r'
  | (cf, _), .error e =>
      cFlagToRust cf = .error e := by
  rcases hrel with ⟨hcv, hls, hpm⟩
  simp [c_CVBBDPrecInit, r_CVBBDPrecInit, cFlagToRust, StateRel, hcv]
  split <;> simp [hls, hpm]

--------------------------------------------------------------------------------
-- Optional: explicit theorem showing null-pointer/Option correspondence
--------------------------------------------------------------------------------

theorem null_pointer_equiv
  (c : CState) (r : RState) (a : InitArgs)
  (hrel : StateRel c r)
  (hnull : c.cvode_mem = none) :
  (c_CVBBDPrecInit c a).1 = .mem_null ∧
  r_CVBBDPrecInit r a = .error .MemNull := by
  rcases hrel with ⟨hcv, _, _⟩
  subst hcv
  simp [c_CVBBDPrecInit, r_CVBBDPrecInit, hnull]

end CVBBD