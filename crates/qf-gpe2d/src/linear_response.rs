//! Bogoliubov dispersion by linear response of the uniform GP ground state.
//!
//! Units are the crate's (`hbar = m = 1`). For background density `n0` and coupling `g` the
//! uniform state `psi = sqrt(n0)` with `mu = g n0` is the ground state on a periodic box; the
//! measurement relaxes to it in imaginary time from a perturbed start, then applies a weak
//! kick at one grid wavevector `k` and records the density Fourier component `A(t)` at `k`
//! while evolving in real time with the crate's Strang split step:
//!
//! * [`Kick::Phase`]: `psi -> psi * exp(i eps cos(k.r))` (an impulsive potential, as in the
//!   short-Bragg-pulse technique). Continuity gives `A(t) = (n0 k^2 eps / omega) sin(omega t)`
//!   to first order in `eps`, so the response measures `omega` twice: from the period, and from
//!   the amplitude through `S_amp = A_max / (2 n0 eps) = (k^2/2)/omega`.
//! * [`Kick::Density`]: `psi -> psi * sqrt(1 + eps cos(k.r))`, giving `A(t) = n0 eps cos(omega t)`
//!   (period only).
//!
//! The linearised GP dynamics about the uniform state only couples `k` with `-k`, and the
//! kinetic step is spectral, so for grid-aligned `k` the continuum answer
//! `eps_B(k) = sqrt(eps0 (eps0 + 2 g n0))`, `eps0 = k^2/2`, is exact in space; the remaining
//! errors are the second-order Strang splitting in time and `O(eps^2)` nonlinearity, and both are
//! measured by refinement rather than assumed. One practical condition was found by running this
//! measurement: the real-time step must be true Strang ([`crate::Splitting::Strang`]). With the
//! crate's Python-reference composition the frequency still converges at second order but the
//! response amplitude converges only at first order (and a long-wavelength 32x32 run grows from
//! round-off late in the record); it stays selectable so that failure can be reproduced. A
//! [`crate::max_dt`] cap (`ResponseConfig::dt_safety`) is kept as a conservative guard.
//!
//! What this does **not** measure: the classical GP field has no quantum density fluctuations,
//! so nothing here is an independent measurement of the quantum static structure factor. The
//! quantity `S = eps0/omega` is the Bijl-Feynman identification (exact for Bogoliubov theory),
//! and `S_amp` is the same quantity reached through the f-sum rule. Their agreement is a
//! self-consistency check of the time stepper, not a many-body result.

use crate::{Grid2D, Splitting, evolve, evolve_observe, max_dt};
use num_complex::Complex64;
use std::f64::consts::PI;

/// Free-particle energy `eps0 = k^2/2`.
pub fn free_energy(k: f64) -> f64 {
    0.5 * k * k
}

/// Bogoliubov energy `eps_B = sqrt(eps0 (eps0 + 2 g n0))`.
pub fn bogoliubov_energy(k: f64, g: f64, n0: f64) -> f64 {
    let e0 = free_energy(k);
    (e0 * (e0 + 2.0 * g * n0)).sqrt()
}

/// Bijl-Feynman structure factor of the Bogoliubov dispersion, `S_B = eps0 / eps_B`.
pub fn bogoliubov_s(k: f64, g: f64, n0: f64) -> f64 {
    free_energy(k) / bogoliubov_energy(k, g, n0)
}

/// `k_s = 2 m c / hbar = 2 sqrt(g n0)`; the self-dual wavenumber of the dual-scale bound.
pub fn sound_wavenumber(g: f64, n0: f64) -> f64 {
    2.0 * (g * n0).sqrt()
}

/// The dual-scale (diffraction-test) threshold `sqrt(k / (2 k_s))`. Bogoliubov theory satisfies
/// `S_B <= sqrt(k/(2 k_s))` with equality only at `k = k_s` (kernel-checked in the
/// QuantumFluids project as `sBog_le_sqrt`).
pub fn dual_scale_bound(k: f64, ks: f64) -> f64 {
    (k / (2.0 * ks)).sqrt()
}

/// How the uniform state is perturbed at `t = 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kick {
    Phase,
    Density,
}

/// One linear-response measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResponseConfig {
    /// Wavevector modulus.
    pub k: f64,
    /// Integer mode `(mx, my)` the kick is imprinted on; the box side is chosen as
    /// `L = 2 pi |m| / k` so that `k` sits exactly on the grid.
    pub mode: (i64, i64),
    /// Grid points per side.
    pub n: usize,
    pub g: f64,
    pub n0: f64,
    /// Kick strength.
    pub eps: f64,
    /// Time steps per *reference* period `2 pi / omega_ref`.
    pub steps_per_period: usize,
    /// Number of reference periods recorded.
    pub periods: usize,
    /// Reference frequency that sets `dt` (for example `eps_B(k)` or `k^2/2`); it only fixes the
    /// step size and the record length, the measured frequency does not use it.
    pub omega_ref: f64,
    pub kick: Kick,
    /// If `Some(s)`, cap the step at [`crate::max_dt`]`(grid, s)` (`dt k2_max / 2 <= s`).
    ///
    /// A conservative guard that keeps every grid mode's kinetic phase per step small. In the
    /// recorded runs (example `bogoliubov_dispersion`) it was *not* what decided correctness:
    /// with [`Splitting::Strang`] the uncapped sweep reproduced the capped one, and the amplitude
    /// errors first seen were caused by [`Splitting::PythonReference`], not by the step size.
    pub dt_safety: Option<f64>,
    /// Nonlinear half-step of the real-time evolution (see [`Splitting`]). Use
    /// [`Splitting::Strang`]; [`Splitting::PythonReference`] reproduces the upstream scheme.
    pub splitting: Splitting,
}

/// Result of [`measure`].
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    pub k: f64,
    pub dt: f64,
    pub box_l: f64,
    /// `max | |psi|^2 - n0 |` of the relaxed ground state before the kick.
    pub ground_state_residual: f64,
    /// Frequency from a least-squares line through the zero-crossing times of `A(t)`.
    pub omega_zero_crossing: f64,
    /// Frequency refined by a least-squares sinusoid fit (golden-section search on `omega`).
    pub omega_fit: f64,
    /// Fitted amplitude of `A(t)`.
    pub amplitude: f64,
    /// Number of zero crossings used.
    pub n_crossings: usize,
    /// The recorded `(t, A(t))` series.
    pub times: Vec<f64>,
    pub series: Vec<f64>,
}

impl Measurement {
    /// `S = eps0 / omega_zero_crossing`.
    pub fn s_freq(&self) -> f64 {
        free_energy(self.k) / self.omega_zero_crossing
    }

    /// `S_amp = A / (2 n0 eps)`, meaningful for [`Kick::Phase`] only.
    pub fn s_amp(&self, n0: f64, eps: f64) -> f64 {
        self.amplitude / (2.0 * n0 * eps)
    }
}

/// `2 Re` of the density Fourier component at wavevector `kv`, normalised so that a density
/// `n0 + A cos(k.r)` returns `A`.
pub fn density_cos_amplitude(psi: &[Complex64], grid: &Grid2D, kv: (f64, f64)) -> f64 {
    let n = grid.n;
    let mut acc = 0.0;
    for i in 0..n {
        let x = grid.coord(i);
        for j in 0..n {
            let y = grid.coord(j);
            acc += psi[i * n + j].norm_sqr() * (kv.0 * x + kv.1 * y).cos();
        }
    }
    2.0 * acc / (n * n) as f64
}

/// Relax a perturbed uniform state to the GP ground state in imaginary time (renormalising the
/// mean density to `n0` after every step) and return it with its residual
/// `max | |psi|^2 - n0 |`.
pub fn relaxed_ground_state(grid: &Grid2D, g: f64, n0: f64) -> (Vec<Complex64>, f64) {
    let n = grid.n;
    let l = grid.l();
    let k1 = 2.0 * PI / l;
    let mut psi = vec![Complex64::new(0.0, 0.0); n * n];
    for i in 0..n {
        let x = grid.coord(i);
        for j in 0..n {
            let y = grid.coord(j);
            let pert =
                0.02 * (k1 * x).cos() + 0.015 * (k1 * y + 0.3).sin() - 0.01 * (k1 * (x + y)).cos();
            psi[i * n + j] = Complex64::new((n0 * (1.0 + pert)).sqrt(), 0.0);
        }
    }
    // Slowest decay is the lowest mode: rate eps_B(k1) (or k1^2/2 when g = 0).
    let rate = bogoliubov_energy(k1, g, n0).max(free_energy(k1));
    let tau_total = 40.0 / rate;
    let n_steps = 4000usize;
    let dtau = tau_total / n_steps as f64;
    let chunk = 50usize;
    for _ in 0..(n_steps / chunk) {
        psi = evolve(
            &psi,
            grid,
            Complex64::new(0.0, -dtau),
            chunk,
            g,
            g * n0,
            false,
        );
        let mean: f64 = psi.iter().map(|z| z.norm_sqr()).sum::<f64>() / (n * n) as f64;
        let scale = (n0 / mean).sqrt();
        for v in psi.iter_mut() {
            *v *= scale;
        }
    }
    let resid = psi
        .iter()
        .map(|z| (z.norm_sqr() - n0).abs())
        .fold(0.0f64, f64::max);
    (psi, resid)
}

/// Run one linear-response measurement (see the module docs).
pub fn measure(cfg: &ResponseConfig) -> Measurement {
    let (mx, my) = cfg.mode;
    let mnorm = ((mx * mx + my * my) as f64).sqrt();
    assert!(mnorm > 0.0, "mode must be nonzero");
    let box_l = 2.0 * PI * mnorm / cfg.k;
    let grid = Grid2D::new(cfg.n, box_l / cfg.n as f64);
    let kv = (2.0 * PI * mx as f64 / box_l, 2.0 * PI * my as f64 / box_l);
    let (mut psi, resid) = relaxed_ground_state(&grid, cfg.g, cfg.n0);
    let n = cfg.n;
    for i in 0..n {
        let x = grid.coord(i);
        for j in 0..n {
            let y = grid.coord(j);
            let c = (kv.0 * x + kv.1 * y).cos();
            let f = match cfg.kick {
                Kick::Phase => Complex64::from_polar(1.0, cfg.eps * c),
                Kick::Density => Complex64::new((1.0 + cfg.eps * c).sqrt(), 0.0),
            };
            psi[i * n + j] *= f;
        }
    }
    let dt_period = 2.0 * PI / (cfg.omega_ref * cfg.steps_per_period as f64);
    let dt = match cfg.dt_safety {
        Some(s) => dt_period.min(max_dt(&grid, s)),
        None => dt_period,
    };
    let t_total = cfg.periods as f64 * 2.0 * PI / cfg.omega_ref;
    let n_steps = (t_total / dt).ceil() as usize;
    let mut times = Vec::with_capacity(n_steps + 1);
    let mut series = Vec::with_capacity(n_steps + 1);
    let mu = cfg.g * cfg.n0;
    evolve_observe(
        &psi,
        &grid,
        Complex64::new(dt, 0.0),
        n_steps,
        cfg.g,
        mu,
        cfg.splitting,
        |s, p| {
            times.push(s as f64 * dt);
            // Both kicks oscillate about zero density modulation at k (to first order).
            series.push(density_cos_amplitude(p, &grid, kv));
        },
    );
    let (omega_zc, n_cross) = zero_crossing_frequency(&times, &series, cfg.kick);
    let (omega_fit, amplitude) = refine_sinusoid(&times, &series, omega_zc);
    Measurement {
        k: cfg.k,
        dt,
        box_l,
        ground_state_residual: resid,
        omega_zero_crossing: omega_zc,
        omega_fit,
        amplitude,
        n_crossings: n_cross,
        times,
        series,
    }
}

/// Zero crossings of `A(t)` (linear interpolation), excluding `t = 0`. For a phase kick the
/// `j`-th crossing is at `j pi / omega`; for a density kick at `(j - 1/2) pi / omega`. A
/// least-squares line through `(j, t_j)` has slope `pi / omega`.
fn zero_crossing_frequency(t: &[f64], a: &[f64], kick: Kick) -> (f64, usize) {
    let mut crossings: Vec<f64> = Vec::new();
    // Skip the first sample for the phase kick (A(0) = 0 exactly).
    let start = 1;
    for i in start..a.len() - 1 {
        let (a0, a1) = (a[i], a[i + 1]);
        if a0 == 0.0 {
            continue;
        }
        if a0 * a1 < 0.0 || a1 == 0.0 {
            let frac = a0 / (a0 - a1);
            crossings.push(t[i] + frac * (t[i + 1] - t[i]));
        }
    }
    let m = crossings.len();
    if m < 2 {
        return (f64::NAN, m);
    }
    let offset = match kick {
        Kick::Phase => 1.0,
        Kick::Density => 0.5,
    };
    let js: Vec<f64> = (0..m).map(|j| j as f64 + offset).collect();
    let jm = js.iter().sum::<f64>() / m as f64;
    let tm = crossings.iter().sum::<f64>() / m as f64;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    for (j, tc) in js.iter().zip(&crossings) {
        sxy += (j - jm) * (tc - tm);
        sxx += (j - jm) * (j - jm);
    }
    let slope = sxy / sxx;
    (PI / slope, m)
}

/// Least-squares fit of `a sin(w t) + b cos(w t)` at fixed `w`; returns `(residual, amplitude)`.
fn fit_at(t: &[f64], a: &[f64], w: f64) -> (f64, f64) {
    let (mut ss, mut cc, mut sc, mut ys, mut yc) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (&ti, &ai) in t.iter().zip(a) {
        let (s, c) = (w * ti).sin_cos();
        ss += s * s;
        cc += c * c;
        sc += s * c;
        ys += ai * s;
        yc += ai * c;
    }
    let det = ss * cc - sc * sc;
    let alpha = (ys * cc - yc * sc) / det;
    let beta = (yc * ss - ys * sc) / det;
    let mut r = 0.0;
    for (&ti, &ai) in t.iter().zip(a) {
        let (s, c) = (w * ti).sin_cos();
        let e = ai - alpha * s - beta * c;
        r += e * e;
    }
    (r, (alpha * alpha + beta * beta).sqrt())
}

/// Golden-section search for the least-squares frequency within `+-1%` of `w0`.
fn refine_sinusoid(t: &[f64], a: &[f64], w0: f64) -> (f64, f64) {
    if !w0.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    let gr = (5f64.sqrt() - 1.0) / 2.0;
    let (mut lo, mut hi) = (0.99 * w0, 1.01 * w0);
    let mut x1 = hi - gr * (hi - lo);
    let mut x2 = lo + gr * (hi - lo);
    let mut f1 = fit_at(t, a, x1).0;
    let mut f2 = fit_at(t, a, x2).0;
    for _ in 0..200 {
        if f1 < f2 {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - gr * (hi - lo);
            f1 = fit_at(t, a, x1).0;
        } else {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + gr * (hi - lo);
            f2 = fit_at(t, a, x2).0;
        }
        if (hi - lo) < 1e-15 * w0 {
            break;
        }
    }
    let w = 0.5 * (lo + hi);
    (w, fit_at(t, a, w).1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_forms_match_hand_values() {
        // g = n0 = 1: k_s = 2; at k = k_s, eps0 = 2, eps_B = sqrt(2*4) = 2 sqrt 2,
        // S_B = 1/sqrt 2 = sqrt(k/(2 k_s)).
        let ks = sound_wavenumber(1.0, 1.0);
        assert_eq!(ks, 2.0);
        assert!((bogoliubov_energy(2.0, 1.0, 1.0) - 8f64.sqrt()).abs() < 1e-15);
        assert!((bogoliubov_s(2.0, 1.0, 1.0) - dual_scale_bound(2.0, ks)).abs() < 1e-15);
        // g = 0 reduces to the free particle.
        assert_eq!(bogoliubov_energy(3.0, 0.0, 1.0), free_energy(3.0));
    }

    #[test]
    fn cos_amplitude_reads_an_imprinted_modulation() {
        let grid = Grid2D::new(16, 0.5);
        let k = 2.0 * PI * 3.0 / grid.l();
        let psi: Vec<Complex64> = (0..16 * 16)
            .map(|idx| {
                let x = grid.coord(idx / 16);
                Complex64::new((1.0 + 0.25 * (k * x).cos()).sqrt(), 0.0)
            })
            .collect();
        assert!((density_cos_amplitude(&psi, &grid, (k, 0.0)) - 0.25).abs() < 1e-12);
        // Orthogonal wavevector reads zero.
        assert!(density_cos_amplitude(&psi, &grid, (0.0, k)).abs() < 1e-12);
    }
}
