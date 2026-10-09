//! `ComplexField2D`: the projected Gross-Pitaevskii equation on a doubly-periodic 2D grid,
//! integrated by the integrating-factor RK4 scheme (linear part exact, nonlinear part RK4),
//! with a sharp circular cutoff projector `k_cut = k_max/2` in Fourier space (the alias-free
//! condition for the cubic nonlinearity `|psi|^2 psi`; see
//! <https://github.com/xaviercallens/SocrateAI-Scientific-QuantumFluids/blob/master/docs/designs/PGPE_BKT_PREREG.md>,
//! amendment A1).
//!
//! ```text
//! i d/dt psi = P[ -1/2 Lap psi + g |psi|^2 psi ],   P = sharp cutoff |k| <= k_cut.
//! ```
//!
//! This is a pure-Rust port of that project's `pgpe.py`, matching numpy's FFT conventions
//! exactly (`fftfreq` ordering; forward FFT unnormalized, inverse divided by the total element
//! count) so that the two are numerically interchangeable to the tolerances of the project's
//! own pre-registered known-answer tests (K1 norm, K3 momentum, K4 the exact plane-wave
//! solution; ported below). **Not ported**: K2's `dt^4` truncation-order scaling regression and
//! K5's Bogoliubov-dispersion fit — real work, left as a named follow-up, not claimed here.
//!
//! No Python binding in this crate (see `rusty-sundials-py` for the pattern); the point of a
//! Rust port is to remove the per-step Python/GIL round-trip that makes `rusty-sundials-py`'s
//! `.solve()` "prohibitive above a few thousand unknowns" for a PDE-derived right-hand side —
//! wiring this crate to Python (via `pyo3`, exposing `step`/`run` on a numpy-backed buffer) is
//! the natural next PR and is not attempted here.

use rustfft::{Fft, FftPlanner, num_complex::Complex64};
use std::sync::Arc;

pub mod scattering;
pub mod thermal;
pub mod transport;
pub mod vortex;

/// The field state, invariants, and the one IF-RK4 step, on an `n x n` doubly-periodic grid of
/// side `l`, nonlinearity strength `g`, time step `dt`, cutoff fraction `kcut_frac` of `k_max`.
pub struct ComplexField2D {
    pub n: usize,
    pub l: f64,
    pub dx: f64,
    pub g: f64,
    pub dt: f64,
    /// Angular wavenumber along the first (row) axis, `kx[i*n+j] = k1[i]` (numpy `meshgrid(..,
    /// indexing="ij")` convention).
    pub kx: Vec<f64>,
    pub ky: Vec<f64>,
    pub k2: Vec<f64>,
    pub kmax: f64,
    pub kcut: f64,
    /// The projector `P = (sqrt(k2) <= kcut)`.
    pub mask: Vec<bool>,
    e1: Vec<Complex64>,
    e2: Vec<Complex64>,
    fft_fwd: Arc<dyn Fft<f64>>,
    fft_inv: Arc<dyn Fft<f64>>,
}

fn c_len(n: usize) -> usize {
    n * n
}

/// `2*pi*fftfreq(n, d)[k]`, numpy's convention: `k` for `k < ceil(n/2)`, else `k - n`, all
/// divided by `n*d`. Exact for both even and odd `n`; this project always uses even `n`.
fn angular_fftfreq(k: usize, n: usize, d: f64) -> f64 {
    let signed = if k < n.div_ceil(2) {
        k as i64
    } else {
        k as i64 - n as i64
    };
    2.0 * std::f64::consts::PI * (signed as f64) / (n as f64 * d)
}

/// In-place 2D FFT of an `n x n` row-major buffer, matching `numpy.fft.fft2` (unnormalized
/// forward transform, axes processed independently — the 2D DFT is separable, so order does not
/// affect the result to floating-point rounding).
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
/// unnormalized; this divides by `n*n` at the end to match numpy's normalized convention).
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

impl ComplexField2D {
    /// `kcut_frac = 1/2` (`k_max/2`) is the alias-free cutoff for the cubic nonlinearity and is
    /// what this project always runs with; see `new` for the general constructor.
    pub fn new(n: usize, l: f64, g: f64, dt: f64) -> Self {
        Self::with_kcut_frac(n, l, g, dt, 0.5)
    }

    pub fn with_kcut_frac(n: usize, l: f64, g: f64, dt: f64, kcut_frac: f64) -> Self {
        let dx = l / n as f64;
        let k1: Vec<f64> = (0..n).map(|k| angular_fftfreq(k, n, dx)).collect();
        let mut kx = vec![0.0; n * n];
        let mut ky = vec![0.0; n * n];
        let mut k2 = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                let (kxi, kyj) = (k1[i], k1[j]);
                kx[i * n + j] = kxi;
                ky[i * n + j] = kyj;
                k2[i * n + j] = kxi * kxi + kyj * kyj;
            }
        }
        let kmax = std::f64::consts::PI / dx;
        let kcut = kcut_frac * kmax;
        let mask: Vec<bool> = k2.iter().map(|&k2v| k2v.sqrt() <= kcut).collect();
        let e1: Vec<Complex64> = k2
            .iter()
            .zip(&mask)
            .map(|(&k2v, &m)| {
                if m {
                    (Complex64::new(0.0, -0.5 * k2v) * (dt / 2.0)).exp()
                } else {
                    Complex64::new(0.0, 0.0)
                }
            })
            .collect();
        let e2: Vec<Complex64> = k2
            .iter()
            .zip(&mask)
            .map(|(&k2v, &m)| {
                if m {
                    (Complex64::new(0.0, -0.5 * k2v) * dt).exp()
                } else {
                    Complex64::new(0.0, 0.0)
                }
            })
            .collect();
        let mut planner = FftPlanner::new();
        let fft_fwd = planner.plan_fft_forward(n);
        let fft_inv = planner.plan_fft_inverse(n);
        Self {
            n,
            l,
            dx,
            g,
            dt,
            kx,
            ky,
            k2,
            kmax,
            kcut,
            mask,
            e1,
            e2,
            fft_fwd,
            fft_inv,
        }
    }

    /// `psi = ifft2(c)`.
    pub fn psi(&self, c: &[Complex64]) -> Vec<Complex64> {
        let mut buf = c.to_vec();
        fft2_inverse(&self.fft_inv, &mut buf, self.n);
        buf
    }

    /// `modes(psi) = P . fft2(psi)`.
    pub fn modes(&self, psi: &[Complex64]) -> Vec<Complex64> {
        let mut buf = psi.to_vec();
        fft2_forward(&self.fft_fwd, &mut buf, self.n);
        for (v, &m) in buf.iter_mut().zip(&self.mask) {
            if !m {
                *v = Complex64::new(0.0, 0.0);
            }
        }
        buf
    }

    /// Full right-hand side `dc/dt = -i k^2/2 c + N(c)` on the projected modes, for external integrators
    /// (rusty-SUNDIALS CVODE in `tests/cvode_crosscheck.rs`).
    pub fn rhs(&self, c: &[Complex64]) -> Vec<Complex64> {
        let nl = self.nonlin(c);
        (0..c.len())
            .map(|i| {
                if self.mask[i] {
                    Complex64::new(0.0, -0.5 * self.k2[i]) * c[i] + nl[i]
                } else {
                    Complex64::new(0.0, 0.0)
                }
            })
            .collect()
    }

    /// Number of modes inside the projector (half the length of the real state vector of [`Self::pack`]).
    pub fn n_modes(&self) -> usize {
        self.mask.iter().filter(|&&m| m).count()
    }

    /// Real state vector `[Re c_k..., Im c_k...]` over the modes inside the projector, in grid order.
    pub fn pack(&self, c: &[Complex64]) -> Vec<f64> {
        let sel: Vec<Complex64> = c
            .iter()
            .zip(&self.mask)
            .filter(|(_, m)| **m)
            .map(|(v, _)| *v)
            .collect();
        sel.iter()
            .map(|v| v.re)
            .chain(sel.iter().map(|v| v.im))
            .collect()
    }

    /// Inverse of [`Self::pack`].
    pub fn unpack(&self, y: &[f64]) -> Vec<Complex64> {
        let m = self.n_modes();
        let mut out = vec![Complex64::new(0.0, 0.0); c_len(self.n)];
        let mut k = 0;
        for (i, &on) in self.mask.iter().enumerate() {
            if on {
                out[i] = Complex64::new(y[k], y[m + k]);
                k += 1;
            }
        }
        out
    }

    /// `N(c) = -i g P[ fft2( |psi|^2 psi ) ]`.
    fn nonlin(&self, c: &[Complex64]) -> Vec<Complex64> {
        let psi = self.psi(c);
        let mut src: Vec<Complex64> = psi.iter().map(|p| p * p.norm_sqr()).collect();
        fft2_forward(&self.fft_fwd, &mut src, self.n);
        let ig = Complex64::new(0.0, -self.g);
        src.iter_mut()
            .zip(&self.mask)
            .for_each(|(v, &m)| *v = if m { ig * *v } else { Complex64::new(0.0, 0.0) });
        src
    }

    /// One IF-RK4 step (linear part exact via `E1`/`E2`, nonlinear part classical RK4) —
    /// the exact scheme of `pgpe.py`'s `step()`.
    pub fn step(&self, c: &[Complex64]) -> Vec<Complex64> {
        let n = c.len();
        let a = self.nonlin(c);
        let tmp: Vec<Complex64> = (0..n)
            .map(|i| self.e1[i] * (c[i] + a[i] * (0.5 * self.dt)))
            .collect();
        let b = self.nonlin(&tmp);
        let tmp2: Vec<Complex64> = (0..n)
            .map(|i| self.e1[i] * c[i] + b[i] * (0.5 * self.dt))
            .collect();
        let cc = self.nonlin(&tmp2);
        let tmp3: Vec<Complex64> = (0..n)
            .map(|i| self.e2[i] * c[i] + self.e1[i] * cc[i] * self.dt)
            .collect();
        let d = self.nonlin(&tmp3);
        (0..n)
            .map(|i| {
                self.e2[i] * c[i]
                    + (self.dt / 6.0)
                        * (self.e2[i] * a[i] + 2.0 * self.e1[i] * (b[i] + cc[i]) + d[i])
            })
            .collect()
    }

    /// `round(t_end / dt)` steps from `c`.
    pub fn run(&self, c: &[Complex64], t_end: f64) -> Vec<Complex64> {
        let nsteps = (t_end / self.dt).round() as usize;
        let mut state = c.to_vec();
        for _ in 0..nsteps {
            state = self.step(&state);
        }
        state
    }

    /// `N = sum(|c|^2) dx^2 / n^2` (Parseval, numpy's unnormalized-FFT convention).
    pub fn norm(&self, c: &[Complex64]) -> f64 {
        let s: f64 = c.iter().map(|v| v.norm_sqr()).sum();
        s * self.dx * self.dx / (self.n * self.n) as f64
    }

    /// `E = sum(0.5 k^2 |c|^2) dx^2/n^2 + 0.5 g sum(|psi|^4) dx^2`.
    pub fn energy(&self, c: &[Complex64]) -> f64 {
        let psi = self.psi(c);
        let kin: f64 = c
            .iter()
            .zip(&self.k2)
            .map(|(v, &k2v)| 0.5 * k2v * v.norm_sqr())
            .sum::<f64>()
            * self.dx
            * self.dx
            / (self.n * self.n) as f64;
        let pot: f64 = psi.iter().map(|p| p.norm_sqr().powi(2)).sum::<f64>()
            * 0.5
            * self.g
            * self.dx
            * self.dx;
        kin + pot
    }

    /// `(sum kx w, sum ky w)`, `w = |c|^2 dx^2/n^2`.
    pub fn momentum(&self, c: &[Complex64]) -> (f64, f64) {
        let w: Vec<f64> = c
            .iter()
            .map(|v| v.norm_sqr() * self.dx * self.dx / (self.n * self.n) as f64)
            .collect();
        let px: f64 = w.iter().zip(&self.kx).map(|(&wi, &kxi)| wi * kxi).sum();
        let py: f64 = w.iter().zip(&self.ky).map(|(&wi, &kyi)| wi * kyi).sum();
        (px, py)
    }

    /// The `x`-coordinate grid, `x[i] = i * dx`, for building initial conditions.
    pub fn coords(&self) -> Vec<f64> {
        (0..self.n).map(|i| i as f64 * self.dx).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// K4 (`PGPE_BKT_PREREG.md`): a plane wave `psi = sqrt(n0) exp(i(k.x - w t))`,
    /// `w = k^2/2 + g n0`, is an exact solution. Same grid, same `k`, same `t_end` as the
    /// Python known-answer test (`known_answers.py`); same pass criterion (`1e-9`), and the
    /// same order of magnitude error (Python: see that file's own printed result).
    #[test]
    fn k4_plane_wave_exact_solution() {
        let n = 64;
        let l = 32.0;
        let field = ComplexField2D::new(n, l, 1.0, 0.005);
        let x = field.coords();
        let kx = 2.0 * std::f64::consts::PI * 3.0 / l;
        let n0 = 1.0_f64;
        let psi0: Vec<Complex64> = (0..n)
            .flat_map(|i| (0..n).map(move |_j| i))
            .map(|i| n0.sqrt() * Complex64::new(0.0, kx * x[i]).exp())
            .collect();
        let c0 = field.modes(&psi0);
        let c_final = field.run(&c0, 10.0);
        let psi_final = field.psi(&c_final);
        let w = 0.5 * kx * kx + n0;
        let phase = Complex64::new(0.0, -w * 10.0).exp();
        let max_err = psi0
            .iter()
            .zip(&psi_final)
            .map(|(p0, pf)| (pf - p0 * phase).norm())
            .fold(0.0_f64, f64::max);
        assert!(
            max_err <= 1e-9,
            "K4 max|error| = {max_err:e}, criterion 1e-9"
        );
    }

    /// K1 (norm conservation) and K3 (momentum conservation): a deterministic, non-uniform
    /// initial state (not Python's `random_state`, which needs a bit-identical PRNG to
    /// replicate — these invariants hold for any state, so a fixed one suffices), run for a
    /// modest time, checked against the same tolerances as the Python test.
    fn seeded_state(field: &ComplexField2D) -> Vec<Complex64> {
        let x = field.coords();
        let n = field.n;
        let k1 = 2.0 * std::f64::consts::PI / field.l;
        let k2c = 2.0 * std::f64::consts::PI * 2.0 / field.l;
        let mut psi = vec![Complex64::new(0.0, 0.0); n * n];
        for i in 0..n {
            for j in 0..n {
                let (xi, yj) = (x[i], x[j]);
                psi[i * n + j] = Complex64::new(1.0, 0.0)
                    + 0.1 * Complex64::new(0.0, k1 * xi + k2c * yj).exp()
                    + 0.05 * Complex64::new(0.0, -k2c * xi + k1 * yj).exp();
            }
        }
        field.modes(&psi)
    }

    #[test]
    fn k1_norm_conservation() {
        let field = ComplexField2D::new(64, 32.0, 1.0, 0.005);
        let c0 = seeded_state(&field);
        let n0 = field.norm(&c0);
        let c_final = field.run(&c0, 20.0);
        let drift = (field.norm(&c_final) - n0).abs() / n0;
        assert!(drift <= 1e-8, "K1 norm drift = {drift:e}, criterion 1e-8");
    }

    #[test]
    fn k3_momentum_conservation() {
        let field = ComplexField2D::new(64, 32.0, 1.0, 0.005);
        let c0 = seeded_state(&field);
        let p0 = field.momentum(&c0);
        let c_final = field.run(&c0, 20.0);
        let pf = field.momentum(&c_final);
        let drift = ((pf.0 - p0.0).abs()).max((pf.1 - p0.1).abs());
        assert!(
            drift <= 1e-10,
            "K3 momentum drift = {drift:e}, criterion 1e-10"
        );
    }

    /// Not a pre-registered criterion (K2 proper is the `dt^4` truncation-order scaling
    /// regression, not ported here): a basic sanity check that energy does not drift more than
    /// K2's own `dt = 0.005` bound over the K1/K3 tests' own run length.
    #[test]
    fn energy_sanity_bound() {
        let field = ComplexField2D::new(64, 32.0, 1.0, 0.005);
        let c0 = seeded_state(&field);
        let e0 = field.energy(&c0);
        let c_final = field.run(&c0, 20.0);
        let drift = (field.energy(&c_final) - e0).abs() / e0.abs();
        assert!(
            drift <= 1e-6,
            "energy drift = {drift:e}, K2's own dt=0.005 bound 1e-6"
        );
    }
}
