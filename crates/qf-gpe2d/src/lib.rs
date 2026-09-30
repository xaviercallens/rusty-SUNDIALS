//! Split-step Fourier solver for the 2D Gross–Pitaevskii equation on a periodic box:
//!
//! ```text
//! i dpsi/dt = -1/2 Lap psi + g |psi|^2 psi - mu psi          (hbar = m = 1)
//! ```
//!
//! A pure-Rust port of
//! [SocrateAI-Scientific-QuantumFluids](https://github.com/xaviercallens/SocrateAI-Scientific-QuantumFluids)'s
//! `src/quantumfluids/gpe/solver2d.py`, matching numpy's FFT conventions exactly. This is a
//! **different** model from this repository's [`qf-pgpe`] crate: `qf-pgpe` ports the BKT-specific
//! *projected* GPE (a sharp Fourier cutoff, no `mu`, integrating-factor RK4), used for the
//! project's dual-scale/BKT rounds. This crate is the general split-step workhorse used across 17
//! files of that project's PGPE-round energy-budget work, known-answer tests, and TDA
//! vortex-detection pipeline — a separate, independently-used solver, not a rename or refinement
//! of `qf-pgpe`.
//!
//! # Units and the timestep bound
//!
//! `g = 1`, background density `n0 = 1`, so `mu = 1`, sound speed `c = 1`, and the healing length
//! is `xi = 1/sqrt(2 mu)` ([`healing_length`]). The split-step scheme conserves the norm to
//! machine precision *by construction* (both half-steps are pointwise or Fourier-diagonal phase
//! rotations) — norm drift cannot fail and is not a useful control; energy drift is, and the test
//! suite uses it. The Fourier half-step advances mode `k` by `exp(-i dt k^2 / 2)`, so accuracy
//! needs `dt k_max^2 / 2 << 1` with `k_max = pi/dx`; [`max_dt`] gives a safe bound. A field with
//! energy at the grid scale (white-noise phase) genuinely needs it — the upstream Python docs
//! record 33% energy drift at 3x too large a step on such a field.
//!
//! # Vortex cores are relaxed, not planted
//!
//! [`plant_vortices`] imprints only a *phase* pattern (density left flat); [`evolve`] in imaginary
//! time (`dt` with negative imaginary part, `renorm: true`) then relaxes it, so the resulting core
//! profile is the solver's own solution, not an ansatz. Relax only briefly: imaginary time
//! descends toward the lowest-energy state, which on a periodic box is the vortex-free uniform
//! one — cores form first, then the vortices are destroyed if relaxation runs too long (see the
//! module docs on `plant_vortices` in the Python original for measured timings, reproduced in
//! this crate's `relaxation_builds_a_core_of_size_xi` / `over_relaxation_destroys_the_vortices`
//! tests).

use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use std::f64::consts::PI;
use std::sync::Arc;

/// A square periodic grid of `n x n` points spaced `dx` apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid2D {
    pub n: usize,
    pub dx: f64,
}

impl Grid2D {
    pub fn new(n: usize, dx: f64) -> Self {
        Grid2D { n, dx }
    }

    /// Box side length `n * dx`.
    pub fn l(&self) -> f64 {
        self.n as f64 * self.dx
    }

    /// `x`-coordinate of row/column index `i`: `i * dx`. Grid point `(i, j)` sits at physical
    /// position `(coord(i), coord(j))`.
    pub fn coord(&self, i: usize) -> f64 {
        i as f64 * self.dx
    }

    /// `k^2 = kx^2 + ky^2` on the row-major `n x n` grid, numpy `fftfreq` convention (angular,
    /// `2 pi k / (n dx)`, wrapped so index `i >= ceil(n/2)` reads as a negative frequency).
    pub fn k2(&self) -> Vec<f64> {
        let k1d: Vec<f64> = (0..self.n)
            .map(|i| angular_fftfreq(i, self.n, self.dx))
            .collect();
        let mut out = vec![0.0; self.n * self.n];
        for i in 0..self.n {
            for j in 0..self.n {
                out[i * self.n + j] = k1d[i] * k1d[i] + k1d[j] * k1d[j];
            }
        }
        out
    }
}

/// `2*pi*fftfreq(n, d)[k]`, numpy's convention. Exact for even and odd `n`.
fn angular_fftfreq(k: usize, n: usize, d: f64) -> f64 {
    let signed = if k < n.div_ceil(2) {
        k as i64
    } else {
        k as i64 - n as i64
    };
    2.0 * PI * (signed as f64) / (n as f64 * d)
}

/// In-place 2D FFT of an `n x n` row-major buffer, matching `numpy.fft.fft2` (unnormalized
/// forward transform).
fn fft2_forward(fft: &Arc<dyn Fft<f64>>, data: &mut [Complex64], n: usize) {
    for row in data.chunks_mut(n) {
        fft.process(row);
    }
    let mut col = vec![Complex64::new(0.0, 0.0); n];
    for j in 0..n {
        for i in 0..n {
            col[i] = data[i * n + j];
        }
        fft.process(&mut col);
        for i in 0..n {
            data[i * n + j] = col[i];
        }
    }
}

/// In-place 2D inverse FFT matching `numpy.fft.ifft2` (rustfft's own inverse plan is
/// unnormalized; this divides by `n*n` to match numpy's convention).
fn fft2_inverse(ifft: &Arc<dyn Fft<f64>>, data: &mut [Complex64], n: usize) {
    for row in data.chunks_mut(n) {
        ifft.process(row);
    }
    let mut col = vec![Complex64::new(0.0, 0.0); n];
    for j in 0..n {
        for i in 0..n {
            col[i] = data[i * n + j];
        }
        ifft.process(&mut col);
        for i in 0..n {
            data[i * n + j] = col[i];
        }
    }
    let scale = 1.0 / (n * n) as f64;
    for v in data.iter_mut() {
        *v *= scale;
    }
}

fn fft_pair(n: usize) -> (Arc<dyn Fft<f64>>, Arc<dyn Fft<f64>>) {
    let mut planner = FftPlanner::new();
    (planner.plan_fft_forward(n), planner.plan_fft_inverse(n))
}

/// `xi = 1/sqrt(2 mu)` in `hbar = m = 1` units.
pub fn healing_length(mu: f64) -> f64 {
    1.0 / (2.0 * mu).sqrt()
}

/// Largest timestep for which the Fourier half-step stays accurate: `safety * 2 / k_max^2`.
pub fn max_dt(grid: &Grid2D, safety: f64) -> f64 {
    let k2max = grid.k2().into_iter().fold(0.0, f64::max);
    safety * 2.0 / k2max
}

/// A minimal, seedable xorshift64* generator with a Box–Muller normal, used only to build a
/// band-limited random phase (below) and independent of numpy's own RNG stream — this crate's
/// tests establish the same *statistical* properties (energy conservation on a physical field),
/// not bit-identical draws.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn uniform01(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn normal(&mut self) -> f64 {
        let u1 = self.uniform01().max(1e-300);
        let u2 = self.uniform01();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }
}

/// A random phase band-limited to `k < k_cut_inv_xi / xi` — a physical initial condition, as
/// opposed to white noise, which puts energy at the grid scale where no split-step is accurate.
pub fn smooth_phase_noise(
    grid: &Grid2D,
    amp: f64,
    k_cut_inv_xi: f64,
    rng: &mut Rng,
    mu: f64,
) -> Vec<Complex64> {
    let xi = healing_length(mu);
    let n = grid.n;
    let mut f: Vec<Complex64> = (0..n * n)
        .map(|_| Complex64::new(rng.normal(), 0.0))
        .collect();
    let (fwd, inv) = fft_pair(n);
    fft2_forward(&fwd, &mut f, n);

    let k2 = grid.k2();
    let cutoff = k_cut_inv_xi / xi;
    for (v, &k2i) in f.iter_mut().zip(&k2) {
        if k2i.sqrt() >= cutoff {
            *v = Complex64::new(0.0, 0.0);
        }
    }
    fft2_inverse(&inv, &mut f, n);
    let ph: Vec<f64> = f.iter().map(|z| z.re).collect();
    let mean = ph.iter().sum::<f64>() / ph.len() as f64;
    let var = ph.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / ph.len() as f64;
    let std = var.sqrt();
    let scale = amp / (std + 1e-30);
    ph.iter()
        .map(|&p| Complex64::from_polar(1.0, p * scale))
        .collect()
}

/// One Strang split step. `dt` real advances real time; `dt` with a negative imaginary part
/// (`dt = -i tau`) is imaginary-time relaxation.
///
/// Builds a fresh FFT plan and recomputes `k2` on every call — convenient for a single step, but
/// [`evolve`] does **not** call this in its loop (it would replan the FFT every step); it shares
/// one plan and one `k2` across all `n_steps` instead.
pub fn step(psi: &[Complex64], grid: &Grid2D, dt: Complex64, g: f64, mu: f64) -> Vec<Complex64> {
    let (fwd, inv) = fft_pair(grid.n);
    let k2 = grid.k2();
    let mut buf = psi.to_vec();
    step_inplace(&mut buf, grid, dt, g, mu, &fwd, &inv, &k2);
    buf
}

#[allow(clippy::too_many_arguments)]
fn step_inplace(
    psi: &mut [Complex64],
    grid: &Grid2D,
    dt: Complex64,
    g: f64,
    mu: f64,
    fwd: &Arc<dyn Fft<f64>>,
    inv: &Arc<dyn Fft<f64>>,
    k2: &[f64],
) {
    let n = grid.n;
    let half: Vec<Complex64> = psi
        .iter()
        .map(|&p| (-Complex64::i() * dt * (g * p.norm_sqr() - mu) / 2.0).exp())
        .collect();
    for (v, &h) in psi.iter_mut().zip(&half) {
        *v *= h;
    }
    fft2_forward(fwd, psi, n);
    for (v, &k2i) in psi.iter_mut().zip(k2) {
        *v *= (-Complex64::i() * dt * k2i / 2.0).exp();
    }
    fft2_inverse(inv, psi, n);
    for (v, &h) in psi.iter_mut().zip(&half) {
        *v *= h;
    }
}

/// `n_steps` of the Strang split step (see [`step`]), sharing one FFT plan and one `k2` array
/// across every step. `renorm: true` rescales the norm back to `n^2` after every step — imaginary
/// time is not norm-preserving, unlike real time.
pub fn evolve(
    psi: &[Complex64],
    grid: &Grid2D,
    dt: Complex64,
    n_steps: usize,
    g: f64,
    mu: f64,
    renorm: bool,
) -> Vec<Complex64> {
    let (fwd, inv) = fft_pair(grid.n);
    let k2 = grid.k2();
    let mut psi = psi.to_vec();
    for _ in 0..n_steps {
        step_inplace(&mut psi, grid, dt, g, mu, &fwd, &inv, &k2);
        if renorm {
            let total: f64 = psi.iter().map(|z| z.norm_sqr()).sum();
            let scale = ((grid.n * grid.n) as f64 / total).sqrt();
            for v in psi.iter_mut() {
                *v *= scale;
            }
        }
    }
    psi
}

/// `E = int [ |grad psi|^2/2 + g|psi|^4/2 ]`. The kinetic part is computed via Fourier (exact on
/// the grid).
pub fn energy(psi: &[Complex64], grid: &Grid2D, g: f64) -> f64 {
    let n = grid.n;
    let mut psik = psi.to_vec();
    let (fwd, _inv) = fft_pair(n);
    fft2_forward(&fwd, &mut psik, n);
    let k2 = grid.k2();
    let kin: f64 = 0.5
        * psik
            .iter()
            .zip(&k2)
            .map(|(z, &k2i)| k2i * z.norm_sqr())
            .sum::<f64>()
        / (n * n) as f64;
    let inter: f64 = 0.5 * g * psi.iter().map(|z| z.norm_sqr().powi(2)).sum::<f64>();
    (kin + inter) * grid.dx * grid.dx
}

/// `int |psi|^2`.
pub fn norm(psi: &[Complex64], grid: &Grid2D) -> f64 {
    psi.iter().map(|z| z.norm_sqr()).sum::<f64>() * grid.dx * grid.dx
}

/// Wrap `x` into `(-l/2, l/2]`, the minimum-image convention on a periodic box of side `l`.
fn wrap(x: f64, l: f64) -> f64 {
    ((x + l / 2.0).rem_euclid(l)) - l / 2.0
}

/// Imprint a phase pattern with the given vortices at `centres` (physical `(x, y)` coordinates)
/// and integer `charges`; density is left flat (`sqrt(n0)`). `charges` must sum to zero — a
/// periodic box cannot carry net circulation.
pub fn plant_vortices(
    grid: &Grid2D,
    centres: &[(f64, f64)],
    charges: &[i32],
    n0: f64,
) -> Result<Vec<Complex64>, String> {
    let net: i32 = charges.iter().sum();
    if net != 0 {
        return Err(format!(
            "net charge {net} != 0 is impossible on a periodic box"
        ));
    }
    let n = grid.n;
    let l = grid.l();
    let mut phase = vec![0.0f64; n * n];
    for (&(cx, cy), &q) in centres.iter().zip(charges) {
        for i in 0..n {
            let dxp = wrap(grid.coord(i) - cx, l);
            for j in 0..n {
                let dyp = wrap(grid.coord(j) - cy, l);
                phase[i * n + j] += q as f64 * dyp.atan2(dxp);
            }
        }
    }
    Ok(phase
        .iter()
        .map(|&p| Complex64::from_polar(n0.sqrt(), p))
        .collect())
}

/// Azimuthally averaged `|psi|^2` around `centre`, in `n_bins` bins out to `r_max` — used to
/// measure a vortex core size. Returns `(bin_centres, profile)`; a bin with no points in range is
/// `NaN`.
pub fn radial_density_profile(
    psi: &[Complex64],
    grid: &Grid2D,
    centre: (f64, f64),
    r_max: f64,
    n_bins: usize,
) -> (Vec<f64>, Vec<f64>) {
    let n = grid.n;
    let l = grid.l();
    let edge = |b: usize| r_max * b as f64 / n_bins as f64;
    let mut sums = vec![0.0f64; n_bins];
    let mut counts = vec![0usize; n_bins];
    for i in 0..n {
        let dxp = wrap(grid.coord(i) - centre.0, l);
        for j in 0..n {
            let dyp = wrap(grid.coord(j) - centre.1, l);
            let r = dxp.hypot(dyp);
            if r < r_max {
                let b = ((r / r_max) * n_bins as f64) as usize;
                let b = b.min(n_bins - 1);
                sums[b] += psi[i * n + j].norm_sqr();
                counts[b] += 1;
            }
        }
    }
    let centres = (0..n_bins).map(|b| 0.5 * (edge(b) + edge(b + 1))).collect();
    let profile = sums
        .iter()
        .zip(&counts)
        .map(|(&s, &c)| if c > 0 { s / c as f64 } else { f64::NAN })
        .collect();
    (centres, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xi() -> f64 {
        healing_length(1.0)
    }
    fn grid(n: usize, per_xi: f64) -> Grid2D {
        Grid2D::new(n, xi() / per_xi)
    }

    #[test]
    fn norm_is_conserved_by_construction() {
        let g = grid(128, 8.0);
        let mut rng = Rng::new(0);
        let psi = smooth_phase_noise(&g, 0.3, 2.0, &mut rng, 1.0);
        let n0 = norm(&psi, &g);
        let dt = Complex64::new(max_dt(&g, 0.05), 0.0);
        let psi = evolve(&psi, &g, dt, 50, 1.0, 1.0, false);
        assert!((norm(&psi, &g) - n0).abs() / n0 < 1e-12);
    }

    #[test]
    fn energy_is_conserved_at_the_prescribed_timestep() {
        let g = grid(128, 8.0);
        let mut rng = Rng::new(1);
        let psi = smooth_phase_noise(&g, 0.3, 2.0, &mut rng, 1.0);
        let e0 = energy(&psi, &g, 1.0);
        let dt = Complex64::new(max_dt(&g, 0.05), 0.0);
        let psi = evolve(&psi, &g, dt, 400, 1.0, 1.0, false);
        let drift = (energy(&psi, &g, 1.0) - e0).abs() / e0.abs();
        assert!(drift < 1e-4, "energy drift {drift:.3e} at max_dt");
    }

    #[test]
    fn oversized_timestep_is_detectably_wrong() {
        let g = grid(128, 8.0);
        let mut rng = Rng::new(1);
        let psi = smooth_phase_noise(&g, 0.3, 2.0, &mut rng, 1.0);
        let e0 = energy(&psi, &g, 1.0);
        let dt = Complex64::new(50.0 * max_dt(&g, 0.05), 0.0);
        let bad = evolve(&psi, &g, dt, 400, 1.0, 1.0, false);
        let drift = (energy(&bad, &g, 1.0) - e0).abs() / e0.abs();
        assert!(
            drift > 1e-3,
            "50x max_dt did not visibly break energy conservation: {drift:.3e}"
        );
    }

    #[test]
    fn energy_drift_shrinks_with_timestep() {
        let g = grid(128, 8.0);
        let mut rng = Rng::new(2);
        let psi0 = smooth_phase_noise(&g, 0.3, 2.0, &mut rng, 1.0);
        let e0 = energy(&psi0, &g, 1.0);
        let base = max_dt(&g, 0.05);
        let coarse = evolve(
            &psi0,
            &g,
            Complex64::new(4.0 * base, 0.0),
            100,
            1.0,
            1.0,
            false,
        );
        let fine = evolve(
            &psi0,
            &g,
            Complex64::new(2.0 * base, 0.0),
            200,
            1.0,
            1.0,
            false,
        );
        let d_coarse = (energy(&coarse, &g, 1.0) - e0).abs();
        let d_fine = (energy(&fine, &g, 1.0) - e0).abs();
        assert!(
            d_fine < d_coarse,
            "drift did not shrink with dt: {d_coarse:.3e} -> {d_fine:.3e}"
        );
    }

    #[test]
    fn relaxation_builds_a_core_of_size_xi() {
        // n=256, matching Python exactly: vortex centres sit at fixed FRACTIONS of the box
        // (0.35 L, 0.65 L), so a smaller n also shrinks the vortex separation in units of xi --
        // a genuinely different physical setup, not a faster version of the same one. n=128
        // failed this test (core density 0.33 instead of <0.2 at tau=2) until this was fixed.
        let g = grid(256, 8.0);
        let l = g.l();
        let c = [(0.35 * l, l / 2.0), (0.65 * l, l / 2.0)];
        let psi = plant_vortices(&g, &c, &[1, -1], 1.0).unwrap();
        let psi = evolve(&psi, &g, Complex64::new(0.0, -0.02), 100, 1.0, 1.0, true); // tau = 2
        let (r, prof) = radial_density_profile(&psi, &g, c[0], 6.0 * xi(), 40);
        assert!(prof[0] < 0.2, "core not formed: {}", prof[0]);
        assert!(
            *prof.last().unwrap() > 0.8,
            "density does not recover: {}",
            prof.last().unwrap()
        );
        // linear interpolation for the half-density radius, matching np.interp
        let ok: Vec<(f64, f64)> = r
            .into_iter()
            .zip(prof)
            .filter(|(_, p)| !p.is_nan())
            .collect();
        let mut half = ok.last().unwrap().0;
        for w in ok.windows(2) {
            let (r0, p0) = w[0];
            let (r1, p1) = w[1];
            if (p0 - 0.5) * (p1 - 0.5) <= 0.0 && p1 != p0 {
                half = r0 + (0.5 - p0) * (r1 - r0) / (p1 - p0);
                break;
            }
        }
        assert!(
            0.5 * xi() < half && half < 3.0 * xi(),
            "core half-density radius {} xi",
            half / xi()
        );
    }

    #[test]
    fn over_relaxation_destroys_the_vortices() {
        let g = grid(256, 8.0);
        let l = g.l();
        let c = [(0.35 * l, l / 2.0), (0.65 * l, l / 2.0)];
        let psi = plant_vortices(&g, &c, &[1, -1], 1.0).unwrap();
        let long = evolve(&psi, &g, Complex64::new(0.0, -0.02), 1000, 1.0, 1.0, true); // tau = 20
        let (_, prof) = radial_density_profile(&long, &g, c[0], 6.0 * xi(), 40);
        assert!(
            prof[0] > 0.9,
            "expected the pair to have annihilated by tau = 20, core density {}",
            prof[0]
        );
    }

    #[test]
    fn net_charge_must_vanish_on_a_periodic_box() {
        let g = grid(32, 8.0);
        let err = plant_vortices(&g, &[(1.0, 1.0)], &[1], 1.0).unwrap_err();
        assert!(err.contains("net charge"));
    }
}
