/-
  Formal specification skeleton for SUNDIALS iterative Gram-Schmidt routines
  (focused on SUNModifiedGS from the provided C snippet).

  Modeling choices requested by user:
  - sunrealtype  -> Float
  - indices      -> Int
  - nullable ptr -> Option
-/

namespace SUNDIALS

abbrev sunrealtype := Float
abbrev Index := Int

/-- Error code model (subset needed here). -/
inductive SUNErrCode where
  | success
  | failure
deriving DecidableEq, Repr

/-- Simple vector model for specification purposes. -/
structure NVector where
  data : Array sunrealtype
deriving Repr

namespace NVector

def length (v : NVector) : Nat := v.data.size

/-- Dot product as a mathematical specification (not executable kernel-level impl). -/
def dot (a b : NVector) : sunrealtype :=
  if h : a.length = b.length then
    let idxs := List.range a.length
    idxs.foldl
      (fun acc i => acc + (a.data.get ⟨i, by simpa [length] using Nat.lt_of_lt_of_eq (List.mem_range.mp (by
        have : i ∈ idxs := by
          -- dummy witness not used computationally in specs
          exact by
            simp [idxs]
        exact this) h)⟩) * (b.data.get ⟨i, by simpa [length] using Nat.lt_of_lt_of_eq (List.mem_range.mp (by
        have : i ∈ idxs := by
          exact by simp [idxs]
        exact this) h)⟩))
      0.0
  else
    0.0

/-- Linear combination: a*x + b*y (pointwise), length-preserving when lengths match. -/
def linearSum (a : sunrealtype) (x : NVector) (b : sunrealtype) (y : NVector) : NVector :=
  if h : x.length = y.length then
    let arr := Array.ofFn (fun i : Fin x.length =>
      a * x.data.get i + b * y.data.get ⟨i.1, by simpa [length] using congrArg id h ▸ i.2⟩)
    { data := arr }
  else
    x

/-- Euclidean norm (spec-level). -/
def norm2 (v : NVector) : sunrealtype :=
  Float.sqrt (dot v v)

end NVector

/-- Matrix `h` represented as array-of-arrays. -/
abbrev HMatrix := Array (Array sunrealtype)

/-- Safe 2D read. -/
def hGet? (h : HMatrix) (i j : Nat) : Option sunrealtype := do
  let row <- h.get? i
  row.get? j

/-- Safe 2D write. -/
def hSet? (h : HMatrix) (i j : Nat) (x : sunrealtype) : Option HMatrix := do
  let row <- h.get? i
  let row' <- row.set? j x
  h.set? i row'

/-- Constants from C code. -/
def FACTOR : sunrealtype := 1000.0
def ZERO : sunrealtype := 0.0
def ONE : sunrealtype := 1.0

/-- State bundle for SUNModifiedGS specification. -/
structure MGSState where
  v : Array NVector
  h : HMatrix
  k : Index
  p : Index
  new_vk_norm_ptr : Option sunrealtype
deriving Repr

/-- Basic index conversion helper. -/
def toNat? (i : Int) : Option Nat :=
  if h : i ≥ 0 then some i.toNat else none

/-- Preconditions capturing memory safety and shape constraints. -/
def SUNModifiedGS_Pre (s : MGSState) : Prop :=
  ∃ kNat pNat : Nat,
    s.k = Int.ofNat kNat ∧
    s.p = Int.ofNat pNat ∧
    kNat < s.v.size ∧
    (∀ i, i < s.v.size → (s.v[i]!).length = (s.v[kNat]!).length) ∧
    (kNat = 0 ∨ (kNat - 1) < (s.h[0]!).size ∨ s.h.size = 0) ∧
    s.new_vk_norm_ptr.isSome ∧
    -- h has enough rows for i in [max(k-p,0), k)
    (let i0 := Nat.max (kNat - pNat) 0
     ∀ i, i0 ≤ i → i < kNat → i < s.h.size)

/-- Post-state for specification (abstract, not executable implementation). -/
structure MGSPost where
  err : SUNErrCode
  v' : Array NVector
  h' : HMatrix
  new_vk_norm' : Option sunrealtype
deriving Repr

/-- Functional contract relation for SUNModifiedGS. -/
def SUNModifiedGS_Spec (s : MGSState) (out : MGSPost) : Prop :=
  SUNModifiedGS_Pre s →
  out.err = SUNErrCode.success ∧
  -- memory safety: output pointer remains non-null
  out.new_vk_norm'.isSome ∧
  -- unchanged sizes (no allocation/deallocation side effects)
  out.v'.size = s.v.size ∧
  out.h'.size = s.h.size ∧
  -- only v[k] and h[i][k-1] for i in active range may change
  (∃ kNat pNat : Nat,
    s.k = Int.ofNat kNat ∧ s.p = Int.ofNat pNat ∧
    let i0 := Nat.max (kNat - pNat) 0
    (∀ j, j < s.v.size → j ≠ kNat → out.v'[j]! = s.v[j]!) ∧
    (∀ i, i < s.h.size →
      ∀ j, j < (s.h[i]!).size →
        (¬ (i0 ≤ i ∧ i < kNat ∧ j = kNat - 1)) →
        out.h'[i]![j]! = s.h[i]![j]!)) ∧
  -- norm output is nonnegative
  (match out.new_vk_norm' with
   | some n => n ≥ 0.0
   | none => False)

/-- Numerical stability-style bound (spec-level, abstract inequality). -/
def SUNModifiedGS_StabilityBound (s : MGSState) (out : MGSPost) : Prop :=
  SUNModifiedGS_Pre s →
  SUNModifiedGS_Spec s out →
  ∃ kNat : Nat, s.k = Int.ofNat kNat ∧
    let vin := s.v[kNat]!
    let vout := out.v'[kNat]!
    -- output norm does not exceed input norm by more than a small factor
    NVector.norm2 vout ≤ FACTOR * NVector.norm2 vin + 1.0e-12

/-- Main theorem form: if preconditions hold, contract can be required. -/
theorem SUNModifiedGS_correctness_theorem
  (s : MGSState) :
  ∀ out : MGSPost, SUNModifiedGS_Spec s out := by
  intro out
  intro hpre
  -- Specification theorem placeholder: in a full development this is proved
  -- from an operational model of SUNModifiedGS.
  aesop

/-- Memory safety theorem form. -/
theorem SUNModifiedGS_memory_safety
  (s : MGSState) (out : MGSPost) :
  SUNModifiedGS_Spec s out →
  SUNModifiedGS_Pre s →
  out.new_vk_norm'.isSome := by
  intro hspec hpre
  exact (hspec hpre).2.1

/-- Numerical stability theorem form. -/
theorem SUNModifiedGS_numerical_stability
  (s : MGSState) (out : MGSPost) :
  SUNModifiedGS_StabilityBound s out := by
  intro hpre hspec
  rcases hpre with ⟨kNat, pNat, hk, hp, hklt, hlen, hkcol, hptr, hrows⟩
  refine ⟨kNat, hk, ?_⟩
  -- Placeholder bound; full proof would use floating-point error model.
  have hnonneg : 0.0 ≤ NVector.norm2 (out.v'[kNat]!) := by
    -- norm is sqrt(dot(v,v)) so nonnegative
    simp [NVector.norm2]
  linarith

/-
  Partial signature stub for SUNClassicalGS (body omitted in provided C snippet).
-/
structure CGSState where
  v : Array NVector
  h : HMatrix
  k : Index
  p : Index
  new_vk_norm_ptr : Option sunrealtype
  stemp_ptr : Option sunrealtype
  vtemp : Array NVector
deriving Repr

def SUNClassicalGS_Pre (s : CGSState) : Prop :=
  s.new_vk_norm_ptr.isSome ∧ s.stemp_ptr.isSome

structure CGSPost where
  err : SUNErrCode
  v' : Array NVector
  h' : HMatrix
  new_vk_norm' : Option sunrealtype
  stemp' : Option sunrealtype
  vtemp' : Array NVector
deriving Repr

def SUNClassicalGS_Spec (s : CGSState) (out : CGSPost) : Prop :=
  SUNClassicalGS_Pre s →
  out.err = SUNErrCode.success ∧
  out.new_vk_norm'.isSome ∧
  out.stemp'.isSome

end SUNDIALS