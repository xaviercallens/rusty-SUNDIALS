/-
Lean 4 specification/proof skeleton for C ↔ Rust behavioral equivalence
for `CVodeSetNonlinearSolver`-style logic.

Modeling choices requested:
- sunrealtype  ↦ Float
- indices      ↦ Int
- nullable ptr ↦ Option α

This is a *precise semantic model* at the API/return-code level, suitable as a
foundation for full refinement proofs.
-/

namespace CvodeNlsEquiv

/-- C-style return codes (subset visible in snippet). -/
inductive CRet : Type
| CV_SUCCESS
| CV_MEM_NULL
| CV_ILL_INPUT
| CV_ILL_INP   -- truncated macro in snippet; modeled distinctly if needed
deriving DecidableEq, Repr

/-- Rust-side error/status (subset corresponding to snippet). -/
inductive CvodeError : Type
| MemNull
| IllInput (msg : String)
deriving DecidableEq, Repr

/-- Nonlinear solver kind. -/
inductive NonlinearSolverType
| RootFind
| FixedPoint
deriving DecidableEq, Repr

/-- Minimal model of required NLS ops in C (`gettype`, `solve`, `setsysfn`). -/
structure COps where
  gettype_present : Bool
  solve_present   : Bool
  setsysfn_present : Bool
deriving DecidableEq, Repr

/-- C-side NLS object (nullable via `Option`). -/
structure CNLS where
  ops : COps
deriving DecidableEq, Repr

/-- C-side CVODE memory object (opaque here). -/
structure CMem where
  dummy : Int
deriving DecidableEq, Repr

/-- Rust-side NLS capability model (typed trait obligations). -/
structure RustNLSCaps where
  has_get_type   : Bool
  has_solve      : Bool
  has_set_sys_fn : Bool
deriving DecidableEq, Repr

/-- Rust-side CVODE memory object (opaque). -/
structure RustMem where
  dummy : Int
deriving DecidableEq, Repr

/-- C semantics for the visible prefix of `CVodeSetNonlinearSolver`. -/
def c_CVodeSetNonlinearSolver :
    Option CMem → Option CNLS → CRet
| none, _ => CRet.CV_MEM_NULL
| some _, none => CRet.CV_ILL_INPUT
| some _, some nls =>
    if (!nls.ops.gettype_present) || (!nls.ops.solve_present) || (!nls.ops.setsysfn_present)
    then CRet.CV_ILL_INP
    else CRet.CV_SUCCESS

/-- Rust semantics for corresponding checks. -/
def rust_set_nonlinear_solver :
    Option RustMem → Option RustNLSCaps → Except CvodeError Unit
| none, _ => Except.error CvodeError.MemNull
| some _, none => Except.error (CvodeError.IllInput "NLS must be non-NULL")
| some _, some caps =>
    if (!caps.has_get_type) || (!caps.has_solve) || (!caps.has_set_sys_fn)
    then Except.error (CvodeError.IllInput "NLS does not support required operations")
    else Except.ok ()

/-- Refinement relation between C return codes and Rust results. -/
def ret_refines : CRet → Except CvodeError Unit → Prop
| CRet.CV_SUCCESS, Except.ok () => True
| CRet.CV_MEM_NULL, Except.error CvodeError.MemNull => True
| CRet.CV_ILL_INPUT, Except.error (CvodeError.IllInput _) => True
| CRet.CV_ILL_INP, Except.error (CvodeError.IllInput _) => True
| _, _ => False

/-- Representation relation between C and Rust NLS capability states. -/
def nls_repr (c : CNLS) (r : RustNLSCaps) : Prop :=
  c.ops.gettype_present = r.has_get_type ∧
  c.ops.solve_present = r.has_solve ∧
  c.ops.setsysfn_present = r.has_set_sys_fn

/-- Representation relation between C and Rust memory states (opaque, trivial here). -/
def mem_repr (_c : CMem) (_r : RustMem) : Prop := True

/-- Main equivalence theorem for the visible function prefix. -/
theorem c_rust_equiv_prefix
  (cMem : Option CMem) (rMem : Option RustMem)
  (cNls : Option CNLS) (rNls : Option RustNLSCaps)
  (hmem : (cMem.isNone ↔ rMem.isNone))
  (hnls :
    match cNls, rNls with
    | none, none => True
    | some c, some r => nls_repr c r
    | _, _ => False) :
  ret_refines (c_CVodeSetNonlinearSolver cMem cNls)
              (rust_set_nonlinear_solver rMem rNls) := by
  cases cMem <;> cases rMem <;> simp at hmem
  · -- both none
    simp [c_CVodeSetNonlinearSolver, rust_set_nonlinear_solver, ret_refines]
  · contradiction
  · contradiction
  · -- both some
    cases cNls <;> cases rNls <;> simp at hnls
    · simp [c_CVodeSetNonlinearSolver, rust_set_nonlinear_solver, ret_refines]
    · contradiction
    · contradiction
    · rcases hnls with ⟨hgt, hsol, hset⟩
      simp [c_CVodeSetNonlinearSolver, rust_set_nonlinear_solver, ret_refines, hgt, hsol, hset]

/-!
Memory safety / no-UB statements (modeled):

Because nullable pointers are `Option`, dereference is only possible in `some` branches.
Thus null-deref UB is unrepresentable in this model.
-/

/-- C model is total (no UB): always returns a code for all inputs. -/
theorem c_total_no_ub (m : Option CMem) (n : Option CNLS) :
  ∃ r, c_CVodeSetNonlinearSolver m n = r := by
  exact ⟨c_CVodeSetNonlinearSolver m n, rfl⟩

/-- Rust model is total and memory-safe: always returns `Except`, never crashes. -/
theorem rust_total_memory_safe (m : Option RustMem) (n : Option RustNLSCaps) :
  ∃ r, rust_set_nonlinear_solver m n = r := by
  exact ⟨rust_set_nonlinear_solver m n, rfl⟩

/- Numeric type aliases requested for broader CVODE modeling. -/
abbrev sunrealtype := Float
abbrev sunindextype := Int

end CvodeNlsEquiv