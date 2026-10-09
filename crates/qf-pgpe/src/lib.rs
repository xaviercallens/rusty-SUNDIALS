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

pub mod flow;
pub mod npy;
pub mod rect;
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
    /// Rows `i` (first axis) / columns `j` (second axis) containing at least one mode inside the projector: the
    /// other lines are identically zero in a projected field, so their 1D FFTs are skipped (exact, not approximate).
    rows_active: Vec<bool>,
    cols_active: Vec<bool>,
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

/// In-place transpose of a square row-major `n x n` buffer (tiled, so both reads and writes stay in cache).
fn transpose_square(data: &mut [Complex64], n: usize) {
    const B: usize = 16;
    for bi in (0..n).step_by(B) {
        for bj in (bi..n).step_by(B) {
            for i in bi..(bi + B).min(n) {
                for j in (bj.max(i + 1))..(bj + B).min(n) {
                    data.swap(i * n + j, j * n + i);
                }
            }
        }
    }
}

/// One pass of 1D FFTs over the rows of `data` that are flagged in `active` (all rows if `None`).
///
/// (A rayon-parallel version of this pass was tried and removed: on the 4-core/8-thread test machine the step did not
/// get faster with 2-4 threads at n = 512 and 1024 (cause not investigated), so independent runs, not threads inside a
/// run, are the unit of parallelism.)
fn row_pass(fft: &Arc<dyn Fft<f64>>, data: &mut [Complex64], n: usize, active: Option<&[bool]>) {
    let scratch_len = fft.get_inplace_scratch_len();
    let mut scratch = vec![Complex64::new(0.0, 0.0); scratch_len];
    for (i, row) in data.chunks_mut(n).enumerate() {
        if active.is_none_or(|a| a[i]) {
            fft.process_with_scratch(row, &mut scratch);
        }
    }
}

/// Rows, transpose, rows, transpose: the 2D DFT is separable, so this equals `numpy.fft.fft2` (forward, unnormalized)
/// or `ifft2` before normalization, with unit-stride access in both passes (the column pass of the previous
/// implementation copied every column through a scratch vector). `first`/`second` optionally restrict the lines
/// transformed in each pass to those that can be nonzero (inputs) or are needed (outputs): lines skipped are zero, so
/// the result is bit-identical to the full transform at the entries that are kept.
fn fft2_pruned(
    fft: &Arc<dyn Fft<f64>>,
    data: &mut [Complex64],
    n: usize,
    first: Option<&[bool]>,
    second: Option<&[bool]>,
) {
    row_pass(fft, data, n, first);
    transpose_square(data, n);
    row_pass(fft, data, n, second);
    transpose_square(data, n);
}

/// In-place 2D FFT of an `n x n` row-major buffer, matching `numpy.fft.fft2` (unnormalized forward transform).
fn fft2_forward(fft: &Arc<dyn Fft<f64>>, data: &mut [Complex64], n: usize) {
    fft2_pruned(fft, data, n, None, None);
}

/// In-place 2D inverse FFT matching `numpy.fft.ifft2` (rustfft's own inverse plan is unnormalized; this divides by
/// `n*n` at the end to match numpy's normalized convention).
fn fft2_inverse(ifft: &Arc<dyn Fft<f64>>, data: &mut [Complex64], n: usize) {
    fft2_pruned(ifft, data, n, None, None);
    let scale = 1.0 / (n * n) as f64;
    for v in data.iter_mut() {
        *v *= scale;
    }
}

/// Scratch buffers of one IF-RK4 step, allocated once per [`ComplexField2D::run`].
pub struct Workspace {
    a: Vec<Complex64>,
    b: Vec<Complex64>,
    cc: Vec<Complex64>,
    d: Vec<Complex64>,
    tmp: Vec<Complex64>,
    psi: Vec<Complex64>,
}

impl Workspace {
    pub fn new(n: usize) -> Self {
        let z = vec![Complex64::new(0.0, 0.0); n * n];
        Workspace {
            a: z.clone(),
            b: z.clone(),
            cc: z.clone(),
            d: z.clone(),
            tmp: z.clone(),
            psi: z,
        }
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
        let rows_active: Vec<bool> = (0..n).map(|i| (0..n).any(|j| mask[i * n + j])).collect();
        let cols_active: Vec<bool> = (0..n).map(|j| (0..n).any(|i| mask[i * n + j])).collect();
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
            rows_active,
            cols_active,
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

    /// `N(c) = -i g P[ fft2( |psi|^2 psi ) ]`, written into `out` (`psi` is a scratch buffer).
    fn nonlin_into(&self, c: &[Complex64], out: &mut [Complex64], psi: &mut [Complex64]) {
        // `c` is a projected field: rows without any mode inside the projector are zero and are not transformed;
        // of the forward transform only the lines that contain modes inside the projector are kept (the rest is
        // zeroed by the mask below), so they are the only ones computed.
        psi.copy_from_slice(c);
        fft2_pruned(&self.fft_inv, psi, self.n, Some(&self.rows_active), None);
        let scale = 1.0 / (self.n * self.n) as f64;
        for (o, p) in out.iter_mut().zip(psi.iter()) {
            let p = p * scale;
            *o = p * p.norm_sqr();
        }
        fft2_pruned(&self.fft_fwd, out, self.n, None, Some(&self.cols_active));
        let ig = Complex64::new(0.0, -self.g);
        out.iter_mut()
            .zip(&self.mask)
            .for_each(|(v, &m)| *v = if m { ig * *v } else { Complex64::new(0.0, 0.0) });
    }

    fn nonlin(&self, c: &[Complex64]) -> Vec<Complex64> {
        let mut out = vec![Complex64::new(0.0, 0.0); c.len()];
        let mut psi = vec![Complex64::new(0.0, 0.0); c.len()];
        self.nonlin_into(c, &mut out, &mut psi);
        out
    }

    /// One IF-RK4 step into `out`, using the scratch buffers of `ws` (no allocation): linear part exact via
    /// `E1`/`E2`, nonlinear part classical RK4 -- the exact scheme of `pgpe.py`'s `step()`.
    #[allow(clippy::needless_range_loop)] // eight arrays are indexed in lock-step; iterators would obscure the scheme
    pub fn step_into(&self, c: &[Complex64], out: &mut [Complex64], ws: &mut Workspace) {
        let n = c.len();
        let dt = self.dt;
        self.nonlin_into(c, &mut ws.a, &mut ws.psi);
        for i in 0..n {
            ws.tmp[i] = self.e1[i] * (c[i] + ws.a[i] * (0.5 * dt));
        }
        self.nonlin_into(&ws.tmp, &mut ws.b, &mut ws.psi);
        for i in 0..n {
            ws.tmp[i] = self.e1[i] * c[i] + ws.b[i] * (0.5 * dt);
        }
        self.nonlin_into(&ws.tmp, &mut ws.cc, &mut ws.psi);
        for i in 0..n {
            ws.tmp[i] = self.e2[i] * c[i] + self.e1[i] * ws.cc[i] * dt;
        }
        self.nonlin_into(&ws.tmp, &mut ws.d, &mut ws.psi);
        for i in 0..n {
            out[i] = self.e2[i] * c[i]
                + (dt / 6.0)
                    * (self.e2[i] * ws.a[i] + 2.0 * self.e1[i] * (ws.b[i] + ws.cc[i]) + ws.d[i]);
        }
    }

    /// One IF-RK4 step (allocating convenience wrapper of [`Self::step_into`]).
    pub fn step(&self, c: &[Complex64]) -> Vec<Complex64> {
        let mut ws = Workspace::new(self.n);
        let mut out = vec![Complex64::new(0.0, 0.0); c.len()];
        self.step_into(c, &mut out, &mut ws);
        out
    }

    /// `round(t_end / dt)` steps from `c` (one workspace, two ping-pong buffers: no allocation in the loop).
    pub fn run(&self, c: &[Complex64], t_end: f64) -> Vec<Complex64> {
        let nsteps = (t_end / self.dt).round() as usize;
        let mut ws = Workspace::new(self.n);
        let mut cur = c.to_vec();
        let mut next = vec![Complex64::new(0.0, 0.0); c.len()];
        for _ in 0..nsteps {
            self.step_into(&cur, &mut next, &mut ws);
            std::mem::swap(&mut cur, &mut next);
        }
        cur
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

    /// The pruned transforms (zero rows skipped on the way in, unneeded lines skipped on the way out) are exact: the
    /// nonlinear term equals, bit for bit, the one computed with full 2D FFTs, for several cutoffs.
    #[test]
    fn pruned_ffts_are_bit_identical_to_full_ffts() {
        for frac in [0.5, 1.0 / 3.0, 0.4] {
            let f = ComplexField2D::with_kcut_frac(32, 16.0, 1.0, 0.01, frac);
            let n = f.n;
            let mut state = 0x1234_5678_9abc_def0u64;
            let mut next = || {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
            };
            let c: Vec<Complex64> = (0..n * n)
                .map(|i| {
                    if f.mask[i] {
                        Complex64::new(next(), next())
                    } else {
                        Complex64::new(0.0, 0.0)
                    }
                })
                .collect();
            let pruned = f.nonlin(&c);
            // reference: full inverse FFT, product, full forward FFT, mask
            let mut psi = c.clone();
            fft2_inverse(&f.fft_inv, &mut psi, n);
            let mut out: Vec<Complex64> = psi.iter().map(|p| p * p.norm_sqr()).collect();
            fft2_forward(&f.fft_fwd, &mut out, n);
            let ig = Complex64::new(0.0, -f.g);
            for (v, &m) in out.iter_mut().zip(&f.mask) {
                *v = if m { ig * *v } else { Complex64::new(0.0, 0.0) };
            }
            assert!(
                pruned.iter().zip(&out).all(|(a, b)| a == b),
                "kcut_frac = {frac}: pruned nonlinear term differs"
            );
            assert!(
                f.rows_active.iter().filter(|&&r| r).count() < n,
                "pruning must skip some rows at frac {frac}"
            );
        }
    }
}
