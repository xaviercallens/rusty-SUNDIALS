/-
Lean 4 specification/proof skeleton for C ↔ Rust equivalence
for SUNDls dense/band constructors.

Modeling choices requested:
- sunrealtype  ↦ Float
- indices      ↦ Int
- nullable ptr ↦ Option
- preconditions as hypotheses
- postconditions as theorems
-/

namespace SundialsEq

abbrev SunReal := Float
abbrev SunIndex := Int

inductive MatType where
  | dense
  | band
deriving DecidableEq, Repr

/-- Abstract matrix object (heap content modeled functionally). -/
structure SUNDlsMat where
  data  : Array SunReal
  cols  : Array Int          -- pointer offsets into `data`
  M     : SunIndex
  N     : SunIndex
  mu    : SunIndex
  ml    : SunIndex
  smu   : SunIndex
  ldim  : SunIndex
  ldata : SunIndex
  mtype : MatType
deriving Repr

/-- C result: nullable pointer + possible allocation failure. -/
abbrev CPtr (α : Type) := Option α

/-- Rust result (simplified). -/
inductive CvodeError where
  | illegalInput
  | memFail
deriving DecidableEq, Repr

abbrev RustResult (α : Type) := Except CvodeError α

/-- Dense constructor semantics for C (spec-level, total). -/
def c_newDense (m n : SunIndex) : CPtr SUNDlsMat :=
  if h : (m ≤ 0 ∨ n ≤ 0) then
    none
  else
    let ldata := m * n
    let data  := Array.mkArray (Int.toNat ldata) (0.0 : SunReal)
    let cols  := Array.ofFn (fun j : Fin (Int.toNat n) => (j.1 : Int) * m)
    some {
      data := data
      cols := cols
      M := m; N := n
      mu := 0; ml := 0; smu := 0
      ldim := m
      ldata := ldata
      mtype := .dense
    }

/-- Dense constructor semantics for Rust (spec-level). -/
def rust_newDense (m n : SunIndex) : RustResult SUNDlsMat :=
  if h : (m ≤ 0 ∨ n ≤ 0) then
    .error .illegalInput
  else
    let ldata := m * n
    let data  := Array.mkArray (Int.toNat ldata) (0.0 : SunReal)
    let cols  := Array.ofFn (fun j : Fin (Int.toNat n) => (j.1 : Int) * m)
    .ok {
      data := data
      cols := cols
      M := m; N := n
      mu := 0; ml := 0; smu := 0
      ldim := m
      ldata := ldata
      mtype := .dense
    }

/-- Band constructor semantics for C (partial snippet completed by spec). -/
def c_newBand (n mu ml smu : SunIndex) : CPtr SUNDlsMat :=
  if h : n ≤ 0 then
    none
  else
    let colSize := smu + ml + 1
    let ldata := n * colSize
    let data := Array.mkArray (Int.toNat ldata) (0.0 : SunReal)
    let cols := Array.ofFn (fun j : Fin (Int.toNat n) => (j.1 : Int) * colSize)
    some {
      data := data
      cols := cols
      M := n; N := n
      mu := mu; ml := ml; smu := smu
      ldim := colSize
      ldata := ldata
      mtype := .band
    }

/-- Rust band constructor semantics. -/
def rust_newBand (n mu ml smu : SunIndex) : RustResult SUNDlsMat :=
  if h : n ≤ 0 then
    .error .illegalInput
  else
    let colSize := smu + ml + 1
    let ldata := n * colSize
    let data := Array.mkArray (Int.toNat ldata) (0.0 : SunReal)
    let cols := Array.ofFn (fun j : Fin (Int.toNat n) => (j.1 : Int) * colSize)
    .ok {
      data := data
      cols := cols
      M := n; N := n
      mu := mu; ml := ml; smu := smu
      ldim := colSize
      ldata := ldata
      mtype := .band
    }

/-! Preconditions and postconditions as theorems -/

/-- Dense success postcondition (shape/layout invariants). -/
theorem dense_post
  (m n : SunIndex)
  (hpos : m > 0 ∧ n > 0) :
  ∃ A, c_newDense m n = some A ∧
    A.M = m ∧ A.N = n ∧
    A.ldim = m ∧ A.ldata = m*n ∧
    A.mtype = .dense := by
  rcases hpos with ⟨hm, hn⟩
  unfold c_newDense
  have hnot : ¬ (m ≤ 0 ∨ n ≤ 0) := by omega
  simp [hnot]
  refine ⟨_, rfl, rfl, rfl, rfl, rfl, rfl⟩

/-- Dense C/Rust behavioral equivalence (ignoring allocator failure distinction). -/
theorem dense_equiv
  (m n : SunIndex)
  (hpos : m > 0 ∧ n > 0) :
  match c_newDense m n, rust_newDense m n with
  | some A, .ok B => A = B
  | _, _ => False := by
  rcases hpos with ⟨hm, hn⟩
  unfold c_newDense rust_newDense
  have hnot : ¬ (m ≤ 0 ∨ n ≤ 0) := by omega
  simp [hnot]

/-- Band C/Rust behavioral equivalence under valid dimension precondition. -/
theorem band_equiv
  (n mu ml smu : SunIndex)
  (hn : n > 0) :
  match c_newBand n mu ml smu, rust_newBand n mu ml smu with
  | some A, .ok B => A = B
  | _, _ => False := by
  unfold c_newBand rust_newBand
  have hnot : ¬ (n ≤ 0) := by omega
  simp [hnot]

/-- No-UB/memory-safety obligation (spec-level):
all computed column offsets are within allocated data. -/
theorem dense_cols_in_bounds
  (m n : SunIndex) (hpos : m > 0 ∧ n > 0) :
  ∀ j : Fin (Int.toNat n),
    let A := (match c_newDense m n with | some a => a | none => panic! "impossible")
    A.cols[j] ≥ 0 ∧ A.cols[j] < A.ldata := by
  intro j
  rcases hpos with ⟨hm, hn⟩
  have hm0 : 0 < m := hm
  have hn0 : 0 < n := hn
  unfold c_newDense
  have hnot : ¬ (m ≤ 0 ∨ n ≤ 0) := by omega
  simp [hnot]
  constructor <;> omega

end SundialsEq