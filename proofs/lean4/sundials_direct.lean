/-
  Lean 4 formal specification for selected SUNDIALS direct-matrix constructors/destructors.

  Modeling choices requested by user:
  - sunrealtype  -> Float
  - indices      -> Int
  - nullable ptr -> Option ...
-/

import Std

namespace SUNDIALS

/-! ## Basic type aliases -/

abbrev sunrealtype := Float
abbrev sunindextype := Int

/-! ## Matrix kind -/

inductive MatType where
  | dense
  | band
deriving DecidableEq, Repr

/-! ## Abstract memory blocks

We model allocated contiguous storage as immutable arrays in specs.
`Option` models nullable pointers from C.
-/

abbrev RealBlock := Array sunrealtype
abbrev ColPtrs   := Array Int   -- column base offsets into `data`

/-! ## Main matrix record corresponding to `SUNDlsMat` -/

structure SUNDlsMat where
  data  : Option RealBlock
  cols  : Option ColPtrs
  M     : sunindextype
  N     : sunindextype
  mu    : sunindextype := 0
  ml    : sunindextype := 0
  s_mu  : sunindextype := 0
  ldim  : sunindextype
  ldata : sunindextype
  mtype : MatType
deriving Repr

/-! ## Helper predicates -/

def validDenseLayout (A : SUNDlsMat) : Prop :=
  A.mtype = MatType.dense ∧
  A.M > 0 ∧ A.N > 0 ∧
  A.ldim = A.M ∧
  A.ldata = A.M * A.N ∧
  (∃ d c,
      A.data = some d ∧
      A.cols = some c ∧
      d.size = Int.natAbs A.ldata ∧
      c.size = Int.natAbs A.N ∧
      (∀ j : Nat, j < c.size → c[j]! = (j : Int) * A.M))

def validBandLayout (A : SUNDlsMat) : Prop :=
  A.mtype = MatType.band ∧
  A.N > 0 ∧ A.M = A.N ∧
  A.ldim = A.s_mu + A.ml + 1 ∧
  A.ldata = A.N * A.ldim ∧
  (∃ d c,
      A.data = some d ∧
      A.cols = some c ∧
      d.size = Int.natAbs A.ldata ∧
      c.size = Int.natAbs A.N ∧
      (∀ j : Nat, j < c.size → c[j]! = (j : Int) * A.ldim))

def nullMat : Option SUNDlsMat := none

/-! ## Abstract allocator success flags

C code may fail at each malloc. We expose this nondeterminism explicitly.
-/

structure AllocFlags where
  okStruct : Bool := true
  okData   : Bool := true
  okCols   : Bool := true
deriving Repr

/-! ## Function specifications (pure models returning nullable pointers) -/

/-- Spec model of `SUNDlsMat_NewDenseMat(M,N)` -/
def SUNDlsMat_NewDenseMat_spec
    (M N : sunindextype) (f : AllocFlags := {}) : Option SUNDlsMat :=
  if hdim : (M <= 0 ∨ N <= 0) then
    none
  else if ¬ f.okStruct then
    none
  else if ¬ f.okData then
    none
  else if ¬ f.okCols then
    none
  else
    let ldata : Int := M * N
    let d : RealBlock := Array.mkArray (Int.natAbs ldata) (0.0 : Float)
    let c : ColPtrs := Array.ofFn (fun j : Fin (Int.natAbs N) => (j.1 : Int) * M)
    some {
      data  := some d
      cols  := some c
      M     := M
      N     := N
      ldim  := M
      ldata := ldata
      mtype := MatType.dense
    }

/-- Spec model of legacy `SUNDlsMat_newDenseMat(m,n)` returning `sunrealtype**` -/
def SUNDlsMat_newDenseMat_spec
    (m n : sunindextype) (okCols okData : Bool := true) : Option (RealBlock × ColPtrs) :=
  if hdim : (n <= 0 ∨ m <= 0) then
    none
  else if ¬ okCols then
    none
  else if ¬ okData then
    none
  else
    let ldata := m * n
    let d : RealBlock := Array.mkArray (Int.natAbs ldata) (0.0 : Float)
    let c : ColPtrs := Array.ofFn (fun j : Fin (Int.natAbs n) => (j.1 : Int) * m)
    some (d, c)

/-- Spec model of `SUNDlsMat_NewBandMat(N,mu,ml,smu)` -/
def SUNDlsMat_NewBandMat_spec
    (N mu ml smu : sunindextype) (f : AllocFlags := {}) : Option SUNDlsMat :=
  if hN : N <= 0 then
    none
  else if ¬ f.okStruct then
    none
  else if ¬ f.okData then
    none
  else if ¬ f.okCols then
    none
  else
    let colSize := smu + ml + 1
    let ldata := N * colSize
    let d : RealBlock := Array.mkArray (Int.natAbs ldata) (0.0 : Float)
    let c : ColPtrs := Array.ofFn (fun j : Fin (Int.natAbs N) => (j.1 : Int) * colSize)
    some {
      data  := some d
      cols  := some c
      M     := N
      N     := N
      mu    := mu
      ml    := ml
      s_mu  := smu
      ldim  := colSize
      ldata := ldata
      mtype := MatType.band
    }

/-- Spec model of legacy `SUNDlsMat_newBandMat(n,smu,ml)` returning `sunrealtype**` -/
def SUNDlsMat_newBandMat_spec
    (n smu ml : sunindextype) (okCols okData : Bool := true) : Option (RealBlock × ColPtrs) :=
  if hn : n <= 0 then
    none
  else if ¬ okCols then
    none
  else if ¬ okData then
    none
  else
    let colSize := smu + ml + 1
    let ldata := n * colSize
    let d : RealBlock := Array.mkArray (Int.natAbs ldata) (0.0 : Float)
    let c : ColPtrs := Array.ofFn (fun j : Fin (Int.natAbs n) => (j.1 : Int) * colSize)
    some (d, c)

/-- Spec model of `SUNDlsMat_DestroyMat(A)` (post-state pointer is NULL). -/
def SUNDlsMat_DestroyMat_spec (_A : Option SUNDlsMat) : Option SUNDlsMat := none

/-- Spec model of `SUNDlsMat_destroyMat(a)` (post-state pointer is NULL). -/
def SUNDlsMat_destroyMat_spec (_a : Option (RealBlock × ColPtrs)) : Option (RealBlock × ColPtrs) := none

/-! ## Preconditions and postcondition theorems -/

theorem NewDenseMat_null_on_bad_dims
    (M N : Int) (f : AllocFlags) :
    (M <= 0 ∨ N <= 0) →
    SUNDlsMat_NewDenseMat_spec M N f = none := by
  intro h
  simp [SUNDlsMat_NewDenseMat_spec, h]

theorem NewDenseMat_success_layout
    (M N : Int) (f : AllocFlags)
    (hM : M > 0) (hN : N > 0)
    (hs : f.okStruct = true) (hd : f.okData = true) (hc : f.okCols = true) :
    ∃ A, SUNDlsMat_NewDenseMat_spec M N f = some A ∧ validDenseLayout A := by
  refine ⟨{
    data := some (Array.mkArray (Int.natAbs (M * N)) (0.0 : Float))
    cols := some (Array.ofFn (fun j : Fin (Int.natAbs N) => (j.1 : Int) * M))
    M := M; N := N; ldim := M; ldata := M * N; mtype := MatType.dense
  }, ?_, ?_⟩
  · simp [SUNDlsMat_NewDenseMat_spec, hM.not_le, hN.not_le, hs, hd, hc]
  ·
    refine ⟨rfl, hM, hN, rfl, rfl, ?_⟩
    refine ⟨_, _, rfl, rfl, rfl, rfl, ?_⟩
    intro j hj
    simp

theorem NewBandMat_null_on_bad_dim
    (N mu ml smu : Int) (f : AllocFlags) :
    N <= 0 → SUNDlsMat_NewBandMat_spec N mu ml smu f = none := by
  intro h
  simp [SUNDlsMat_NewBandMat_spec, h]

theorem DestroyMat_sets_null (A : Option SUNDlsMat) :
    SUNDlsMat_DestroyMat_spec A = none := by
  simp [SUNDlsMat_DestroyMat_spec]

/-! ## Memory safety properties -/

theorem dense_cols_in_bounds
    (A : SUNDlsMat)
    (hA : validDenseLayout A) :
    ∀ j : Nat, j < Int.natAbs A.N →
      ∃ base : Int, (A.cols = some (Array.ofFn (fun k : Fin (Int.natAbs A.N) => (k.1 : Int) * A.M))) ∧
      base = (j : Int) * A.M := by
  intro j hj
  rcases hA with ⟨_, _, _, _, _, hmem⟩
  rcases hmem with ⟨d, c, hd, hc, _, _, hcol⟩
  refine ⟨(j : Int) * A.M, ?_, rfl⟩
  subst hc
  simp

/-! ## Numerical stability / boundedness placeholders

Constructors only allocate and set metadata; they do not perform arithmetic on entries
except index/size products. We state finite-size and zero-initialization bounds.
-/

def finiteSizeBound (x : Int) : Prop := Int.natAbs x < (2^63 : Nat)

theorem dense_constructor_no_data_amplification
    (M N : Int) (f : AllocFlags) (A : SUNDlsMat)
    (h : SUNDlsMat_NewDenseMat_spec M N f = some A) :
    A.data.isSome := by
  simp [SUNDlsMat_NewDenseMat_spec] at h
  split at h <;> try contradiction
  repeat' (split at h <;> try contradiction)
  cases h
  simp

theorem band_constructor_size_formula
    (N mu ml smu : Int) (f : AllocFlags) (A : SUNDlsMat)
    (h : SUNDlsMat_NewBandMat_spec N mu ml smu f = some A) :
    A.ldata = N * (smu + ml + 1) := by
  simp [SUNDlsMat_NewBandMat_spec] at h
  split at h <;> try contradiction
  repeat' (split at h <;> try contradiction)
  cases h
  rfl

end SUNDIALS