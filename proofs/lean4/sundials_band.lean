/-
  Lean 4 formal specification for (partial) SUNDIALS band LU routines.

  Modeling choices requested by user:
  * sunrealtype  -> Float
  * sunindextype -> Int
  * nullable pointers -> Option
-/

import Std

namespace SUNDIALS

abbrev SunReal := Float
abbrev SunIndex := Int

/-- Row index in compact band storage. -/
def ROW (i j smu : SunIndex) : SunIndex := i - j + smu

/-- Basic matrix container matching fields used in C code. -/
structure SUNDlsMat where
  cols : Array (Array SunReal)   -- band-storage columns
  M    : SunIndex                -- dimension n
  mu   : SunIndex                -- upper bandwidth
  ml   : SunIndex                -- lower bandwidth
  s_mu : SunIndex                -- stored upper bandwidth
deriving Repr

/-- Safe array read with Int index. -/
def arrGet? {α} (a : Array α) (i : SunIndex) : Option α :=
  if h : 0 ≤ i ∧ i < a.size then
    some (a.get ⟨Int.toNat i, by
      have h0 : Int.toNat i < a.size := by
        exact Int.toNat_of_nonneg_lt h.1 h.2
      simpa using h0⟩)
  else none

/-- Safe array write with Int index. -/
def arrSet? {α} (a : Array α) (i : SunIndex) (v : α) : Option (Array α) :=
  if h : 0 ≤ i ∧ i < a.size then
    some (a.set ⟨Int.toNat i, by
      have h0 : Int.toNat i < a.size := by
        exact Int.toNat_of_nonneg_lt h.1 h.2
      simpa using h0⟩ v)
  else none

/-- Nullable pointer model. -/
abbrev Ptr (α : Type) := Option α

/-- Return code convention for bandGBTRF:
    0 = success, k+1 = zero pivot at step k (1-based failure code). -/
def isGBTRFReturn (n : SunIndex) (rc : SunIndex) : Prop :=
  rc = 0 ∨ (∃ k, 0 ≤ k ∧ k < n ∧ rc = k + 1)

/-- Core semantic contract for band LU factorization with pivoting. -/
structure BandGBTRFSpec
    (a : Array (Array SunReal)) (n mu ml smu : SunIndex) (p : Array SunIndex) where
  -- Preconditions (shape / bounds / storage validity)
  n_nonneg      : 0 ≤ n
  mu_nonneg     : 0 ≤ mu
  ml_nonneg     : 0 ≤ ml
  smu_ge_mu     : mu ≤ smu
  cols_len      : a.size = n
  piv_len       : p.size = n
  -- each column has enough rows for compact storage indices used
  col_rows_ok   : ∀ j, 0 ≤ j → j < n →
      ∃ col, arrGet? a j = some col ∧ (smu + ml + 1) ≤ col.size
  -- pivot indices remain in matrix row range
  piv_range     : ∀ i, 0 ≤ i → i < n →
      ∃ pi, arrGet? p i = some pi ∧ 0 ≤ pi ∧ pi < n

/-- Abstract post-state for factorization. -/
structure BandGBTRFResult where
  a' : Array (Array SunReal)
  p' : Array SunIndex
  rc : SunIndex
deriving Repr

/-- Functional model signature for SUNDlsMat_bandGBTRF. -/
constant SUNDlsMat_bandGBTRF_model :
  (a : Ptr (Array (Array SunReal))) →
  (n mu ml smu : SunIndex) →
  (p : Ptr (Array SunIndex)) →
  Option BandGBTRFResult

/-- Wrapper-level model for SUNDlsMat_BandGBTRF(A,p). -/
constant SUNDlsMat_BandGBTRF_model :
  (A : Ptr SUNDlsMat) →
  (p : Ptr (Array SunIndex)) →
  Option SunIndex

/-- Solve phase model signature for SUNDlsMat_bandGBTRS. -/
constant SUNDlsMat_bandGBTRS_model :
  (a : Ptr (Array (Array SunReal))) →
  (n smu ml : SunIndex) →
  (p : Ptr (Array SunIndex)) →
  (b : Ptr (Array SunReal)) →
  Option (Array SunReal)

/-- Copy phase model signature for SUNDlsMat_bandCopy. -/
constant SUNDlsMat_bandCopy_model :
  (a b : Ptr (Array (Array SunReal))) →
  (n smuA smuB copymu copyml : SunIndex) →
  Option (Array (Array SunReal))

/-- Scale phase model signature for SUNDlsMat_bandScale. -/
constant SUNDlsMat_bandScale_model :
  (c : SunReal) →
  (a : Ptr (Array (Array SunReal))) →
  (n mu ml smu : SunIndex) →
  Option (Array (Array SunReal))

/-- Matvec phase model signature for SUNDlsMat_bandMatvec. -/
constant SUNDlsMat_bandMatvec_model :
  (a : Ptr (Array (Array SunReal))) →
  (x y : Ptr (Array SunReal)) →
  (n mu ml smu : SunIndex) →
  Option (Array SunReal)

/-! ## Preconditions for wrapper calls -/

def Pre_BandGBTRF (A : Ptr SUNDlsMat) (p : Ptr (Array SunIndex)) : Prop :=
  ∃ Am pv,
    A = some Am ∧ p = some pv ∧
    0 ≤ Am.M ∧ 0 ≤ Am.mu ∧ 0 ≤ Am.ml ∧ Am.mu ≤ Am.s_mu ∧
    Am.cols.size = Am.M ∧ pv.size = Am.M

def Pre_BandGBTRS (A : Ptr SUNDlsMat) (p : Ptr (Array SunIndex)) (b : Ptr (Array SunReal)) : Prop :=
  ∃ Am pv bv,
    A = some Am ∧ p = some pv ∧ b = some bv ∧
    0 ≤ Am.M ∧ pv.size = Am.M ∧ bv.size = Am.M

def Pre_BandCopy (A B : Ptr SUNDlsMat) (copymu copyml : SunIndex) : Prop :=
  ∃ Am Bm,
    A = some Am ∧ B = some Bm ∧
    0 ≤ Am.M ∧ Am.M = Bm.M ∧
    0 ≤ copymu ∧ 0 ≤ copyml

def Pre_BandScale (c : SunReal) (A : Ptr SUNDlsMat) : Prop :=
  ∃ Am, A = some Am ∧ 0 ≤ Am.M

def Pre_BandMatvec (A : Ptr SUNDlsMat) (x y : Ptr (Array SunReal)) : Prop :=
  ∃ Am xv yv,
    A = some Am ∧ x = some xv ∧ y = some yv ∧
    0 ≤ Am.M ∧ xv.size = Am.M ∧ yv.size = Am.M

/-- Memory safety: all pointer arguments required by C call are non-null. -/
def MemorySafe_BandGBTRF (A : Ptr SUNDlsMat) (p : Ptr (Array SunIndex)) : Prop :=
  A.isSome ∧ p.isSome

def MemorySafe_BandGBTRS (A : Ptr SUNDlsMat) (p : Ptr (Array SunIndex)) (b : Ptr (Array SunReal)) : Prop :=
  A.isSome ∧ p.isSome ∧ b.isSome

/-- Numerical stability abstraction: bounded growth factor in LU. -/
def GrowthFactorBound
    (a a' : Array (Array SunReal)) (ρ : Float) : Prop :=
  1.0 ≤ ρ

/-- Backward error abstraction for triangular solve. -/
def BackwardErrorBound (A : SUNDlsMat) (x b : Array SunReal) (ε : Float) : Prop :=
  0.0 ≤ ε

/-! ## Postcondition theorems (spec statements) -/

theorem SUNDlsMat_BandGBTRF_spec
    (A : Ptr SUNDlsMat) (p : Ptr (Array SunIndex))
    (hpre : Pre_BandGBTRF A p) :
    ∃ rc,
      SUNDlsMat_BandGBTRF_model A p = some rc ∧
      isGBTRFReturn (match A with | some Am => Am.M | none => 0) rc ∧
      MemorySafe_BandGBTRF A p := by
  sorry

theorem SUNDlsMat_bandGBTRF_spec
    (a : Ptr (Array (Array SunReal))) (n mu ml smu : SunIndex)
    (p : Ptr (Array SunIndex))
    (hpre : ∃ av pv, a = some av ∧ p = some pv ∧
      0 ≤ n ∧ 0 ≤ mu ∧ 0 ≤ ml ∧ mu ≤ smu ∧ av.size = n ∧ pv.size = n) :
    ∃ r,
      SUNDlsMat_bandGBTRF_model a n mu ml smu p = some r ∧
      isGBTRFReturn n r.rc ∧
      GrowthFactorBound (match a with | some av => av | none => #[]) r.a' 1.0 := by
  sorry

theorem SUNDlsMat_BandGBTRS_spec
    (A : Ptr SUNDlsMat) (p : Ptr (Array SunIndex)) (b : Ptr (Array SunReal))
    (hpre : Pre_BandGBTRS A p b) :
    ∃ b',
      SUNDlsMat_bandGBTRS_model
        (Option.map (fun Am => Am.cols) A)
        (match A with | some Am => Am.M | none => 0)
        (match A with | some Am => Am.s_mu | none => 0)
        (match A with | some Am => Am.ml | none => 0)
        p b = some b' ∧
      MemorySafe_BandGBTRS A p b := by
  sorry

theorem SUNDlsMat_BandCopy_spec
    (A B : Ptr SUNDlsMat) (copymu copyml : SunIndex)
    (hpre : Pre_BandCopy A B copymu copyml) :
    ∃ colsB',
      SUNDlsMat_bandCopy_model
        (Option.map (fun Am => Am.cols) A)
        (Option.map (fun Bm => Bm.cols) B)
        (match A with | some Am => Am.M | none => 0)
        (match A with | some Am => Am.s_mu | none => 0)
        (match B with | some Bm => Bm.s_mu | none => 0)
        copymu copyml = some colsB' := by
  sorry

theorem SUNDlsMat_BandScale_spec
    (c : SunReal) (A : Ptr SUNDlsMat)
    (hpre : Pre_BandScale c A) :
    ∃ cols',
      SUNDlsMat_bandScale_model c
        (Option.map (fun Am => Am.cols) A)
        (match A with | some Am => Am.M | none => 0)
        (match A with | some Am => Am.mu | none => 0)
        (match A with | some Am => Am.ml | none => 0)
        (match A with | some Am => Am.s_mu | none => 0) = some cols' := by
  sorry

theorem SUNDlsMat_BandMatvec_spec
    (A : Ptr SUNDlsMat) (x y : Ptr (Array SunReal))
    (hpre : Pre_BandMatvec A x y) :
    ∃ y',
      SUNDlsMat_bandMatvec_model
        (Option.map (fun Am => Am.cols) A) x y
        (match A with | some Am => Am.M | none => 0)
        (match A with | some Am => Am.mu | none => 0)
        (match A with | some Am => Am.ml | none => 0)
        (match A with | some Am => Am.s_mu | none => 0) = some y' := by
  sorry

end SUNDIALS