# A proposed directory structure for rusty-SUNDIALS

Written 2026-09-27, alongside the `qf-cmb-cascade` crate PR, at the owner's request to "propose a
new GitHub structure." **This is a proposal, not a mandate, and nothing in this document is
executed by that PR.** No existing file is moved, renamed, or deleted here.

## The problem this proposes to solve

`crates/` is a flat list (`sundials-core`, `cvode`, `nvector`, `ida`, `benchmarks`,
`rusty-sundials-py`, `qf-pgpe`, and now `qf-cmb-cascade`) that mixes the actual ODE-solver engine
with Python bindings and domain-specific scientific ports, with no structural signal for which is
which. `examples/` is a single flat directory of 30+ files spanning chaotic-attractor demos
(`lorenz.rs`, `rossler.rs`, `three_body.rs`, `double_pendulum.rs`, `rigid_body.rs`, `vanderpol.rs`),
biology/epidemiology (`fitzhugh_nagumo.rs`, `hodgkin_huxley.rs`, `lotka_volterra.rs`,
`sir_epidemic.rs`), chemistry/kinetics (`hires.rs`, `oregonator.rs`, `brusselator1d.rs`,
`robertson*.rs`), PDE/finite-difference demos (`heat1d_banded.rs`, `cv_advdiff_bnd.rs`,
`grand_unified_gray_scott.rs`), plasma/fusion (`fusion_mhd_benchmark.rs`, `fusion_sciml_phase5.rs`,
`iter_disruption*.rs`, `tearing_mode_hero_test.rs`), photonics (`optimize_photonics.rs`), several
`exp{1..5}_*.rs` experiment files whose names alone don't say what they demonstrate, and (currently
parked in another session's stash, not touched by this proposal or its PR) new cosmology and
climate examples. A new contributor — or a future session picking this up cold — has to read every
filename to find the one relevant to their question.

## The proposed structure

```
crates/
  core/               (grouping note only — see "Cargo caveat" below)
    sundials-core/
    cvode/
    ida/
    nvector/
  bindings/
    rusty-sundials-py/
  physics/
    qf-pgpe/
    qf-cmb-cascade/
    (future: plasma/climate/cosmology domain crates, once the parked work lands and is reviewed)
  benchmarks/          (unchanged)

examples/
  chaos/              lorenz, rossler, three_body, double_pendulum, rigid_body, vanderpol
  biology/            fitzhugh_nagumo, hodgkin_huxley, lotka_volterra, sir_epidemic
  chemistry/          hires, oregonator, brusselator1d, robertson, robertson_csv, robertson_paper_data
  pde/                heat1d_banded, cv_advdiff_bnd, grand_unified_gray_scott
  plasma-fusion/      fusion_mhd_benchmark, fusion_sciml_phase5, iter_disruption, iter_disruption_3d,
                      tearing_mode_hero_test
  photonics/          optimize_photonics
  benchmarks/         bench_nvector, benchmark_a100_hpc, cv_advdiff_bnd (if not counted under pde)
  experiments/        exp1_dynamic_imex .. exp5_fogno_xmhd (renamed, if their authors are willing,
                      to names that say what they demonstrate rather than a bare number)
  cosmology/          (reserved, empty until the parked work lands)
  climate/            (reserved, empty until the parked work lands)
```

### Cargo caveat

Cargo has no native concept of a crate "group" beyond directory layout — `crates/core/cvode` is
just a longer path to the same crate, and every `workspace.members` entry in the root `Cargo.toml`
would need its path updated to match. This is real, mechanical churn: every crate's own
`[dependencies]` path references (if any use relative paths rather than the crate name) and every
downstream `Cargo.toml`/import path would need the same update, and any in-flight branch (referring
to old paths) would conflict. `examples/` reorganizing into subdirectories is lower-risk in
principle (Cargo's `[[example]]` targets can point anywhere via `path = "..."`), but with 30+ files
and a single `examples/Cargo.toml`, it's still a mechanical, easy-to-get-wrong bulk edit.

## What this document recommends

1. **Do not bundle this into the `qf-cmb-cascade` PR**, and this PR does not. That PR adds exactly
   one new crate at `crates/qf-cmb-cascade/` and this document; nothing existing moves.
2. **Treat the reorganization as its own PR**, timed for a moment when no other branch has
   in-flight changes to `examples/` or `crates/*/Cargo.toml` — in particular, coordinate with
   whichever session's cosmology/climate work is currently stashed, since moving files out from
   under a parked `git stash` is exactly the kind of silent conflict this project's own established
   discipline (stash-and-restore before touching another session's work) exists to prevent.
3. **If approved, do the crate move and the example move as two separate PRs**, not one — the
   crate move touches the root `Cargo.toml` and is higher-risk; the example reorganization is purely
   additive (new subdirectories) and can be done, tested, and reviewed independently.
4. **Rename the `exp{1..5}_*.rs` files as part of (or before) the example reorganization**, once
   whoever wrote them can say what each one demonstrates — a numbered filename is exactly the kind
   of thing this reorganization is meant to fix, and carrying the number forward into a new
   subdirectory (`examples/experiments/exp3_flagno.rs`) would not actually solve the problem.
