//! ITER current-quench, 0-D coupled plasma/vessel circuit model.
//!
//! This example replaces the prescribed-trajectory "disruption" of
//! `iter_disruption.rs` (whose right-hand side never reads the state, see
//! `docs/audit/fusion-2026-09-27/B_numerics.md`) with the smallest model that
//! is physically standard and genuinely state dependent:
//!
//! ```text
//!   L_p dI_p/dt + M   dI_v/dt = -R_p(T_e) I_p        (plasma loop)
//!   M   dI_p/dt + L_v dI_v/dt = -R_v      I_v        (vessel loop)
//! ```
//!
//! * `R_p(T_e)` is the Spitzer parallel resistance of the post-thermal-quench
//!   plasma column (NRL Plasma Formulary, "Transport" section:
//!   eta_perp = 1.03e-2 Z lnLambda T_e^{-3/2} Ohm cm, eta_par = 0.51 eta_perp).
//! * `L_p = mu0 R0 (ln(8 R0 / (a sqrt(kappa))) - 2 + l_i/2)` is the standard
//!   large-aspect-ratio inductance of an elongated current ring.
//! * The Coulomb logarithm follows the NRL electron-ion formula.
//! * T_e is either a fixed parameter (5-20 eV scan) or the quasi-static
//!   Ohmic-radiative balance temperature R_p(T_e) I_p^2 = n_e n_imp L_z V.
//! * The vessel parameters (L_v, R_v, M) are order-of-magnitude ASSUMPTIONS,
//!   exposed as parameters. They are not ITER design values.
//!
//! In addition to the two currents the state carries the integrated Ohmic
//! dissipation in each loop, so the energy balance
//! `W_mag(0) - W_mag(t) = Q_p(t) + Q_v(t)` can be checked to solver accuracy.
//!
//! Outputs: I_p(t), I_v(t), the ITER-convention 80%-20% quench time
//! t_CQ = (t20 - t80)/0.6, the energy residual, CSV trajectories and a JSON
//! summary. The controls at the bottom of this file run under
//! `cargo test -p examples --example iter_current_quench_0d`.
//!
//! Limitations (also in `docs/audit/fusion-2026-09-27/CURRENT_QUENCH_0D.md`):
//! 0-D only, no halo currents, no runaway electrons, no MHD, no vertical
//! motion, single-loop vessel, no impurity transport.

use cvode::{Cvode, CvodeError, Method, Task};
use nvector::SerialVector;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
use sundials_core::generated::sundials_dense::DenseMat;

/// Vacuum permeability [H/m] (CODATA 2018).
pub const MU0: f64 = 1.256_637_062_12e-6;
/// Elementary charge [C].
pub const E_CHARGE: f64 = 1.602_176_634e-19;
/// NRL Spitzer coefficient for eta_par in SI: 0.51 * 1.03e-4 Ohm m eV^{3/2}.
pub const SPITZER_PAR_COEFF_SI: f64 = 0.51 * 1.03e-4;

/// How the post-thermal-quench electron temperature is determined.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TeMode {
    /// T_e held fixed at `Params::te_ev` for the whole quench.
    Fixed,
    /// Quasi-static Ohmic-radiative balance R_p(T_e) I_p^2 = n_e n_imp L_z V,
    /// floored at `Params::te_floor_ev`. Justified because the thermal energy
    /// of a few-eV plasma (~1e5 J) relaxes in ~1e-5..1e-4 s, far below t_CQ.
    OhmicRadiative,
}

/// Model parameters. Every value is exposed so the controls can perturb it.
#[derive(Clone, Debug)]
pub struct Params {
    /// Major radius [m].
    pub r0: f64,
    /// Minor radius [m].
    pub a: f64,
    /// Elongation.
    pub kappa: f64,
    /// Toroidal field on axis [T] (reported only; the circuit model does not use it).
    pub b0: f64,
    /// Pre-quench plasma current [A].
    pub ip0: f64,
    /// Normalised internal inductance l_i.
    pub li: f64,
    /// Electron density [m^-3].
    pub ne: f64,
    /// Effective charge.
    pub zeff: f64,
    /// Fixed post-TQ electron temperature [eV] (TeMode::Fixed).
    pub te_ev: f64,
    /// Temperature model.
    pub te_mode: TeMode,
    /// Impurity density as a fraction of n_e (TeMode::OhmicRadiative).
    pub n_imp_frac: f64,
    /// Radiative cooling coefficient L_z [W m^3] (TeMode::OhmicRadiative).
    pub lz: f64,
    /// Temperature floor [eV] (TeMode::OhmicRadiative).
    pub te_floor_ev: f64,
    /// Vessel self-inductance [H] (assumption).
    pub lv: f64,
    /// Plasma-vessel mutual inductance [H] (assumption).
    pub m: f64,
    /// Vessel one-turn resistance [Ohm] (assumption).
    pub rv: f64,
    /// Multiplier on the plasma resistance (1.0 = Spitzer). Used by the controls.
    pub eta_multiplier: f64,
}

impl Params {
    /// ITER reference values. R0, a, kappa, B0, Ip from the ITER Physics Basis
    /// (Nucl. Fusion 39 (1999) 2137) / Progress in the ITER Physics Basis
    /// (Nucl. Fusion 47 (2007)). Post-TQ plasma (T_e, n_e, Z_eff) and the
    /// vessel numbers are assumptions documented in CURRENT_QUENCH_0D.md.
    pub fn iter_default() -> Self {
        Self {
            r0: 6.2,
            a: 2.0,
            kappa: 1.7,
            b0: 5.3,
            ip0: 15.0e6,
            li: 0.8,
            ne: 1.0e20,
            zeff: 2.0,
            te_ev: 10.0,
            te_mode: TeMode::Fixed,
            n_imp_frac: 0.02,
            lz: 1.0e-31,
            te_floor_ev: 1.0,
            lv: 5.0e-6,
            m: 4.0e-6,
            rv: 8.0e-6,
            eta_multiplier: 1.0,
        }
    }

    /// Plasma volume of an elliptical-cross-section torus [m^3].
    pub fn volume(&self) -> f64 {
        2.0 * std::f64::consts::PI.powi(2) * self.r0 * self.a.powi(2) * self.kappa
    }

    /// Cross-section area [m^2].
    pub fn area(&self) -> f64 {
        std::f64::consts::PI * self.a.powi(2) * self.kappa
    }

    /// L_p = mu0 R0 (ln(8 R0/(a sqrt(kappa))) - 2 + l_i/2)  [H].
    pub fn plasma_inductance(&self) -> f64 {
        MU0 * self.r0 * ((8.0 * self.r0 / (self.a * self.kappa.sqrt())).ln() - 2.0 + 0.5 * self.li)
    }

    /// Geometric resistance factor: R_p = eta * 2 pi R0 / (pi a^2 kappa) = eta * geom.
    pub fn resistance_geometry(&self) -> f64 {
        2.0 * self.r0 / (self.a.powi(2) * self.kappa)
    }

    /// Plasma resistance at electron temperature `te_ev` [Ohm].
    pub fn plasma_resistance(&self, te_ev: f64) -> f64 {
        let lnl = coulomb_log_ei(self.ne, te_ev, self.zeff);
        self.eta_multiplier
            * spitzer_eta_parallel(te_ev, self.zeff, lnl)
            * self.resistance_geometry()
    }

    /// Radiated power in the Ohmic-radiative mode [W]: P_rad = n_e n_imp L_z V.
    pub fn radiated_power(&self) -> f64 {
        self.ne * (self.n_imp_frac * self.ne) * self.lz * self.volume()
    }

    /// Electron temperature used by the RHS for plasma current `ip` [eV].
    pub fn electron_temperature(&self, ip: f64) -> f64 {
        match self.te_mode {
            TeMode::Fixed => self.te_ev,
            TeMode::OhmicRadiative => self.te_ohmic_radiative(ip),
        }
    }

    /// Solve R_p(T) I^2 = P_rad for T with a fixed-point iteration on lnLambda:
    /// T = (eta_mult * C Z lnL(T) geom I^2 / P_rad)^{2/3}, floored at te_floor.
    pub fn te_ohmic_radiative(&self, ip: f64) -> f64 {
        let p_rad = self.radiated_power();
        let geom = self.resistance_geometry();
        let mut te = self.te_ev.max(self.te_floor_ev);
        for _ in 0..8 {
            let lnl = coulomb_log_ei(self.ne, te, self.zeff);
            let num = self.eta_multiplier * SPITZER_PAR_COEFF_SI * self.zeff * lnl * geom * ip * ip;
            let te_new = (num / p_rad).powf(2.0 / 3.0);
            te = te_new.max(self.te_floor_ev);
        }
        te
    }

    /// Coupling coefficient k = M / sqrt(L_p L_v); must be < 1 for a passive system.
    pub fn coupling_coefficient(&self) -> f64 {
        self.m / (self.plasma_inductance() * self.lv).sqrt()
    }
}

/// NRL Plasma Formulary electron-ion Coulomb logarithm (n_e in cm^-3, T_e in eV):
///   T_e < 10 Z^2 eV : 23 - ln(n_e^{1/2} Z T_e^{-3/2})
///   T_e > 10 Z^2 eV : 24 - ln(n_e^{1/2} T_e^{-1})
/// Clamped below at 2 (the formula is meaningless once lnLambda ~ 1).
pub fn coulomb_log_ei(ne_m3: f64, te_ev: f64, z: f64) -> f64 {
    let ne_cm3 = ne_m3 * 1.0e-6;
    let lnl = if te_ev < 10.0 * z * z {
        23.0 - (ne_cm3.sqrt() * z * te_ev.powf(-1.5)).ln()
    } else {
        24.0 - (ne_cm3.sqrt() / te_ev).ln()
    };
    lnl.max(2.0)
}

/// Spitzer parallel resistivity [Ohm m] with linear Z_eff scaling and the
/// Z = 1 parallel factor 0.51 (NRL form). The exact Spitzer-Haerm factor is
/// 0.44 at Z = 2 and 0.38 at Z = 4, so this overestimates eta by 10-25 % at
/// Z_eff > 1. Neoclassical corrections are neglected (collisional plasma).
pub fn spitzer_eta_parallel(te_ev: f64, zeff: f64, ln_lambda: f64) -> f64 {
    SPITZER_PAR_COEFF_SI * zeff * ln_lambda * te_ev.powf(-1.5)
}

/// Integration options.
#[derive(Clone, Debug)]
pub struct SolveOpts {
    pub rtol: f64,
    pub atol: f64,
    /// Output spacing [s].
    pub dt_out: f64,
    /// Hard stop [s].
    pub t_max: f64,
    /// Stop once I_p < stop_frac * I_p0.
    pub stop_frac: f64,
}

impl Default for SolveOpts {
    fn default() -> Self {
        Self {
            rtol: 1.0e-6,
            atol: 1.0e-3,
            dt_out: 2.0e-4,
            t_max: 2.0,
            stop_frac: 0.01,
        }
    }
}

/// Solution sampled on the output grid.
#[derive(Clone, Debug, Default)]
pub struct Trajectory {
    pub t: Vec<f64>,
    pub ip: Vec<f64>,
    pub iv: Vec<f64>,
    pub qp: Vec<f64>,
    pub qv: Vec<f64>,
    pub te: Vec<f64>,
    pub rp: Vec<f64>,
    pub wmag: Vec<f64>,
    pub nst: usize,
    pub nfe: usize,
    pub lp: f64,
}

/// Magnetic energy of the two coupled loops [J].
pub fn magnetic_energy(p: &Params, lp: f64, ip: f64, iv: f64) -> f64 {
    0.5 * lp * ip * ip + p.m * ip * iv + 0.5 * p.lv * iv * iv
}

/// Right-hand side of the 4-state system y = [I_p, I_v, Q_p, Q_v].
pub fn rhs(p: &Params, lp: f64, y: &[f64], ydot: &mut [f64]) -> Result<(), String> {
    let ip = y[0];
    let iv = y[1];
    let te = p.electron_temperature(ip);
    let rp = p.plasma_resistance(te);
    let ep = -rp * ip;
    let ev = -p.rv * iv;
    let det = lp * p.lv - p.m * p.m;
    if det <= 0.0 {
        return Err(format!("L_p L_v - M^2 = {det} <= 0: non-passive coupling"));
    }
    ydot[0] = (p.lv * ep - p.m * ev) / det;
    ydot[1] = (lp * ev - p.m * ep) / det;
    ydot[2] = rp * ip * ip;
    ydot[3] = p.rv * iv * iv;
    Ok(())
}

/// Analytic Jacobian, column-major (`cols[j][i] = df_i/dy_j`). The only
/// nonlinearity is R_p(T_e(I_p)) in the Ohmic-radiative mode; its derivative
/// is taken by a central difference of the scalar function R_p(I_p).
pub fn jacobian(p: &Params, lp: f64, y: &[f64], j: &mut DenseMat) {
    let ip = y[0];
    let iv = y[1];
    let rp = p.plasma_resistance(p.electron_temperature(ip));
    let drp_dip = match p.te_mode {
        TeMode::Fixed => 0.0,
        TeMode::OhmicRadiative => {
            let h = 1.0e-6 * ip.abs().max(1.0);
            let rp_plus = p.plasma_resistance(p.electron_temperature(ip + h));
            let rp_minus = p.plasma_resistance(p.electron_temperature(ip - h));
            (rp_plus - rp_minus) / (2.0 * h)
        }
    };
    let gprime = rp + ip * drp_dip; // d(R_p I_p)/dI_p
    let det = lp * p.lv - p.m * p.m;
    for col in j.cols.iter_mut() {
        for v in col.iter_mut() {
            *v = 0.0;
        }
    }
    j.cols[0][0] = -p.lv * gprime / det;
    j.cols[1][0] = p.m * p.rv / det;
    j.cols[0][1] = p.m * gprime / det;
    j.cols[1][1] = -lp * p.rv / det;
    j.cols[0][2] = 2.0 * rp * ip + ip * ip * drp_dip;
    j.cols[1][3] = 2.0 * p.rv * iv;
}

/// Integrate the circuit model with CVODE (BDF) and sample it on the output grid.
pub fn simulate(p: &Params, o: &SolveOpts) -> Result<Trajectory, CvodeError> {
    let lp = p.plasma_inductance();
    if p.coupling_coefficient() >= 1.0 {
        return Err(CvodeError::Config(format!(
            "coupling coefficient k = {} >= 1 (L_p={lp:e}, L_v={:e}, M={:e})",
            p.coupling_coefficient(),
            p.lv,
            p.m
        )));
    }
    let pc = p.clone();
    let f =
        move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> { rhs(&pc, lp, y, ydot) };
    let pj = p.clone();
    let jac = move |_t: f64, y: &[f64], j: &mut DenseMat| -> Result<(), String> {
        jacobian(&pj, lp, y, j);
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[p.ip0, 0.0, 0.0, 0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(o.rtol)
        .atol(o.atol)
        .max_steps(5_000_000)
        .jacobian(jac)
        .build(f, 0.0, y0)?;

    let mut tr = Trajectory {
        lp,
        ..Default::default()
    };
    let push = |tr: &mut Trajectory, t: f64, y: &[f64]| {
        let te = p.electron_temperature(y[0]);
        tr.t.push(t);
        tr.ip.push(y[0]);
        tr.iv.push(y[1]);
        tr.qp.push(y[2]);
        tr.qv.push(y[3]);
        tr.te.push(te);
        tr.rp.push(p.plasma_resistance(te));
        tr.wmag.push(magnetic_energy(p, lp, y[0], y[1]));
    };
    push(&mut tr, 0.0, &[p.ip0, 0.0, 0.0, 0.0]);

    let n_out = (o.t_max / o.dt_out).round() as usize;
    for k in 1..=n_out {
        let tout = k as f64 * o.dt_out;
        let (t, y) = solver.solve(tout, Task::Normal)?;
        let y_copy: Vec<f64> = y.to_vec();
        push(&mut tr, t, &y_copy);
        if y_copy[0] < o.stop_frac * p.ip0 {
            break;
        }
    }
    tr.nst = solver.num_steps();
    tr.nfe = solver.num_rhs_evals();
    Ok(tr)
}

/// First downward crossing of `level` by `ip`, linearly interpolated in time.
fn crossing_time(t: &[f64], ip: &[f64], level: f64) -> Option<f64> {
    for i in 1..t.len() {
        if ip[i - 1] >= level && ip[i] < level {
            let f = (ip[i - 1] - level) / (ip[i - 1] - ip[i]);
            return Some(t[i - 1] + f * (t[i] - t[i - 1]));
        }
    }
    None
}

/// ITER-convention quench time: (t20 - t80) / 0.6, extrapolating the 80-20 %
/// interval to a 100-0 % linear decay. Returns (t80, t20, t_cq).
pub fn quench_time(tr: &Trajectory, ip0: f64) -> Option<(f64, f64, f64)> {
    let t80 = crossing_time(&tr.t, &tr.ip, 0.8 * ip0)?;
    let t20 = crossing_time(&tr.t, &tr.ip, 0.2 * ip0)?;
    Some((t80, t20, (t20 - t80) / 0.6))
}

/// Least-squares e-folding time of I_p over the 80 %-20 % window.
pub fn fit_decay_time(tr: &Trajectory, ip0: f64) -> Option<f64> {
    let (n, sx, sy, sxx, sxy) =
        tr.t.iter()
            .zip(&tr.ip)
            .filter(|&(_, &i)| i <= 0.8 * ip0 && i >= 0.2 * ip0)
            .fold(
                (0.0_f64, 0.0, 0.0, 0.0, 0.0),
                |(n, sx, sy, sxx, sxy), (&t, &i)| {
                    let y = i.ln();
                    (n + 1.0, sx + t, sy + y, sxx + t * t, sxy + t * y)
                },
            );
    if n < 3.0 {
        return None;
    }
    let slope = (n * sxy - sx * sy) / (n * sxx - sx * sx);
    if slope >= 0.0 {
        None
    } else {
        Some(-1.0 / slope)
    }
}

/// Max over the grid of |W(0) - W(t) - Q_p(t) - Q_v(t)| / W(0), with Q from the
/// ODE quadrature states (same solve as the currents).
pub fn energy_residual(tr: &Trajectory) -> f64 {
    let w0 = tr.wmag[0];
    tr.t.iter()
        .enumerate()
        .map(|(k, _)| ((w0 - tr.wmag[k]) - tr.qp[k] - tr.qv[k]).abs() / w0)
        .fold(0.0, f64::max)
}

/// Independent check: trapezoid integration of R_p I_p^2 + R_v I_v^2 on the output grid.
pub fn energy_residual_trapezoid(p: &Params, tr: &Trajectory) -> f64 {
    let w0 = tr.wmag[0];
    let mut q = 0.0;
    let mut worst: f64 = 0.0;
    for k in 1..tr.t.len() {
        let f0 = tr.rp[k - 1] * tr.ip[k - 1].powi(2) + p.rv * tr.iv[k - 1].powi(2);
        let f1 = tr.rp[k] * tr.ip[k].powi(2) + p.rv * tr.iv[k].powi(2);
        q += 0.5 * (f0 + f1) * (tr.t[k] - tr.t[k - 1]);
        worst = worst.max(((w0 - tr.wmag[k]) - q).abs() / w0);
    }
    worst
}

/// Max relative drift of the vessel flux M I_p + L_v I_v (conserved when R_v = 0).
pub fn vessel_flux_drift(p: &Params, tr: &Trajectory) -> f64 {
    let psi0 = p.m * tr.ip[0] + p.lv * tr.iv[0];
    tr.ip
        .iter()
        .zip(&tr.iv)
        .map(|(&ip, &iv)| ((p.m * ip + p.lv * iv) - psi0).abs() / psi0.abs())
        .fold(0.0, f64::max)
}

/// Peak vessel current and its time.
pub fn peak_vessel_current(tr: &Trajectory) -> (f64, f64) {
    let mut best: (f64, f64) = (0.0, 0.0);
    for (k, &iv) in tr.iv.iter().enumerate() {
        if iv.abs() > best.0.abs() {
            best = (iv, tr.t[k]);
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Output helpers (no serde in this crate; JSON is written by hand).
// ---------------------------------------------------------------------------

fn write_csv(path: &Path, tr: &Trajectory) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    writeln!(f, "t_s,Ip_A,Iv_A,Qp_J,Qv_J,Te_eV,Rp_Ohm,Wmag_J")?;
    for k in 0..tr.t.len() {
        writeln!(
            f,
            "{:.7e},{:.9e},{:.9e},{:.9e},{:.9e},{:.6e},{:.6e},{:.9e}",
            tr.t[k], tr.ip[k], tr.iv[k], tr.qp[k], tr.qv[k], tr.te[k], tr.rp[k], tr.wmag[k]
        )?;
    }
    Ok(())
}

fn json_num(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.9e}")
    } else {
        "null".to_string()
    }
}

fn json_opt(v: Option<f64>) -> String {
    match v {
        Some(x) => json_num(x),
        None => "null".to_string(),
    }
}

struct RunSummary {
    label: String,
    te_mode: String,
    te_ev: f64,
    zeff: f64,
    eta_multiplier: f64,
    rv: f64,
    m: f64,
    rtol: f64,
    lp: f64,
    rp0: f64,
    tau_lr: f64,
    t80: Option<f64>,
    t20: Option<f64>,
    t_cq: Option<f64>,
    tau_fit: Option<f64>,
    iv_peak: f64,
    t_iv_peak: f64,
    t_end: f64,
    ip_end: f64,
    energy_residual: f64,
    energy_residual_trapz: f64,
    flux_drift: f64,
    nst: usize,
    nfe: usize,
    csv: String,
}

impl RunSummary {
    fn to_json(&self) -> String {
        format!(
            "{{\"label\":\"{}\",\"te_mode\":\"{}\",\"te_eV\":{},\"zeff\":{},\"eta_multiplier\":{},\"Rv_Ohm\":{},\"M_H\":{},\"rtol\":{},\"Lp_H\":{},\"Rp0_Ohm\":{},\"tau_LR_s\":{},\"t80_s\":{},\"t20_s\":{},\"t_CQ_s\":{},\"tau_fit_s\":{},\"Iv_peak_A\":{},\"t_Iv_peak_s\":{},\"t_end_s\":{},\"Ip_end_A\":{},\"energy_residual_rel\":{},\"energy_residual_trapz_rel\":{},\"vessel_flux_drift_rel\":{},\"cvode_steps\":{},\"cvode_rhs_evals\":{},\"csv\":\"{}\"}}",
            self.label,
            self.te_mode,
            json_num(self.te_ev),
            json_num(self.zeff),
            json_num(self.eta_multiplier),
            json_num(self.rv),
            json_num(self.m),
            json_num(self.rtol),
            json_num(self.lp),
            json_num(self.rp0),
            json_num(self.tau_lr),
            json_opt(self.t80),
            json_opt(self.t20),
            json_opt(self.t_cq),
            json_opt(self.tau_fit),
            json_num(self.iv_peak),
            json_num(self.t_iv_peak),
            json_num(self.t_end),
            json_num(self.ip_end),
            json_num(self.energy_residual),
            json_num(self.energy_residual_trapz),
            json_num(self.flux_drift),
            self.nst,
            self.nfe,
            self.csv
        )
    }
}

fn run_and_record(
    label: &str,
    p: &Params,
    o: &SolveOpts,
    out_dir: &Path,
) -> Result<(RunSummary, Trajectory), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let tr = simulate(p, o)?;
    let elapsed = t0.elapsed();
    let csv_path = out_dir.join(format!("{label}.csv"));
    write_csv(&csv_path, &tr)?;
    let rp0 = p.plasma_resistance(p.electron_temperature(p.ip0));
    let (iv_peak, t_iv_peak) = peak_vessel_current(&tr);
    let q = quench_time(&tr, p.ip0);
    let s = RunSummary {
        label: label.to_string(),
        te_mode: format!("{:?}", p.te_mode),
        te_ev: p.te_ev,
        zeff: p.zeff,
        eta_multiplier: p.eta_multiplier,
        rv: p.rv,
        m: p.m,
        rtol: o.rtol,
        lp: tr.lp,
        rp0,
        tau_lr: tr.lp / rp0,
        t80: q.map(|x| x.0),
        t20: q.map(|x| x.1),
        t_cq: q.map(|x| x.2),
        tau_fit: fit_decay_time(&tr, p.ip0),
        iv_peak,
        t_iv_peak,
        t_end: *tr.t.last().unwrap(),
        ip_end: *tr.ip.last().unwrap(),
        energy_residual: energy_residual(&tr),
        energy_residual_trapz: energy_residual_trapezoid(p, &tr),
        flux_drift: vessel_flux_drift(p, &tr),
        nst: tr.nst,
        nfe: tr.nfe,
        csv: csv_path.display().to_string(),
    };
    println!(
        "  {label:<28} Te={:>6.2} eV Zeff={:.1} Rp0={:.3e} Ohm  tau_LR={:.4} s  t_CQ={}  Iv_peak={:.3} MA @ {:.4} s  dE={:.2e}  nst={} ({:.2?})",
        p.electron_temperature(p.ip0),
        p.zeff,
        rp0,
        tr.lp / rp0,
        s.t_cq
            .map(|x| format!("{:.4} s", x))
            .unwrap_or_else(|| "n/a".into()),
        iv_peak * 1e-6,
        t_iv_peak,
        s.energy_residual,
        tr.nst,
        elapsed
    );
    Ok((s, tr))
}

fn resolve_out_dir() -> PathBuf {
    if let Ok(d) = std::env::var("CQ0D_OUT_DIR") {
        return PathBuf::from(d);
    }
    let disk2 = PathBuf::from(
        "/mnt/disks/disk-socrateai-local-1/AutoevolveAI/fusion_audit_2026-09-27/current_quench",
    );
    if fs::create_dir_all(&disk2).is_ok() {
        return disk2;
    }
    let fallback =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fusion/current_quench_0d/bulk");
    eprintln!(
        "warning: {} not writable, using {}",
        disk2.display(),
        fallback.display()
    );
    fallback
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!(" ITER current quench: 0-D coupled plasma/vessel circuit (CVODE BDF)");
    println!("============================================================");

    let out_dir = resolve_out_dir();
    fs::create_dir_all(&out_dir)?;
    let summary_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fusion/current_quench_0d");
    fs::create_dir_all(&summary_dir)?;

    let base = Params::iter_default();
    let opts = SolveOpts::default();
    let lp = base.plasma_inductance();
    println!(
        "Geometry: R0={} m a={} m kappa={} B0={} T Ip0={} MA  V={:.1} m^3",
        base.r0,
        base.a,
        base.kappa,
        base.b0,
        base.ip0 * 1e-6,
        base.volume()
    );
    println!(
        "L_p = {:.4e} H (l_i={}), L_v = {:.2e} H, M = {:.2e} H, R_v = {:.2e} Ohm, k = {:.3}, tau_v = L_v/R_v = {:.3} s",
        lp,
        base.li,
        base.lv,
        base.m,
        base.rv,
        base.coupling_coefficient(),
        base.lv / base.rv
    );
    println!(
        "Spitzer check: eta_par(25 keV, Zeff=1, lnL=17) = {:.3e} Ohm m (NRL reference 2.26e-10)",
        spitzer_eta_parallel(25.0e3, 1.0, 17.0)
    );
    println!(
        "W_mag(0) = {:.4e} J",
        magnetic_energy(&base, lp, base.ip0, 0.0)
    );
    println!(
        "Solver: BDF rtol={:e} atol={:e} dt_out={:e} s",
        opts.rtol, opts.atol, opts.dt_out
    );
    println!("Output dir: {}", out_dir.display());
    println!();

    let mut runs: Vec<RunSummary> = Vec::new();

    println!(
        "[1] Reference run (fixed T_e = {} eV, Z_eff = {})",
        base.te_ev, base.zeff
    );
    let (s, _) = run_and_record("reference_Te10_Zeff2", &base, &opts, &out_dir)?;
    runs.push(s);

    println!("[2] T_e scan 5-20 eV (fixed), Z_eff in {{1, 2, 3}}");
    let te_scan = [5.0, 7.5, 10.0, 12.5, 15.0, 20.0];
    for &z in &[1.0, 2.0, 3.0] {
        for &te in &te_scan {
            let mut p = base.clone();
            p.te_ev = te;
            p.zeff = z;
            let label = format!("scan_Te{:.1}_Zeff{:.0}", te, z);
            let (s, _) = run_and_record(&label, &p, &opts, &out_dir)?;
            runs.push(s);
        }
    }

    println!(
        "[3] Ohmic-radiative T_e (n_imp/n_e = {}, L_z = {:e} W m^3, P_rad = {:.3e} W)",
        base.n_imp_frac,
        base.lz,
        base.radiated_power()
    );
    let mut p_or = base.clone();
    p_or.te_mode = TeMode::OhmicRadiative;
    let (s, _) = run_and_record("ohmic_radiative_Zeff2", &p_or, &opts, &out_dir)?;
    runs.push(s);

    println!("[4] Controls (same code paths as the #[test]s)");
    let mut p_m0 = base.clone();
    p_m0.m = 0.0;
    let (s_m0, tr_m0) = run_and_record("ctrl1_uncoupled_M0", &p_m0, &opts, &out_dir)?;
    let tau = tr_m0.lp / p_m0.plasma_resistance(p_m0.te_ev);
    let mut worst = 0.0_f64;
    for (k, &t) in tr_m0.t.iter().enumerate() {
        if t <= 3.0 * tau {
            let exact = p_m0.ip0 * (-t / tau).exp();
            worst = worst.max((tr_m0.ip[k] - exact).abs() / exact);
        }
    }
    println!(
        "      ctrl1 analytic exp(-t R/L): max rel err (t<=3 tau) = {:.3e}, t_CQ analytic = {:.5} s vs solver {:.5} s",
        worst,
        tau * 4.0_f64.ln() / 0.6,
        s_m0.t_cq.unwrap_or(f64::NAN)
    );
    runs.push(s_m0);

    let mut p_2r = p_m0.clone();
    p_2r.eta_multiplier = 2.0;
    let (s_2r, _) = run_and_record("ctrl3_uncoupled_2Rp", &p_2r, &opts, &out_dir)?;
    println!(
        "      ctrl3 (M=0) tau_fit ratio 2R/1R = {:.6}",
        s_2r.tau_fit.unwrap_or(f64::NAN) / runs.last().unwrap().tau_fit.unwrap_or(f64::NAN)
    );
    let mut p_2rc = base.clone();
    p_2rc.eta_multiplier = 2.0;
    let (s_2rc, _) = run_and_record("ctrl3_coupled_2Rp", &p_2rc, &opts, &out_dir)?;
    println!(
        "      ctrl3 (coupled) t_CQ ratio 2R/1R = {:.6}",
        s_2rc.t_cq.unwrap_or(f64::NAN) / runs[0].t_cq.unwrap_or(f64::NAN)
    );
    runs.push(s_2r);
    runs.push(s_2rc);

    let mut p_iw = base.clone();
    p_iw.rv = 0.0;
    let (s_iw, _) = run_and_record("ctrl4_ideal_wall_Rv0", &p_iw, &opts, &out_dir)?;
    println!(
        "      ctrl4 vessel flux M Ip + Lv Iv drift = {:.3e}; Iv at end = {:.4} MA vs (M/Lv)(Ip0 - Ip_end) = {:.4} MA",
        s_iw.flux_drift,
        s_iw.iv_peak * 1e-6,
        p_iw.m / p_iw.lv * (p_iw.ip0 - s_iw.ip_end) * 1e-6
    );
    runs.push(s_iw);

    let mut p_pert = base.clone();
    p_pert.ip0 = base.ip0 + 1.0e6;
    let (_, tr_pert) = run_and_record("ctrl6_perturbed_Ip0_plus1MA", &p_pert, &opts, &out_dir)?;
    let (_, tr_ref) = run_and_record("ctrl6_reference", &base, &opts, &out_dir)?;
    let n = tr_pert.t.len().min(tr_ref.t.len());
    let d0 = tr_pert.ip[0] - tr_ref.ip[0];
    let dend = tr_pert.ip[n - 1] - tr_ref.ip[n - 1];
    println!(
        "      ctrl6 initial-condition offset: dIp(0) = {:.3e} A -> dIp(t={:.4} s) = {:.3e} A (ratio {:.4}); a prescribed trajectory would keep the offset constant",
        d0,
        tr_ref.t[n - 1],
        dend,
        dend / d0
    );

    println!("[5] Tolerance refinement of the reference run");
    for &rt in &[1.0e-4, 1.0e-5, 1.0e-6, 1.0e-7] {
        let o = SolveOpts {
            rtol: rt,
            ..opts.clone()
        };
        let label = format!("reference_rtol{:.0e}", rt);
        let (s, _) = run_and_record(&label, &base, &o, &out_dir)?;
        runs.push(s);
    }

    let json = format!(
        "{{\"model\":\"0-D coupled plasma/vessel circuit, Spitzer R_p(T_e)\",\"geometry\":{{\"R0_m\":{},\"a_m\":{},\"kappa\":{},\"B0_T\":{},\"Ip0_A\":{},\"li\":{},\"ne_m3\":{},\"Lp_H\":{},\"Lv_H\":{},\"M_H\":{},\"Rv_Ohm\":{},\"coupling_k\":{}}},\"solver\":{{\"method\":\"CVODE BDF (rusty-SUNDIALS cvode crate)\",\"rtol_default\":{},\"atol\":{},\"dt_out_s\":{}}},\"iter_reference_tcq_range_s\":[0.05,0.15],\"runs\":[{}]}}",
        json_num(base.r0),
        json_num(base.a),
        json_num(base.kappa),
        json_num(base.b0),
        json_num(base.ip0),
        json_num(base.li),
        json_num(base.ne),
        json_num(lp),
        json_num(base.lv),
        json_num(base.m),
        json_num(base.rv),
        json_num(base.coupling_coefficient()),
        json_num(opts.rtol),
        json_num(opts.atol),
        json_num(opts.dt_out),
        runs.iter()
            .map(RunSummary::to_json)
            .collect::<Vec<_>>()
            .join(",")
    );
    let bulk_json = out_dir.join("summary.json");
    fs::write(&bulk_json, &json)?;
    let small_json = summary_dir.join("summary.json");
    fs::write(&small_json, &json)?;
    println!();
    println!("Summary JSON: {}", bulk_json.display());
    println!("Summary JSON: {}", small_json.display());
    println!(
        "Published ITER current-quench range for comparison: ~50-150 ms (Hender et al. 2007, Nucl. Fusion 47 S128; ITER Physics Basis). Not enforced."
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Controls. Run with: cargo test -p examples --example iter_current_quench_0d
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> SolveOpts {
        SolveOpts::default()
    }

    #[test]
    fn spitzer_matches_nrl_reference_value() {
        // NRL Plasma Formulary at 25 keV, lnLambda = 17, Z = 1: eta_par = 2.26e-10 Ohm m.
        let eta = spitzer_eta_parallel(25.0e3, 1.0, 17.0);
        assert!(
            (eta - 2.26e-10).abs() / 2.26e-10 < 0.01,
            "eta_par = {eta:e}"
        );
        // 10 eV, Zeff = 1, lnL = 10: 5.25e-5 * 10 / 31.62 = 1.66e-5 Ohm m
        let eta10 = spitzer_eta_parallel(10.0, 1.0, 10.0);
        assert!(
            (eta10 - 1.661e-5).abs() / 1.661e-5 < 0.01,
            "eta(10 eV) = {eta10:e}"
        );
        // Coulomb log for n_e = 1e20 m^-3, T_e = 10 eV, Z = 1 sits near 10.
        let lnl = coulomb_log_ei(1.0e20, 10.0, 1.0);
        assert!(lnl > 9.0 && lnl < 11.5, "lnLambda = {lnl}");
    }

    #[test]
    fn inductance_formula_and_coupling_are_sane() {
        let p = Params::iter_default();
        let lp = p.plasma_inductance();
        // mu0 R0 = 7.79e-6 H; bracket = ln(8*6.2/(2*sqrt(1.7))) - 2 + 0.4 = 1.331
        let expected = MU0 * 6.2 * ((49.6 / (2.0 * 1.7_f64.sqrt())).ln() - 1.6);
        assert!((lp - expected).abs() < 1e-12);
        assert!(lp > 9.0e-6 && lp < 12.0e-6, "L_p = {lp:e}");
        let k = p.coupling_coefficient();
        assert!(k > 0.3 && k < 1.0, "k = {k}");
    }

    /// Control 1 (positive): M = 0 => I_p = I_p0 exp(-t R_p/L_p).
    #[test]
    fn ctrl1_uncoupled_decay_matches_analytic() {
        let mut p = Params::iter_default();
        p.m = 0.0;
        let o = opts();
        let tr = simulate(&p, &o).expect("solve");
        let tau = tr.lp / p.plasma_resistance(p.te_ev);
        let mut worst = 0.0_f64;
        let mut n_checked = 0;
        for (k, &t) in tr.t.iter().enumerate() {
            if t <= 3.0 * tau {
                let exact = p.ip0 * (-t / tau).exp();
                worst = worst.max((tr.ip[k] - exact).abs() / exact);
                n_checked += 1;
            }
        }
        assert!(n_checked > 100, "only {n_checked} points checked");
        assert!(worst < 1.0e-4, "max rel err {worst:e} at rtol {:e}", o.rtol);
        // Vessel stays at zero when M = 0.
        assert!(tr.iv.iter().all(|&v| v.abs() < 1e-6));
        // t_CQ for a pure exponential is tau ln(4) / 0.6.
        let (_, _, tcq) = quench_time(&tr, p.ip0).expect("80/20 crossings");
        let tcq_exact = tau * 4.0_f64.ln() / 0.6;
        assert!(
            (tcq - tcq_exact).abs() / tcq_exact < 1.0e-4,
            "t_CQ {tcq} vs {tcq_exact}"
        );
        // Tolerance refinement: the error at rtol 1e-4 must be larger than at 1e-6.
        let o4 = SolveOpts {
            rtol: 1.0e-4,
            ..opts()
        };
        let tr4 = simulate(&p, &o4).expect("solve rtol 1e-4");
        let mut worst4 = 0.0_f64;
        for (k, &t) in tr4.t.iter().enumerate() {
            if t <= 3.0 * tau {
                let exact = p.ip0 * (-t / tau).exp();
                worst4 = worst4.max((tr4.ip[k] - exact).abs() / exact);
            }
        }
        assert!(
            worst4 > worst,
            "rtol 1e-4 err {worst4:e} not above rtol 1e-6 err {worst:e}"
        );
    }

    /// Control 2: magnetic energy decrease equals integrated Ohmic dissipation.
    #[test]
    fn ctrl2_energy_balance_coupled() {
        let p = Params::iter_default();
        let tr = simulate(&p, &opts()).expect("solve");
        let res = energy_residual(&tr);
        let res_trapz = energy_residual_trapezoid(&p, &tr);
        assert!(res < 1.0e-3, "quadrature-state residual {res:e}");
        assert!(res_trapz < 1.0e-3, "trapezoid residual {res_trapz:e}");
        // The vessel must actually carry current in the coupled run.
        let (iv_peak, _) = peak_vessel_current(&tr);
        assert!(iv_peak > 1.0e6, "I_v peak {iv_peak} A");
        // Total dissipation must be a substantial fraction of W_mag(0) by the end of the run.
        let w0 = tr.wmag[0];
        assert!((tr.qp.last().unwrap() + tr.qv.last().unwrap()) / w0 > 0.5);
    }

    /// Control 3 (perturbation): doubling R_p halves the decay time.
    #[test]
    fn ctrl3_doubling_resistance_halves_decay_time() {
        let mut p1 = Params::iter_default();
        p1.m = 0.0;
        let mut p2 = p1.clone();
        p2.eta_multiplier = 2.0;
        let tr1 = simulate(&p1, &opts()).expect("solve 1R");
        let tr2 = simulate(&p2, &opts()).expect("solve 2R");
        let tau1 = fit_decay_time(&tr1, p1.ip0).expect("fit 1R");
        let tau2 = fit_decay_time(&tr2, p2.ip0).expect("fit 2R");
        let ratio = tau2 / tau1;
        assert!((ratio - 0.5).abs() < 1.0e-3, "tau ratio {ratio}");
        let tcq1 = quench_time(&tr1, p1.ip0).unwrap().2;
        let tcq2 = quench_time(&tr2, p2.ip0).unwrap().2;
        assert!(
            (tcq2 / tcq1 - 0.5).abs() < 1.0e-3,
            "t_CQ ratio {}",
            tcq2 / tcq1
        );
        // Coupled case: the response is not exactly 1/2 (vessel time constant is
        // fixed) but t_CQ must still fall clearly.
        let pc1 = Params::iter_default();
        let mut pc2 = pc1.clone();
        pc2.eta_multiplier = 2.0;
        let c1 = quench_time(&simulate(&pc1, &opts()).unwrap(), pc1.ip0)
            .unwrap()
            .2;
        let c2 = quench_time(&simulate(&pc2, &opts()).unwrap(), pc2.ip0)
            .unwrap()
            .2;
        assert!(c2 < 0.8 * c1, "coupled t_CQ did not respond: {c1} -> {c2}");
        assert!(
            c2 > 0.4 * c1,
            "coupled t_CQ ratio {} implausibly small",
            c2 / c1
        );
    }

    /// Control 4 (ideal wall): with R_v = 0 the vessel flux M I_p + L_v I_v is
    /// exactly conserved and I_v -> (M/L_v)(I_p0 - I_p).
    #[test]
    fn ctrl4_ideal_wall_conserves_vessel_flux() {
        let mut p = Params::iter_default();
        p.rv = 0.0;
        let tr = simulate(&p, &opts()).expect("solve");
        let drift = vessel_flux_drift(&p, &tr);
        let o7 = SolveOpts {
            rtol: 1.0e-7,
            ..opts()
        };
        let drift7 = vessel_flux_drift(&p, &simulate(&p, &o7).expect("solve rtol 1e-7"));
        println!(
            "ideal-wall vessel flux drift: rtol 1e-6 -> {drift:e}, rtol 1e-7 -> {drift7:e} (nst={}, nfe={}, t_end={}, n_out={})",
            tr.nst,
            tr.nfe,
            tr.t.last().unwrap(),
            tr.t.len()
        );
        // A linear invariant is preserved to the Newton stopping tolerance,
        // which scales with rtol; it is not exact because the iteration
        // matrix I - gamma J is reused across steps (modified Newton).
        assert!(drift < 1.0e-5, "vessel flux drift {drift:e}");
        assert!(
            drift7 < drift,
            "drift did not shrink with rtol: {drift:e} -> {drift7:e}"
        );
        let k = tr.t.len() - 1;
        let iv_expected = p.m / p.lv * (p.ip0 - tr.ip[k]);
        assert!(
            (tr.iv[k] - iv_expected).abs() / iv_expected < 1.0e-5,
            "I_v end {} vs {}",
            tr.iv[k],
            iv_expected
        );
        // The plasma flux L_p I_p + M I_v is NOT conserved (R_p > 0): it must drop.
        let psi_p0 = tr.lp * tr.ip[0];
        let psi_pk = tr.lp * tr.ip[k] + p.m * tr.iv[k];
        assert!(
            psi_pk < 0.5 * psi_p0,
            "plasma flux should decay: {psi_p0} -> {psi_pk}"
        );
        // With a resistive vessel the same flux is not conserved (negative control).
        let pr = Params::iter_default();
        let drift_r = vessel_flux_drift(&pr, &simulate(&pr, &opts()).unwrap());
        assert!(
            drift_r > 1.0e-2,
            "resistive vessel flux drift only {drift_r:e}"
        );
    }

    /// Control 5: t_CQ over the 5-20 eV scan is finite, positive, monotone in T_e,
    /// and the comparison to the published ITER band is reported, not enforced.
    #[test]
    fn ctrl5_te_scan_monotone_and_reported() {
        let mut prev = 0.0;
        let mut inside_band = 0;
        for &te in &[5.0, 7.5, 10.0, 12.5, 15.0, 20.0] {
            let mut p = Params::iter_default();
            p.te_ev = te;
            let tr = simulate(&p, &opts()).expect("solve");
            let (_, _, tcq) = quench_time(&tr, p.ip0).expect("crossings");
            assert!(tcq.is_finite() && tcq > 0.0);
            assert!(
                tcq > prev,
                "t_CQ not increasing with T_e: {prev} -> {tcq} at {te} eV"
            );
            if (0.05..=0.15).contains(&tcq) {
                inside_band += 1;
            }
            println!(
                "Te = {te:5.1} eV  t_CQ = {:.4} s  (ITER band 0.05-0.15 s)",
                tcq
            );
            prev = tcq;
        }
        println!("{inside_band} of 6 scan points fall inside the published band (Zeff = 2)");
        // Scaling check: Spitzer gives tau ~ T^{3/2} up to lnLambda; between
        // 5 and 20 eV (factor 4 in T) the uncoupled decay time must grow by
        // roughly 8x within a lnLambda-driven tolerance.
        let mut p5 = Params::iter_default();
        p5.m = 0.0;
        p5.te_ev = 5.0;
        let mut p20 = p5.clone();
        p20.te_ev = 20.0;
        let r = fit_decay_time(&simulate(&p20, &opts()).unwrap(), p20.ip0).unwrap()
            / fit_decay_time(&simulate(&p5, &opts()).unwrap(), p5.ip0).unwrap();
        let lnl_ratio = coulomb_log_ei(p5.ne, 5.0, p5.zeff) / coulomb_log_ei(p5.ne, 20.0, p5.zeff);
        let expected = 8.0 * lnl_ratio;
        assert!(
            (r - expected).abs() / expected < 1.0e-3,
            "tau(20)/tau(5) = {r}, expected {expected}"
        );
    }

    /// Control 6 (C2 from the audit): an initial-condition offset must evolve,
    /// not persist as an additive constant.
    #[test]
    fn ctrl6_initial_offset_decays() {
        let p = Params::iter_default();
        let mut pp = p.clone();
        pp.ip0 = p.ip0 + 1.0e6;
        let a = simulate(&p, &opts()).unwrap();
        let b = simulate(&pp, &opts()).unwrap();
        let n = a.t.len().min(b.t.len());
        let d0 = b.ip[0] - a.ip[0];
        let dk = b.ip[n - 1] - a.ip[n - 1];
        assert!((d0 - 1.0e6).abs() < 1.0);
        assert!(dk.abs() < 0.2 * d0, "offset persisted: {d0} -> {dk}");
        assert!(
            dk > 0.0,
            "ordering must be preserved by a linear stable system"
        );
    }

    /// The Ohmic-radiative temperature closes the balance R_p(T) I^2 = P_rad.
    #[test]
    fn ohmic_radiative_temperature_closes_balance() {
        let mut p = Params::iter_default();
        p.te_mode = TeMode::OhmicRadiative;
        let te = p.te_ohmic_radiative(p.ip0);
        assert!(te > p.te_floor_ev);
        let p_ohm = p.plasma_resistance(te) * p.ip0 * p.ip0;
        let p_rad = p.radiated_power();
        assert!(
            (p_ohm - p_rad).abs() / p_rad < 1.0e-6,
            "P_ohm {p_ohm:e} vs P_rad {p_rad:e}"
        );
        // The quench in this mode still satisfies the magnetic energy balance.
        let tr = simulate(&p, &opts()).unwrap();
        assert!(energy_residual(&tr) < 1.0e-3);
        assert!(quench_time(&tr, p.ip0).is_some());
    }
}
