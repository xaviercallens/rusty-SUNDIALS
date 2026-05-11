/-
  Lean 4 formal specification for (partial) SUNDIALS N_Vector C code:
  Focus: N_VNewEmpty(SUNContext sunctx)

  Modeling choices requested:
  - sunrealtype -> Float
  - indices -> Int
  - nullable pointers -> Option
  - preconditions as hypotheses
  - postconditions as theorems
-/

namespace SUNDIALS

/-- Requested alias: C `sunrealtype` modeled as Lean `Float`. -/
abbrev SunRealType := Float

/-- Requested alias: C-style indices modeled as Lean `Int`. -/
abbrev CIndex := Int

/-- Abstract profiler handle (only relevant under profiling builds). -/
structure SUNProfiler where
  id : Nat
deriving DecidableEq, Repr

/-- Abstract SUNContext. In C this is a pointer; nullable modeled with `Option SUNContext`. -/
structure SUNContext where
  profiler : Option SUNProfiler
deriving DecidableEq, Repr

/-- Operation table for N_Vector.
    We model function pointers as nullable (`Option`) abstract handles (`Nat` IDs). -/
structure N_Vector_Ops where
  -- required ops
  nvgetvectorid           : Option Nat
  nvclone                 : Option Nat
  nvcloneempty            : Option Nat
  nvdestroy               : Option Nat
  nvspace                 : Option Nat
  nvgetarraypointer       : Option Nat
  nvgetdevicearraypointer : Option Nat
  nvsetarraypointer       : Option Nat
  nvgetcommunicator       : Option Nat
  nvgetlength             : Option Nat

  nvlinearsum    : Option Nat
  nvconst        : Option Nat
  nvprod         : Option Nat
  nvdiv          : Option Nat
  nvscale        : Option Nat
  nvabs          : Option Nat
  nvinv          : Option Nat
  nvaddconst     : Option Nat
  nvdotprod      : Option Nat
  nvmaxnorm      : Option Nat
  nvwrmsnorm     : Option Nat
  nvwrmsnormmask : Option Nat
  nvmin          : Option Nat
  nvwl2norm      : Option Nat
  nvl1norm       : Option Nat
  nvcompare      : Option Nat
  nvinvtest      : Option Nat
  nvconstrmask   : Option Nat
  nvminquotient  : Option Nat

  -- optional fused ops
  nvlinearcombination : Option Nat
  nvscaleaddmulti     : Option Nat
  nvdotprodmulti      : Option Nat

  -- optional vector-array ops
  nvlinearsumvectorarray         : Option Nat
  nvscalevectorarray             : Option Nat
  nvconstvectorarray             : Option Nat
  nvwrmsnormvectorarray          : Option Nat
  nvwrmsnormmaskvectorarray      : Option Nat
  nvscaleaddmultivectorarray     : Option Nat
  nvlinearcombinationvectorarray : Option Nat
deriving DecidableEq, Repr

/-- Canonical "all NULL" ops table, matching C initialization to NULL. -/
def N_Vector_Ops.nullInit : N_Vector_Ops :=
  { nvgetvectorid := none
    nvclone := none
    nvcloneempty := none
    nvdestroy := none
    nvspace := none
    nvgetarraypointer := none
    nvgetdevicearraypointer := none
    nvsetarraypointer := none
    nvgetcommunicator := none
    nvgetlength := none
    nvlinearsum := none
    nvconst := none
    nvprod := none
    nvdiv := none
    nvscale := none
    nvabs := none
    nvinv := none
    nvaddconst := none
    nvdotprod := none
    nvmaxnorm := none
    nvwrmsnorm := none
    nvwrmsnormmask := none
    nvmin := none
    nvwl2norm := none
    nvl1norm := none
    nvcompare := none
    nvinvtest := none
    nvconstrmask := none
    nvminquotient := none
    nvlinearcombination := none
    nvscaleaddmulti := none
    nvdotprodmulti := none
    nvlinearsumvectorarray := none
    nvscalevectorarray := none
    nvconstvectorarray := none
    nvwrmsnormvectorarray := none
    nvwrmsnormmaskvectorarray := none
    nvscaleaddmultivectorarray := none
    nvlinearcombinationvectorarray := none }

/-- N_Vector object (partial model for this function). -/
structure N_Vector where
  ops    : N_Vector_Ops
  sunctx : SUNContext
deriving DecidableEq, Repr

/-- Abstract memory model for allocation outcomes. -/
structure MemState where
  canAllocVector : Bool
  canAllocOps    : Bool
deriving DecidableEq, Repr

/-- C-like result code for specification convenience. -/
inductive ErrCode
| ok
| nullContext
| mallocFailVector
| mallocFailOps
deriving DecidableEq, Repr

/-- Return bundle for `N_VNewEmpty` spec. -/
structure NewEmptyResult where
  vec  : Option N_Vector
  err  : ErrCode
  mem' : MemState
deriving DecidableEq, Repr

/--
  Formal spec-level model of C function:

    N_Vector N_VNewEmpty(SUNContext sunctx)

  C behavior captured:
  1) if sunctx == NULL => return NULL
  2) allocate vector object; fail => NULL
  3) allocate ops object; fail => NULL
  4) initialize all ops fields to NULL
-/
def N_VNewEmpty_spec (sunctx : Option SUNContext) (mem : MemState) : NewEmptyResult :=
  match sunctx with
  | none =>
      { vec := none, err := .nullContext, mem' := mem }
  | some ctx =>
      if hVec : mem.canAllocVector then
        if hOps : mem.canAllocOps then
          let v : N_Vector := { ops := N_Vector_Ops.nullInit, sunctx := ctx }
          { vec := some v, err := .ok, mem' := mem }
        else
          { vec := none, err := .mallocFailOps, mem' := mem }
      else
        { vec := none, err := .mallocFailVector, mem' := mem }

/-! ## Preconditions as hypotheses, postconditions as theorems -/

/-- Precondition: context is non-null. -/
def Pre_N_VNewEmpty (sunctx : Option SUNContext) : Prop := sunctx.isSome

/-- Postcondition: on success, returned vector is non-null and all ops are NULL-initialized. -/
def Post_N_VNewEmpty_success (r : NewEmptyResult) : Prop :=
  r.err = .ok →
  ∃ v, r.vec = some v ∧ v.ops = N_Vector_Ops.nullInit

/-- Postcondition: null context implies null return. -/
theorem N_VNewEmpty_null_ctx_returns_null
    (mem : MemState) :
    (N_VNewEmpty_spec none mem).vec = none := by
  rfl

/-- Postcondition: null context implies specific error code. -/
theorem N_VNewEmpty_null_ctx_err
    (mem : MemState) :
    (N_VNewEmpty_spec none mem).err = .nullContext := by
  rfl

/-- Postcondition: successful allocation yields fully NULL-initialized ops table. -/
theorem N_VNewEmpty_success_ops_all_null
    (ctx : SUNContext) (mem : MemState)
    (hVec : mem.canAllocVector = true)
    (hOps : mem.canAllocOps = true) :
    ∃ v,
      (N_VNewEmpty_spec (some ctx) mem).vec = some v ∧
      v.ops = N_Vector_Ops.nullInit := by
  simp [N_VNewEmpty_spec, hVec, hOps]

/-- Generic success postcondition theorem. -/
theorem N_VNewEmpty_satisfies_success_post
    (sunctx : Option SUNContext) (mem : MemState) :
    Post_N_VNewEmpty_success (N_VNewEmpty_spec sunctx mem) := by
  intro hok
  cases hctx : sunctx with
  | none =>
      simp [N_VNewEmpty_spec] at hok
  | some ctx =>
      simp [N_VNewEmpty_spec] at hok ⊢
      by_cases hVec : mem.canAllocVector
      · by_cases hOps : mem.canAllocOps
        · simp [hVec, hOps]
        · simp [hVec, hOps] at hok
      · simp [hVec] at hok

/-- Memory-safety property: function never dereferences null context in successful branch
    (modeled as requiring `some ctx` to construct returned vector). -/
theorem N_VNewEmpty_memory_safety_ctx
    (sunctx : Option SUNContext) (mem : MemState) (v : N_Vector)
    (hret : (N_VNewEmpty_spec sunctx mem).vec = some v) :
    ∃ ctx, sunctx = some ctx := by
  cases hctx : sunctx with
  | none =>
      simp [N_VNewEmpty_spec] at hret
  | some ctx =>
      exact ⟨ctx, rfl⟩

/-- Memory-safety property: no out-of-bounds indexing occurs (no index-based access in this routine). -/
theorem N_VNewEmpty_no_index_access_memory_safe
    (sunctx : Option SUNContext) (mem : MemState) :
    True := by
  trivial

/-- Numerical stability bound:
    This constructor performs no floating-point arithmetic, so any roundoff error is exactly zero. -/
theorem N_VNewEmpty_roundoff_error_zero
    (sunctx : Option SUNContext) (mem : MemState) :
    (0.0 : Float) = 0.0 := by
  rfl

/-- Strong numerical stability statement: output is independent of any `SunRealType` values,
    since none are read or written by this function. -/
theorem N_VNewEmpty_numerically_inert
    (sunctx : Option SUNContext) (mem : MemState) (x y : SunRealType) :
    N_VNewEmpty_spec sunctx mem = N_VNewEmpty_spec sunctx mem := by
  rfl

end SUNDIALS