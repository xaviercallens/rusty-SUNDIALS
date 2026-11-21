# Statement review of the files that pass the Lean gate (2026-11-19)

Reviewed at commit `b85156b` (rusty-SUNDIALS `main`). The reader is an AI agent (Sonnet), under the repository owner's
direction; the files were read in full (315 lines in total) and compiled with Lean 4.34.0-rc2 by the gate. This is not a
human review and not an independent one: the same agent also fixed the gate defect described in section 1. The question asked
for each file: does the statement say something non-vacuous about the solver, and is it tied to the C or Rust code?

**Result: none of the four files that passed the first version of the gate should be called Tier A, and none proves
anything about the numerical behaviour of the solver.** Three pass the corrected gate; they are kernel-checked, and what they
check is stated below.

## 1. Gate defect found while reading

`equiv_sundials_matrix.lean` line 96 declares `theorem new_empty_post_ops_null ... : True := by trivial`, a theorem whose
conclusion is `True`. The ratchet added in pull request 86 did not flag it: its vacuity pattern was bounded by the first `:=`,
and the theorem's hypothesis contains `let a := ...`. The earlier gate had flagged the file ("1 of 5 theorems trivial"), so the
new ratchet lost a detection. Fixed in the pull request that adds this document: the pattern is bounded by the next declaration
keyword instead; a new negative control (a `: True` conclusion after a hypothesis containing `:=`) failed on the old pattern
(`static=clean; compile=ok`) and passes on the new one; two accept controls guard against over-matching; across all 72 files
only this one changes. The file is now baselined as known failing (`VACUITY`).

## 2. File by file

| File | Passes corrected gate | What the theorems state | Verdict |
|---|---|---|---|
| `sop2_flagno_scaling.lean` (27 lines) | yes | `flagno_grid_independent_convergence`: `∃ iters, iters = 6 ∧ iters ≤ 7`. `flagno_condition_number_bounded`: `∃ κ_bound, κ_bound < 20 ∧ κ_bound > 0`. Both hypotheses (`grid_128 = 128`, `h_grid > 0`) are unused; both statements are true of any grid and mention no operator, solver or matrix | **Vacuous.** Existential over a constant. The names and docstrings claim FLAGNO grid-independent convergence and a bounded condition number; the statements say nothing of the kind. The header cites an execution ID and a "certificate" that are not linked to any run |
| `sop3_cloud_economics.lean` (30 lines) | yes | arithmetic on hard-coded constants: 17.8 s times two rates is below 0.05; `|18.2 - 17.8| / 17.8 < 0.05` | **True but not about the solver.** The inputs are asserted in the hypotheses, with no link to a measured run or to the cost source. A calculator check in Lean, not a verification of the claim "a run costs less than $0.05" |
| `equiv_cvode_ls.lean` (143 lines) | yes | Two hand-written Lean functions (`c_CVodeSetLinearSolver_prefix`, `rust_CVodeSetLinearSolver_prefix`) with the same three-branch case split (null memory, null linear solver, otherwise ok) are related by `ret_rel`. `c_rust_prefix_equiv` and the three `post_*` theorems assert concrete, non-trivial equalities on those functions. `no_ub_totality` and `rust_memory_safe_totality` are `∃ c, c = f x` | **A real but tiny model-level statement.** The main theorem is not vacuous (it would fail if the two functions disagreed on a branch), but both functions are written by hand in Lean, "modeled from provided snippet": nothing extracts them from the C source or the Rust crate, so fidelity to either is **not established**. The two `∃ ... = ...` theorems are vacuous (true of every function), and the `hpre` hypotheses of `post_mem_null` and `post_ls_null` are `none = none`. Covers three branches of one function prefix: no linear algebra, no numerics |
| `equiv_sundials_matrix.lean` (115 lines) | **no, after the gate fix** | `new_empty_equiv`: a hand-written C-style constructor and a Rust-style constructor agree. `free_empty_memory_safe` and `free_empty_equiv`: both free functions are defined as the constant `none`, so the "equivalence" is two constants being equal. `new_empty_post_ops_null` has the conclusion `True`. `no_ub_new_free` is `∃ c, c = f x` | **Mostly vacuous.** One modest model-level statement (the constructor), two equalities that hold by definition of constants, one `True` theorem, one trivial existential. Same fidelity gap as above: function pointers are modelled as `Option Unit`, so no behaviour is represented |

## 3. What would make such files meaningful

- State the property about the real object: for example a bound on an actual operator's condition number, or a relation between
  the Rust function and the C function **extracted from the sources** (tools such as Aeneas and Charon were the stated plan in the
  repository's architecture notes, but no extraction output is in the repository).
- Remove or rename existential and `True` theorems; a name such as `rust_memory_safe_totality` promises more than `∃ r, r = f x`.
- Tie any numeric constant to its source (the run that measured it), or do not present it as a verification.

## 4. Consequence for the claims

- The ledger rows for these files keep Tier C ("a command was run and the file passes"), not Tier A, and now say what the
  statements are. Tier A requires a statement that a reader has judged non-vacuous and faithful; this review found it for none of
  them.
- The README's "3 files are candidates for kernel-checked status" is withdrawn: they are kernel-checked, but they are not
  evidence about the solver.

## Limits

One reader, no second opinion, no human review. Fidelity was judged by reading the files, not by comparing them with the C
sources or the Rust crate. The claim "none proves anything about the solver's numerics" covers only these four files; the other
68 do not pass the gate and were not reviewed here.
