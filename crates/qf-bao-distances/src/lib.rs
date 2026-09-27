//! Baryon-acoustic-oscillation (BAO) distances for flat FLRW cosmologies, computed on this
//! repository's own CVODE solver, and a Gaussian chi^2 / best fit against the real DESI DR2 BAO
//! data vector.
//!
//! This is a pure-Rust port of the distance numerics behind AutoevolveAI's cosmology exercises
//! (`results/bao_flcdm` — DESI DR1 flat-LCDM fit — and `results/desi_dr2_bao` — DESI DR2
//! LCDM/wCDM fit; <https://github.com/xaviercallens/AutoevolveAI>). Those used Python
//! (`scipy.integrate.quad` / astropy); here the line-of-sight integral is solved as an initial-value
//! problem with `crates/cvode` and cross-checked by an independent adaptive Gauss–Kronrod rule.
//!
//! # What is computed
//!
//! * `E(z) = H(z)/H0` for flat LCDM, flat wCDM (constant `w`) and flat w0waCDM
//!   (Chevallier–Polarski–Linder, `w(a) = w0 + wa (1 - a)`).
//! * Radiation is **off by default** — `E^2 = Om (1+z)^3 + (1-Om) f_DE(z)` — which is what DESI's
//!   own BAO-only `(Om, h r_d)` parametrisation assumes to good accuracy at `z <= 2.33` and what the
//!   AutoevolveAI primary model uses. [`FlatCosmology::with_radiation`] switches on photons plus
//!   `N_eff` massless neutrinos (`Om_r = Om_gamma (1 + 7/8 (4/11)^{4/3} N_eff)`, `Om_DE = 1 - Om -
//!   Om_r`), matching astropy's `FlatLambdaCDM(..., Tcmb0, Neff, m_nu=0)`. With radiation on,
//!   `E(z)` depends on `h` separately, not only through `h r_d`.
//! * Comoving distance `D_C(z) = (c/H0) chi(z)`, `chi(z) = int_0^z dz'/E(z')`, obtained by
//!   integrating `d chi/dz = 1/E(z)` with CVODE (BDF — see "Solver findings" for why not Adams),
//!   or by adaptive Gauss–Kronrod (G7/K15) quadrature.
//! * `D_M = D_C` (flat), `D_H = c/H(z)`, `D_V = (z D_M^2 D_H)^{1/3}`, each divided by the sound
//!   horizon `r_d`. Because `c/H0 = (c/100 km/s) / h` Mpc, `D/r_d` depends on `(h, r_d)` only through
//!   the product `h r_d` (in Mpc) when radiation is off.
//!
//! # Solver findings (measured on this repository's `crates/cvode`, not changed here)
//!
//! 1. **Adams is not a working method.** `Method::Adams` never leaves order 1: `compute_l` in
//!    `crates/cvode/src/solver.rs` is a placeholder (all coefficients 1), and on `chi' =
//!    (1+z)^{-3/2}` the error scales like `sqrt(rtol)` — 2.1e-4 relative at `rtol = 1e-7` after
//!    ~7 000 steps, 6.9e-5 at `rtol = 1e-8` after ~21 000 steps. (cvode's own Adams unit test only
//!    asserts `|y - 5| < 3.5` on `y' = 1`.) This crate therefore uses **BDF**, which reaches order
//!    5 and is accurate, even though the problem is non-stiff.
//! 2. **Many outputs from one run degrade BDF accuracy.** Requesting 40 output redshifts from one
//!    BDF run gave 4e-6 worst relative error at `rtol = 1e-7`; a fresh solve per redshift gives
//!    ~6e-7. [`chi_cvode`] does one fresh solve per distinct redshift.
//! 3. **`rtol <= 1e-8` fails** ("too many error test failures at one step") on the Einstein–de
//!    Sitter integrand (also recorded in `crates/sundials-mcp/README.md`). The CVODE path runs at
//!    [`CVODE_RTOL`] = 1e-7, [`CVODE_ATOL`] = 1e-10, where its measured global error is several
//!    times `rtol` (7.4e-7 relative on the closed-form case, up to 6.9e-7 against quadrature for
//!    w0waCDM, on `0.1 <= z <= 4`). A user-set `init_step >= 1e-4` makes it far worse (~2e-4
//!    relative), so none is set.
//!
//! Every result can be cross-checked against the quadrature path (relative error ~1e-15 on the
//! closed-form case), which has none of these limitations.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::path::{Path, PathBuf};

/// Speed of light in km/s (exact, SI definition).
pub const C_KM_S: f64 = 299_792.458;
/// Hubble distance for `h = 1`, `c / (100 km/s/Mpc)`, in Mpc.
pub const HUBBLE_DISTANCE_H1_MPC: f64 = C_KM_S / 100.0;
/// Relative tolerance used for the CVODE path (see the crate docs for why not tighter).
pub const CVODE_RTOL: f64 = 1e-7;
/// Absolute tolerance used for the CVODE path (`chi` is dimensionless and O(1)).
pub const CVODE_ATOL: f64 = 1e-10;
/// Default relative tolerance of the adaptive Gauss–Kronrod path.
pub const QUAD_RTOL: f64 = 1e-12;
/// Largest redshift accepted (the integrand stays smooth, but the physics here is late-time only).
pub const Z_MAX: f64 = 1100.0;

// ============================================================================================
// Cosmology
// ============================================================================================

/// Dark-energy equation of state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DarkEnergy {
    /// Cosmological constant, `w = -1`.
    Lambda,
    /// Constant equation of state `w`.
    W(f64),
    /// CPL: `w(a) = w0 + wa (1 - a)`.
    Cpl { w0: f64, wa: f64 },
}

impl DarkEnergy {
    /// `rho_DE(z) / rho_DE(0)`.
    pub fn density_ratio(&self, z: f64) -> f64 {
        let zp1 = 1.0 + z;
        match *self {
            DarkEnergy::Lambda => 1.0,
            DarkEnergy::W(w) => zp1.powf(3.0 * (1.0 + w)),
            DarkEnergy::Cpl { w0, wa } => {
                zp1.powf(3.0 * (1.0 + w0 + wa)) * (-3.0 * wa * z / zp1).exp()
            }
        }
    }
}

/// Radiation content: CMB photons at temperature `t_cmb` (K) plus `n_eff` massless neutrinos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radiation {
    pub t_cmb: f64,
    pub n_eff: f64,
}

impl Radiation {
    /// Planck-2018-like default: `T_cmb = 2.7255 K`, `N_eff = 3.046`.
    pub const PLANCK: Radiation = Radiation {
        t_cmb: 2.7255,
        n_eff: 3.046,
    };
}

/// `Omega_gamma h^2` for a black body at `t_cmb` kelvin, from CODATA 2018 constants
/// (`rho_gamma = 4 sigma T^4 / c^3` over `rho_crit/h^2 = 3 (100 km/s/Mpc)^2 / (8 pi G)`).
pub fn omega_gamma_h2(t_cmb: f64) -> f64 {
    const SIGMA_SB: f64 = 5.670_374_419e-8; // W m^-2 K^-4
    const C_M_S: f64 = 299_792_458.0;
    const G: f64 = 6.674_30e-11; // m^3 kg^-1 s^-2
    const MPC_M: f64 = 3.085_677_581_491_367e22;
    let rho_gamma = 4.0 * SIGMA_SB * t_cmb.powi(4) / C_M_S.powi(3);
    let h100 = 1.0e5 / MPC_M; // s^-1
    let rho_crit_h1 = 3.0 * h100 * h100 / (8.0 * std::f64::consts::PI * G);
    rho_gamma / rho_crit_h1
}

/// A spatially flat FLRW cosmology.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatCosmology {
    /// Matter density today, `Omega_m`.
    pub omega_m: f64,
    /// `h = H0 / (100 km/s/Mpc)`. Only enters `E(z)` when radiation is on; it also sets the
    /// absolute distance scale of [`FlatCosmology::comoving_distance_mpc`].
    pub h: f64,
    pub dark_energy: DarkEnergy,
    /// `None` (the default) means no radiation term.
    pub radiation: Option<Radiation>,
}

impl FlatCosmology {
    pub fn lcdm(omega_m: f64, h: f64) -> Self {
        Self {
            omega_m,
            h,
            dark_energy: DarkEnergy::Lambda,
            radiation: None,
        }
    }

    pub fn wcdm(omega_m: f64, w: f64, h: f64) -> Self {
        Self {
            omega_m,
            h,
            dark_energy: DarkEnergy::W(w),
            radiation: None,
        }
    }

    pub fn w0wacdm(omega_m: f64, w0: f64, wa: f64, h: f64) -> Self {
        Self {
            omega_m,
            h,
            dark_energy: DarkEnergy::Cpl { w0, wa },
            radiation: None,
        }
    }

    /// Switch radiation on (photons + massless neutrinos); dark energy absorbs the difference so
    /// the model stays flat.
    pub fn with_radiation(mut self, radiation: Radiation) -> Self {
        self.radiation = Some(radiation);
        self
    }

    /// `Omega_r` today (0 when radiation is off).
    pub fn omega_r(&self) -> f64 {
        match self.radiation {
            None => 0.0,
            Some(r) => {
                let og = omega_gamma_h2(r.t_cmb) / (self.h * self.h);
                let nu_per_species = 7.0 / 8.0 * (4.0_f64 / 11.0).powf(4.0 / 3.0);
                og * (1.0 + nu_per_species * r.n_eff)
            }
        }
    }

    /// `Omega_DE` today, `1 - Omega_m - Omega_r`.
    pub fn omega_de(&self) -> f64 {
        1.0 - self.omega_m - self.omega_r()
    }

    /// `E(z) = H(z) / H0`.
    pub fn e_of_z(&self, z: f64) -> f64 {
        let zp1 = 1.0 + z;
        let or = self.omega_r();
        let e2 = or * zp1.powi(4)
            + self.omega_m * zp1.powi(3)
            + (1.0 - self.omega_m - or) * self.dark_energy.density_ratio(z);
        e2.sqrt()
    }

    /// `H(z)` in km/s/Mpc.
    pub fn hubble(&self, z: f64) -> f64 {
        100.0 * self.h * self.e_of_z(z)
    }

    /// `c / H0` in Mpc.
    pub fn hubble_distance_mpc(&self) -> f64 {
        HUBBLE_DISTANCE_H1_MPC / self.h
    }

    /// Reject parameter values for which `E(z)` is not a positive finite number on `[0, z_max]`.
    fn check(&self, z_max: f64) -> Result<(), String> {
        if !(self.omega_m.is_finite() && self.h.is_finite() && self.h > 0.0) {
            return Err(format!(
                "non-finite or non-positive parameters: Om = {}, h = {}",
                self.omega_m, self.h
            ));
        }
        // E^2 is a sum of monotone power laws and (for CPL) a smooth factor; sampling catches the
        // cases (e.g. Om > 1 with w << -1) where it turns negative inside the range.
        let n = 64;
        for i in 0..=n {
            let z = z_max * i as f64 / n as f64;
            let e = self.e_of_z(z);
            if !(e.is_finite() && e > 0.0) {
                return Err(format!("E(z) = {e} is not positive and finite at z = {z}"));
            }
        }
        Ok(())
    }

    /// Comoving distance in Mpc (`D_C = (c/H0) chi`).
    pub fn comoving_distance_mpc(
        &self,
        zs: &[f64],
        method: Integrator,
    ) -> Result<Vec<f64>, String> {
        let dh = self.hubble_distance_mpc();
        Ok(chi(self, zs, method)?.into_iter().map(|c| dh * c).collect())
    }
}

fn check_redshifts(zs: &[f64]) -> Result<f64, String> {
    let mut zmax = 0.0_f64;
    for &z in zs {
        if !z.is_finite() || !(0.0..=Z_MAX).contains(&z) {
            return Err(format!("redshift {z} outside [0, {Z_MAX}]"));
        }
        zmax = zmax.max(z);
    }
    Ok(zmax)
}

// ============================================================================================
// Line-of-sight integral
// ============================================================================================

/// How to evaluate `chi(z) = int_0^z dz/E`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Integrator {
    /// `crates/cvode`, BDF, on `d chi/dz = 1/E(z)` (one fresh solve per distinct redshift).
    Cvode { rtol: f64, atol: f64 },
    /// Adaptive Gauss–Kronrod G7/K15 with this relative tolerance.
    Quadrature { rtol: f64 },
}

impl Integrator {
    /// CVODE at the crate defaults ([`CVODE_RTOL`], [`CVODE_ATOL`]).
    pub const CVODE: Integrator = Integrator::Cvode {
        rtol: CVODE_RTOL,
        atol: CVODE_ATOL,
    };
    /// Gauss–Kronrod at [`QUAD_RTOL`].
    pub const QUADRATURE: Integrator = Integrator::Quadrature { rtol: QUAD_RTOL };
}

/// Solver statistics from one CVODE integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CvodeStats {
    pub steps: usize,
    pub rhs_evals: usize,
}

/// `chi(z)` for each requested redshift (any order, duplicates allowed).
pub fn chi(cosmo: &FlatCosmology, zs: &[f64], method: Integrator) -> Result<Vec<f64>, String> {
    match method {
        Integrator::Cvode { rtol, atol } => chi_cvode(cosmo, zs, rtol, atol).map(|(c, _)| c),
        Integrator::Quadrature { rtol } => chi_quadrature(cosmo, zs, rtol),
    }
}

/// `chi(z)` by CVODE (BDF) — a fresh integration from 0 to each distinct `z`.
///
/// Why BDF and why one solver per redshift, although the ODE is non-stiff and a single run with
/// dense output would be the textbook choice: both alternatives were measured to be inaccurate in
/// this repository's CVODE (see the crate docs, "Solver findings").
pub fn chi_cvode(
    cosmo: &FlatCosmology,
    zs: &[f64],
    rtol: f64,
    atol: f64,
) -> Result<(Vec<f64>, CvodeStats), String> {
    let zmax = check_redshifts(zs)?;
    cosmo.check(zmax)?;
    let mut out = vec![0.0; zs.len()];
    let mut done: Vec<(f64, f64)> = Vec::new();
    let mut stats = CvodeStats {
        steps: 0,
        rhs_evals: 0,
    };
    for (i, &z) in zs.iter().enumerate() {
        if z == 0.0 {
            continue;
        }
        if let Some(&(_, v)) = done.iter().find(|(zd, _)| *zd == z) {
            out[i] = v;
            continue;
        }
        let c = *cosmo;
        let rhs = move |x: f64, _y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
            ydot[0] = 1.0 / c.e_of_z(x);
            Ok(())
        };
        let mut solver = Cvode::builder(Method::Bdf)
            .rtol(rtol)
            .atol(atol)
            .max_steps(100_000)
            .build(rhs, 0.0, SerialVector::from_slice(&[0.0]))
            .map_err(|e| format!("CVODE setup failed: {e}"))?;
        let (t, y) = solver
            .solve(z, Task::Normal)
            .map_err(|e| format!("CVODE failed integrating chi to z = {z}: {e}"))?;
        if (t - z).abs() > 1e-12 * z.max(1.0) {
            return Err(format!("CVODE returned at z = {t}, not the requested {z}"));
        }
        let v = y[0];
        out[i] = v;
        done.push((z, v));
        stats.steps += solver.num_steps();
        stats.rhs_evals += solver.num_rhs_evals();
    }
    Ok((out, stats))
}

/// `chi(z)` by adaptive Gauss–Kronrod quadrature, each `z` integrated from 0 independently.
pub fn chi_quadrature(cosmo: &FlatCosmology, zs: &[f64], rtol: f64) -> Result<Vec<f64>, String> {
    let zmax = check_redshifts(zs)?;
    cosmo.check(zmax)?;
    let f = |z: f64| 1.0 / cosmo.e_of_z(z);
    Ok(zs
        .iter()
        .map(|&z| {
            if z == 0.0 {
                0.0
            } else {
                adaptive_gauss_kronrod(&f, 0.0, z, rtol)
            }
        })
        .collect())
}

// QUADPACK qk15 abscissae and weights.
const XGK: [f64; 8] = [
    0.991_455_371_120_812_6,
    0.949_107_912_342_758_5,
    0.864_864_423_359_769_1,
    0.741_531_185_599_394_4,
    0.586_087_235_467_691_1,
    0.405_845_151_377_397_2,
    0.207_784_955_007_898_5,
    0.0,
];
const WGK: [f64; 8] = [
    0.022_935_322_010_529_22,
    0.063_092_092_629_978_55,
    0.104_790_010_322_250_2,
    0.140_653_259_715_525_9,
    0.169_004_726_639_267_9,
    0.190_350_578_064_785_4,
    0.204_432_940_075_298_9,
    0.209_482_141_084_727_8,
];
const WG: [f64; 4] = [
    0.129_484_966_168_869_7,
    0.279_705_391_489_276_7,
    0.381_830_050_505_118_9,
    0.417_959_183_673_469_4,
];

/// One G7/K15 panel: returns (Kronrod estimate, |Kronrod - Gauss|).
fn gk15(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> (f64, f64) {
    let c = 0.5 * (a + b);
    let hl = 0.5 * (b - a);
    let fc = f(c);
    let mut k = WGK[7] * fc;
    let mut g = WG[3] * fc;
    for j in 0..7 {
        let dx = hl * XGK[j];
        let s = f(c - dx) + f(c + dx);
        k += WGK[j] * s;
        if j % 2 == 1 {
            g += WG[j / 2] * s;
        }
    }
    (k * hl, ((k - g) * hl).abs())
}

/// Adaptive (recursive bisection) Gauss–Kronrod integral of `f` on `[a, b]`.
pub fn adaptive_gauss_kronrod(f: &dyn Fn(f64) -> f64, a: f64, b: f64, rtol: f64) -> f64 {
    let (whole, _) = gk15(f, a, b);
    let abs_tol = (rtol * whole.abs()).max(1e-300);
    gk_recurse(f, a, b, abs_tol, 0)
}

fn gk_recurse(f: &dyn Fn(f64) -> f64, a: f64, b: f64, tol: f64, depth: u32) -> f64 {
    let (k, err) = gk15(f, a, b);
    if err <= tol || depth >= 48 {
        return k;
    }
    let m = 0.5 * (a + b);
    gk_recurse(f, a, m, 0.5 * tol, depth + 1) + gk_recurse(f, m, b, 0.5 * tol, depth + 1)
}

// ============================================================================================
// BAO observables
// ============================================================================================

/// A BAO observable as named in DESI data files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaoQuantity {
    DmOverRd,
    DhOverRd,
    DvOverRd,
}

impl BaoQuantity {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "DM_over_rs" | "DM_over_rd" => Ok(BaoQuantity::DmOverRd),
            "DH_over_rs" | "DH_over_rd" => Ok(BaoQuantity::DhOverRd),
            "DV_over_rs" | "DV_over_rd" => Ok(BaoQuantity::DvOverRd),
            other => Err(format!("unknown BAO quantity {other:?}")),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BaoQuantity::DmOverRd => "DM_over_rs",
            BaoQuantity::DhOverRd => "DH_over_rs",
            BaoQuantity::DvOverRd => "DV_over_rs",
        }
    }
}

/// `D_M/r_d`, `D_H/r_d`, `D_V/r_d` at one redshift.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaoDistances {
    pub z: f64,
    pub dm_over_rd: f64,
    pub dh_over_rd: f64,
    pub dv_over_rd: f64,
}

impl BaoDistances {
    pub fn get(&self, q: BaoQuantity) -> f64 {
        match q {
            BaoQuantity::DmOverRd => self.dm_over_rd,
            BaoQuantity::DhOverRd => self.dh_over_rd,
            BaoQuantity::DvOverRd => self.dv_over_rd,
        }
    }
}

/// BAO distances over `r_d` given `h r_d` in Mpc. (`cosmo.h` is used only if radiation is on.)
pub fn bao_distances(
    cosmo: &FlatCosmology,
    h_rd: f64,
    zs: &[f64],
    method: Integrator,
) -> Result<Vec<BaoDistances>, String> {
    if !(h_rd.is_finite() && h_rd > 0.0) {
        return Err(format!("h_rd = {h_rd} must be positive and finite"));
    }
    let chis = chi(cosmo, zs, method)?;
    let scale = HUBBLE_DISTANCE_H1_MPC / h_rd;
    Ok(zs
        .iter()
        .zip(chis)
        .map(|(&z, c)| {
            let dm = scale * c;
            let dh = scale / cosmo.e_of_z(z);
            BaoDistances {
                z,
                dm_over_rd: dm,
                dh_over_rd: dh,
                dv_over_rd: (z * dm * dm * dh).cbrt(),
            }
        })
        .collect())
}

// ============================================================================================
// DESI-format data and chi^2
// ============================================================================================

/// One row of a DESI BAO mean file: `z value quantity`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaoPoint {
    pub z: f64,
    pub value: f64,
    pub quantity: BaoQuantity,
}

/// A Gaussian BAO likelihood: data vector in file order and the Cholesky factor of its covariance.
#[derive(Debug, Clone, PartialEq)]
pub struct BaoDataset {
    pub points: Vec<BaoPoint>,
    pub cov: Vec<Vec<f64>>,
    chol: Vec<Vec<f64>>,
}

/// Default location of the DESI DR2 Gaussian BAO files on this project's data disk. Override with
/// the directory in env var `QF_BAO_DR2_DIR`.
pub fn desi_dr2_paths() -> (PathBuf, PathBuf) {
    let dir = std::env::var("QF_BAO_DR2_DIR").unwrap_or_else(|_| {
        "/mnt/disks/disk-socrateai-local-1/dualscale-data-r3/desi_sdss_bao/desi_bao_dr2".to_string()
    });
    let dir = PathBuf::from(dir);
    (
        dir.join("desi_gaussian_bao_ALL_GCcomb_mean.txt"),
        dir.join("desi_gaussian_bao_ALL_GCcomb_cov.txt"),
    )
}

impl BaoDataset {
    /// Parse DESI-format mean and covariance text (comment lines start with `#`).
    pub fn from_strs(mean: &str, cov: &str) -> Result<Self, String> {
        let mut points = Vec::new();
        for (ln, line) in mean.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() != 3 {
                return Err(format!("mean line {}: expected 'z value quantity'", ln + 1));
            }
            let z: f64 = f[0]
                .parse()
                .map_err(|e| format!("mean line {}: bad z: {e}", ln + 1))?;
            let value: f64 = f[1]
                .parse()
                .map_err(|e| format!("mean line {}: bad value: {e}", ln + 1))?;
            points.push(BaoPoint {
                z,
                value,
                quantity: BaoQuantity::parse(f[2])?,
            });
        }
        let mut rows = Vec::new();
        for (ln, line) in cov.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let r: Result<Vec<f64>, _> = line.split_whitespace().map(str::parse).collect();
            rows.push(r.map_err(|e| format!("cov line {}: {e}", ln + 1))?);
        }
        let n = points.len();
        if n == 0 {
            return Err("mean file has no data rows".into());
        }
        if rows.len() != n || rows.iter().any(|r| r.len() != n) {
            return Err(format!(
                "covariance must be {n}x{n} to match the mean file; got {} rows",
                rows.len()
            ));
        }
        for i in 0..n {
            for j in 0..i {
                if (rows[i][j] - rows[j][i]).abs() > 1e-12 * (rows[i][i] * rows[j][j]).sqrt() {
                    return Err(format!("covariance not symmetric at ({i}, {j})"));
                }
            }
        }
        let chol = cholesky(&rows)?;
        Ok(Self {
            points,
            cov: rows,
            chol,
        })
    }

    /// Read a DESI-format mean file and covariance file.
    pub fn load(mean_path: &Path, cov_path: &Path) -> Result<Self, String> {
        let m = std::fs::read_to_string(mean_path)
            .map_err(|e| format!("reading {}: {e}", mean_path.display()))?;
        let c = std::fs::read_to_string(cov_path)
            .map_err(|e| format!("reading {}: {e}", cov_path.display()))?;
        Self::from_strs(&m, &c)
    }

    /// Load the DESI DR2 files from [`desi_dr2_paths`], or `None` if they are not present.
    pub fn desi_dr2() -> Option<Result<Self, String>> {
        let (m, c) = desi_dr2_paths();
        if !(m.exists() && c.exists()) {
            return None;
        }
        Some(Self::load(&m, &c))
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Distinct redshifts in first-appearance order.
    pub fn redshifts(&self) -> Vec<f64> {
        let mut zs: Vec<f64> = Vec::new();
        for p in &self.points {
            if !zs.contains(&p.z) {
                zs.push(p.z);
            }
        }
        zs
    }

    /// Model prediction for each data row, in file order.
    pub fn predict(
        &self,
        cosmo: &FlatCosmology,
        h_rd: f64,
        method: Integrator,
    ) -> Result<Vec<f64>, String> {
        let zs = self.redshifts();
        let d = bao_distances(cosmo, h_rd, &zs, method)?;
        Ok(self
            .points
            .iter()
            .map(|p| {
                let k = zs.iter().position(|&z| z == p.z).unwrap_or(0);
                d[k].get(p.quantity)
            })
            .collect())
    }

    /// `chi^2 = r^T C^{-1} r` for a prediction vector in file order.
    pub fn chi2_of(&self, prediction: &[f64]) -> f64 {
        let r: Vec<f64> = self
            .points
            .iter()
            .zip(prediction)
            .map(|(p, m)| p.value - m)
            .collect();
        // Solve L y = r; chi^2 = |y|^2.
        let n = r.len();
        let mut y = vec![0.0; n];
        let mut chi2 = 0.0;
        for i in 0..n {
            let mut s = r[i];
            for (k, yk) in y.iter().enumerate().take(i) {
                s -= self.chol[i][k] * yk;
            }
            y[i] = s / self.chol[i][i];
            chi2 += y[i] * y[i];
        }
        chi2
    }

    /// `chi^2` of a cosmology.
    pub fn chi2(
        &self,
        cosmo: &FlatCosmology,
        h_rd: f64,
        method: Integrator,
    ) -> Result<f64, String> {
        Ok(self.chi2_of(&self.predict(cosmo, h_rd, method)?))
    }
}

/// Lower Cholesky factor of a symmetric positive-definite matrix.
pub fn cholesky(a: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let n = a.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let s = a[i][j]
                - l[i][..j]
                    .iter()
                    .zip(&l[j][..j])
                    .map(|(x, y)| x * y)
                    .sum::<f64>();
            if i == j {
                if s <= 0.0 || !s.is_finite() {
                    return Err(format!("matrix not positive definite (pivot {i} = {s})"));
                }
                l[i][i] = s.sqrt();
            } else {
                l[i][j] = s / l[j][j];
            }
        }
    }
    Ok(l)
}

// ============================================================================================
// Flat-LCDM best fit in (Omega_m, h r_d)
// ============================================================================================

/// Result of [`fit_flat_lcdm`].
#[derive(Debug, Clone, PartialEq)]
pub struct LcdmFit {
    pub omega_m: f64,
    pub h_rd: f64,
    pub chi2: f64,
    /// Data points minus 2 fitted parameters.
    pub dof: usize,
    /// `sqrt` of the diagonal of `(H/2)^{-1}`, `H` the numerical Hessian of chi^2 at the minimum
    /// (Gaussian/Fisher approximation — not a posterior mean/std).
    pub sigma_omega_m: f64,
    pub sigma_h_rd: f64,
    pub corr: f64,
    /// Best point of the coarse 2-D grid that seeded Nelder–Mead.
    pub grid_best: (f64, f64, f64),
    /// chi^2 evaluations used by Nelder–Mead.
    pub nm_evaluations: usize,
}

/// Minimise chi^2 over `(Omega_m, h r_d)` in flat LCDM without radiation: a 2-D grid
/// (`Omega_m` in [0.10, 0.60] step 0.005, `h r_d` in [80, 125] Mpc step 0.25) seeds a Nelder–Mead
/// simplex; the Hessian at the minimum gives Gaussian errors.
pub fn fit_flat_lcdm(data: &BaoDataset, method: Integrator) -> Result<LcdmFit, String> {
    // Grid. D/r_d is linear in 1/h_rd, so chi(z) is computed once per Omega_m row.
    let zs = data.redshifts();
    let mut best = (f64::NAN, f64::NAN, f64::INFINITY);
    for i in 0..=100 {
        let om = 0.10 + 0.005 * i as f64;
        let cosmo = FlatCosmology::lcdm(om, 0.7);
        let unit = bao_distances(&cosmo, 1.0, &zs, method)?;
        for j in 0..=180 {
            let hrd = 80.0 + 0.25 * j as f64;
            let pred: Vec<f64> = data
                .points
                .iter()
                .map(|p| {
                    let k = zs.iter().position(|&z| z == p.z).unwrap_or(0);
                    let v = unit[k].get(p.quantity);
                    // D_M, D_H scale as 1/h_rd; D_V too (cube root of three such factors).
                    v / hrd
                })
                .collect();
            let c2 = data.chi2_of(&pred);
            if c2 < best.2 {
                best = (om, hrd, c2);
            }
        }
    }
    let f = |x: &[f64; 2]| -> f64 {
        if !(0.01..=0.99).contains(&x[0]) || x[1] <= 1.0 {
            return f64::INFINITY;
        }
        data.chi2(&FlatCosmology::lcdm(x[0], 0.7), x[1], method)
            .unwrap_or(f64::INFINITY)
    };
    let (x, fx, evals) = nelder_mead_2d(&f, [best.0, best.1], [0.01, 0.5], 1e-10, 1000);
    if !fx.is_finite() {
        return Err("Nelder-Mead did not find a finite chi^2".into());
    }
    // Numerical Hessian (central differences).
    let hs = [2e-3, 0.2];
    let mut hess = [[0.0; 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            let shift = |da: f64, db: f64| {
                let mut p = x;
                p[a] += da * hs[a];
                p[b] += db * hs[b];
                f(&p)
            };
            hess[a][b] = if a == b {
                (shift(1.0, 0.0) + shift(-1.0, 0.0) - 2.0 * fx) / (hs[a] * hs[a])
            } else {
                (shift(1.0, 1.0) - shift(1.0, -1.0) - shift(-1.0, 1.0) + shift(-1.0, -1.0))
                    / (4.0 * hs[a] * hs[b])
            };
        }
    }
    // Fisher F = H/2, covariance = F^{-1}.
    let f00 = 0.5 * hess[0][0];
    let f11 = 0.5 * hess[1][1];
    let f01 = 0.5 * 0.5 * (hess[0][1] + hess[1][0]);
    let det = f00 * f11 - f01 * f01;
    if det <= 0.0 {
        return Err(format!(
            "chi^2 Hessian at the minimum is not positive definite (det {det})"
        ));
    }
    let c00 = f11 / det;
    let c11 = f00 / det;
    let c01 = -f01 / det;
    Ok(LcdmFit {
        omega_m: x[0],
        h_rd: x[1],
        chi2: fx,
        dof: data.len().saturating_sub(2),
        sigma_omega_m: c00.sqrt(),
        sigma_h_rd: c11.sqrt(),
        corr: c01 / (c00 * c11).sqrt(),
        grid_best: best,
        nm_evaluations: evals,
    })
}

/// Nelder–Mead on two parameters. Returns (argmin, min, function evaluations).
pub fn nelder_mead_2d(
    f: &dyn Fn(&[f64; 2]) -> f64,
    x0: [f64; 2],
    step: [f64; 2],
    ftol: f64,
    max_iter: usize,
) -> ([f64; 2], f64, usize) {
    let mut s = [x0, [x0[0] + step[0], x0[1]], [x0[0], x0[1] + step[1]]];
    let mut fs = [f(&s[0]), f(&s[1]), f(&s[2])];
    let mut evals = 3;
    let lerp =
        |a: &[f64; 2], b: &[f64; 2], t: f64| [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
    for _ in 0..max_iter {
        // Sort ascending.
        let mut idx = [0, 1, 2];
        idx.sort_by(|&a, &b| fs[a].total_cmp(&fs[b]));
        s = [s[idx[0]], s[idx[1]], s[idx[2]]];
        fs = [fs[idx[0]], fs[idx[1]], fs[idx[2]]];
        let size = (0..2)
            .map(|k| ((s[1][k] - s[0][k]).abs().max((s[2][k] - s[0][k]).abs())) / step[k])
            .fold(0.0_f64, f64::max);
        if (fs[2] - fs[0]).abs() <= ftol * (1.0 + fs[0].abs()) && size < 1e-5 {
            break;
        }
        let centroid = lerp(&s[0], &s[1], 0.5);
        let xr = lerp(&centroid, &s[2], -1.0);
        let fr = f(&xr);
        evals += 1;
        if fr < fs[0] {
            let xe = lerp(&centroid, &s[2], -2.0);
            let fe = f(&xe);
            evals += 1;
            if fe < fr {
                s[2] = xe;
                fs[2] = fe;
            } else {
                s[2] = xr;
                fs[2] = fr;
            }
        } else if fr < fs[1] {
            s[2] = xr;
            fs[2] = fr;
        } else {
            let (xc, fc) = if fr < fs[2] {
                let xc = lerp(&centroid, &xr, 0.5);
                (xc, f(&xc))
            } else {
                let xc = lerp(&centroid, &s[2], 0.5);
                (xc, f(&xc))
            };
            evals += 1;
            if fc < fs[2].min(fr) {
                s[2] = xc;
                fs[2] = fc;
            } else {
                for k in 1..3 {
                    s[k] = lerp(&s[0], &s[k], 0.5);
                    fs[k] = f(&s[k]);
                    evals += 1;
                }
            }
        }
    }
    let mut ib = 0;
    for k in 1..3 {
        if fs[k] < fs[ib] {
            ib = k;
        }
    }
    (s[ib], fs[ib], evals)
}
