# qf-bao-distances

BAO distances for flat FLRW cosmologies, computed on this repository's own CVODE, and a Gaussian χ² /
best fit against the real DESI DR2 BAO data vector. Pure Rust; dependencies are `crates/cvode` and
`crates/nvector` only.

This is the Rust port of the distance numerics behind two AutoevolveAI cosmology exercises
([github.com/xaviercallens/AutoevolveAI](https://github.com/xaviercallens/AutoevolveAI)):

- `results/bao_flcdm`: DESI DR1 flat-ΛCDM BAO fit.
- `results/desi_dr2_bao`: DESI DR2 ΛCDM / wCDM BAO fit. Its Python χ²-minimum is used below as an
  independent positive control.

## What it computes

| Quantity | Definition |
|---|---|
| `E(z)` | flat ΛCDM, flat wCDM (constant `w`), flat w0waCDM (CPL, `w(a) = w0 + wa(1−a)`) |
| radiation | **off by default**: `E² = Ωm(1+z)³ + (1−Ωm) f_DE(z)`. `with_radiation(Radiation::PLANCK)` adds photons plus `N_eff` massless neutrinos, matching astropy `FlatLambdaCDM(Tcmb0, Neff, m_nu=0)`. With radiation on, `E(z)` depends on `h` separately. |
| `χ(z)` | `∫₀ᶻ dz'/E(z')`: either CVODE on `dχ/dz = 1/E(z)` or adaptive Gauss–Kronrod G7/K15 |
| `D_M = D_C` | `(c/H0) χ(z)` (flat) |
| `D_H` | `c / H(z)` |
| `D_V` | `(z D_M² D_H)^{1/3}` |
| `/r_d` | all three divided by the sound horizon. With radiation off they depend on `(h, r_d)` only through `h·r_d` [Mpc]. |
| χ² | `rᵀ C⁻¹ r` over a DESI-format mean file (`z value DM_over_rs/DH_over_rs/DV_over_rs`) and covariance file, via Cholesky |
| fit | flat ΛCDM `(Ωm, h·r_d)`: a 2-D grid seeds Nelder–Mead, and the Hessian at the minimum gives Gaussian (Fisher) errors |

The DESI DR2 files are read from
`/mnt/disks/disk-socrateai-local-1/dualscale-data-r3/desi_sdss_bao/desi_bao_dr2/desi_gaussian_bao_ALL_GCcomb_{mean,cov}.txt`.
Set `QF_BAO_DR2_DIR` to override the directory. Tests that need the files print `SKIP` and return
when the files are absent.

## Solver findings (measured on `crates/cvode`, not changed here)

1. **The CVODE path uses BDF, not Adams.** Adams was the intended method, since the problem is
   non-stiff. But `Method::Adams` never leaves order 1: `compute_l` in `crates/cvode/src/solver.rs`
   is a placeholder with all coefficients set to 1. On `χ' = (1+z)^{-3/2}` its error scales like
   `√rtol`: 2.1e-4 relative at `rtol = 1e-7` after about 7,000 steps, and 6.9e-5 at `rtol = 1e-8`
   after about 21,000 steps. cvode's own Adams test only asserts `|y − 5| < 3.5` on `y' = 1`.
   BDF reaches order 5.
2. **Many outputs from one BDF run lose accuracy.** Forty `tout`s from one run gave 4e-6 worst
   relative error at `rtol = 1e-7`. A fresh solve per redshift gives about 6e-7, so `chi_cvode`
   uses one fresh solve per distinct redshift.
3. **`rtol ≤ 1e-8` fails** ("too many error test failures at one step") on the Einstein–de Sitter
   integrand. The CVODE path therefore runs at `rtol = 1e-7`, `atol = 1e-10`. Its measured global
   error there is several times `rtol`. Setting `init_step ≥ 1e-4` makes it about 2e-4, so no
   initial step is set.

Consequence: the requested bound of 1e-7 against the EdS closed form is **not met by the CVODE path**
(measured 7.4e-7). That test is kept at 1e-7 and marked `#[ignore]` with this reason, so that
`cargo test -- --ignored` shows the real failure. An active test asserts the bound CVODE does reach
(1e-6). The quadrature path meets 1e-7 with about seven orders of magnitude to spare.

## Validation

Output of `cargo test -p qf-bao-distances -- --nocapture` (debug build). 12 passed and 1 ignored; the
ignored test fails when run, as intended.

| Check | Target | Measured |
|---|---|---|
| EdS closed form `2(1 − 1/√(1+z))`, quadrature, z ∈ [0.1, 4] | < 1e-7 | 1.3e-15 |
| EdS closed form, CVODE BDF rtol 1e-7 | < 1e-7 (spec) | **7.4e-7, NOT MET** (ignored test) |
| EdS closed form, CVODE BDF rtol 1e-7 | < 1e-6 (achieved) | 7.4e-7 (9,356 steps over 40 redshifts) |
| wCDM(w=−1) and w0waCDM(−1, 0) vs ΛCDM | exact equality | `E(z)` and all distances bit-identical |
| CVODE vs quadrature, z ∈ [0.1, 4]: ΛCDM / wCDM(−0.85) / CPL(−0.75, −0.8) / ΛCDM+radiation | < 1e-6 | 4.4e-7 / 6.3e-7 / 6.9e-7 / 3.6e-7 |
| astropy 8.0.1 `FlatLambdaCDM(67.36, 0.3153, Tcmb0=0)`, D_C at 7 DESI redshifts, both paths | < 1e-6 | pass; `H(z)` to 1e-12 |
| astropy radiation-on (`Tcmb0=2.7255, Neff=3.046`), FlatwCDM(−0.9), Flatw0waCDM(−0.75, −0.8) | < 1e-6 | pass; `Ω_r` matches astropy's `Ogamma0 + Onu0` to 1e-6 |
| Negative control: Ωm × 1.01 moves D_M(z=1) | > 1e-3 | 1.8e-3 (and in the right direction) |
| DR2: EdS / h·r_d + 5 % vs published point | Δχ² > 100 / > 25 | χ² 11089 / 306 vs 10.27 |

### DESI DR2 flat-ΛCDM best fit (13 points, radiation off)

| | Ωm | h·r_d [Mpc] | corr | χ²_min / dof |
|---|---|---|---|---|
| **Rust, CVODE path** | 0.29743 ± 0.00861 | 101.543 ± 0.735 | −0.9239 | 10.2710 / 11 |
| **Rust, quadrature path** | 0.29746 ± 0.00862 | 101.540 ± 0.736 | −0.9240 | 10.2710 / 11 |
| AutoevolveAI Python χ²-min (`results/desi_dr2_bao/fit.json`) | 0.29746 ± 0.00858 | 101.540 ± 0.733 | −0.9234 | 10.2710 / 11 |
| DESI published, arXiv:2503.14738 eq. (17) | 0.2975 ± 0.0086 | 101.54 ± 0.73 | — | — |

The test asserts the following:

- Both Rust fits lie within 1σ of the published values.
- The quadrature fit matches the Python minimum to within 2e-4 in Ωm, 0.02 Mpc in h·r_d and 1e-3 in
  χ², and matches its Fisher errors to within 3%.
- The CVODE and quadrature minima agree to within 1% of σ.

Caveat: the published values are posterior means and standard deviations from DESI's full pipeline,
which includes radiation and massive neutrinos. The values here are a χ² minimum with Fisher errors
and radiation off. The agreement is well below 1σ, but these are not the same estimator.

## Use

```rust
use qf_bao_distances::{BaoDataset, FlatCosmology, Integrator, bao_distances, fit_flat_lcdm};

let lcdm = FlatCosmology::lcdm(0.2975, 0.7);
let d = bao_distances(&lcdm, 101.54, &[0.51, 2.33], Integrator::CVODE)?;
if let Some(data) = BaoDataset::desi_dr2() {
    let fit = fit_flat_lcdm(&data?, Integrator::QUADRATURE)?;
}
```

`crates/sundials-mcp` exposes this crate as the `bao_distances` MCP tool.
