/-
Lean 4 specification/proof sketch for equivalence between:

C:    N_VNewEmpty(SUNContext sunctx)
Rust: safe constructor for generic NVector scaffolding with Option/Result

Modeling choices requested:
- sunrealtype := Float
- indices     := Int
- nullable pointers := Option
- preconditions as hypotheses
- postconditions as theorems

This is a *semantic model* (deep embedding), not direct extraction from C/Rust.
-/

namespace SundialsEq

abbrev SunReal    := Float
abbrev SunIndex   := Int

/-- Abstract profiler handle. -/
structure SunProfiler where
  dummy : Unit := ()

/-- SUNContext with optional profiler (nullable modeled by Option). -/
structure SunContext where
  profiler : Option SunProfiler

/-- C-side operation table: all fields nullable function pointers.
    We model only the prefix shown in the snippet. -/
structure NVectorOpsC where
  nvgetvectorid           : Option Unit := none
  nvclone                 : Option Unit := none
  nvcloneempty            : Option Unit := none
  nvdestroy               : Option Unit := none
  nvspace                 : Option Unit := none
  nvgetarraypointer       : Option Unit := none
  nvgetdevicearraypointer : Option Unit := none
  nvsetarraypointer       : Option Unit := none
  nvgetcommunicator       : Option Unit := none
  nvgetlength             : Option Unit := none
  nvlinearsum             : Option Unit := none
  nvconst                 : Option Unit := none
  nvprod                  : Option Unit := none
  nvdiv                   : Option Unit := none
  nvscale                 : Option Unit := none
  nvabs                   : Option Unit := none
  nvinv                   : Option Unit := none
  nvaddconst              : Option Unit := none

/-- C-side N_Vector object (reduced model). -/
structure NVectorC where
  sunctx : SunContext
  ops    : NVectorOpsC

/-- Rust-side error type (subset relevant to constructor). -/
inductive CvodeError
| MemFail
| ArgCorrupt (msg : String)
deriving Repr, DecidableEq

/-- Rust-side NVector object (reduced model). -/
structure NVectorR where
  sunctx : SunContext
  ops    : NVectorOpsC

/-- C allocator outcome model. -/
structure AllocModel where
  allocVecOk : Bool
  allocOpsOk : Bool

/-- C semantics of N_VNewEmpty:
    - if sunctx == NULL => NULL
    - else allocate vector and ops; on any malloc failure => NULL
    - initialize ops fields to NULL
-/
def c_N_VNewEmpty (sunctx : Option SunContext) (a : AllocModel) : Option NVectorC :=
  match sunctx with
  | none => none
  | some ctx =>
      if h1 : a.allocVecOk = true then
        if h2 : a.allocOpsOk = true then
          some { sunctx := ctx, ops := {} }
        else
          none
      else
        none

/-- Rust constructor semantics:
    - None context => Err ArgCorrupt
    - allocation failure => Err MemFail
    - success => Ok with default ops = None for all entries
-/
def rust_new_empty (sunctx : Option SunContext) (a : AllocModel) :
    Except CvodeError NVectorR :=
  match sunctx with
  | none => .error (.ArgCorrupt "sunctx is null")
  | some ctx =>
      if a.allocVecOk = true ∧ a.allocOpsOk = true then
        .ok { sunctx := ctx, ops := {} }
      else
        .error .MemFail

/-- Observational relation between C nullable return and Rust Result. -/
def ObsEq (c : Option NVectorC) (r : Except CvodeError NVectorR) : Prop :=
  match c, r with
  | none, .error _ => True
  | some vc, .ok vr => vc.sunctx = vr.sunctx ∧ vc.ops = vr.ops
  | _, _ => False

/-- Safety predicate: constructed object has fully-null ops table (as in C init). -/
def OpsAllNull (ops : NVectorOpsC) : Prop :=
  ops = {}

/-- No-UB/memory-safety abstraction:
    constructor either fails cleanly or returns fully initialized object. -/
def SafeCtorC (out : Option NVectorC) : Prop :=
  match out with
  | none => True
  | some v => OpsAllNull v.ops

def SafeCtorR (out : Except CvodeError NVectorR) : Prop :=
  match out with
  | .error _ => True
  | .ok v => OpsAllNull v.ops

/-- Preconditions as hypotheses:
    `halloc` ties Rust and C allocation outcomes (same environment). -/
theorem c_rust_equiv_new_empty
  (sunctx : Option SunContext) (a : AllocModel)
  (halloc : True := by trivial) :
  ObsEq (c_N_VNewEmpty sunctx a) (rust_new_empty sunctx a) := by
  cases sunctx with
  | none =>
      simp [c_N_VNewEmpty, rust_new_empty, ObsEq]
  | some ctx =>
      by_cases hv : a.allocVecOk = true ∧ a.allocOpsOk = true
      · have hv1 : a.allocVecOk = true := hv.left
        have hv2 : a.allocOpsOk = true := hv.right
        simp [c_N_VNewEmpty, rust_new_empty, ObsEq, hv, hv1, hv2]
      · have hnot : ¬ (a.allocVecOk = true ∧ a.allocOpsOk = true) := hv
        by_cases hv1 : a.allocVecOk = true
        · by_cases hv2 : a.allocOpsOk = true
          · exfalso; exact hnot ⟨hv1, hv2⟩
          · simp [c_N_VNewEmpty, rust_new_empty, ObsEq, hv1, hv2, hnot]
        · simp [c_N_VNewEmpty, rust_new_empty, ObsEq, hv1, hnot]

/-- Postcondition theorem: C constructor is memory-safe in this model. -/
theorem c_new_empty_safe (sunctx : Option SunContext) (a : AllocModel) :
  SafeCtorC (c_N_VNewEmpty sunctx a) := by
  cases sunctx <;> simp [c_N_VNewEmpty, SafeCtorC, OpsAllNull]
  split <;> simp [SafeCtorC, OpsAllNull]
  split <;> simp [SafeCtorC, OpsAllNull]

/-- Postcondition theorem: Rust constructor is memory-safe in this model. -/
theorem rust_new_empty_safe (sunctx : Option SunContext) (a : AllocModel) :
  SafeCtorR (rust_new_empty sunctx a) := by
  cases sunctx <;> simp [rust_new_empty, SafeCtorR, OpsAllNull]
  split <;> simp [SafeCtorR, OpsAllNull]

/-- Strong behavioral equivalence + safety corollary. -/
theorem c_rust_equiv_and_safe
  (sunctx : Option SunContext) (a : AllocModel) :
  ObsEq (c_N_VNewEmpty sunctx a) (rust_new_empty sunctx a)
  ∧ SafeCtorC (c_N_VNewEmpty sunctx a)
  ∧ SafeCtorR (rust_new_empty sunctx a) := by
  exact ⟨c_rust_equiv_new_empty sunctx a, c_new_empty_safe sunctx a, rust_new_empty_safe sunctx a⟩

end SundialsEq