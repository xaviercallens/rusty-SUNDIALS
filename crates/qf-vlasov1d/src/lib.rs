//! 1D1V electrostatic Vlasov–Poisson, electrons on a neutralising background:
//!
//! ```text
//! df/dt + v df/dx - E df/dv = 0,     dE/dx = 1 - rho,     rho = int f dv.
//! ```
//!
//! A pure-Rust port of
//! [SocrateAI-Scientific-QuantumFluids](https://github.com/xaviercallens/SocrateAI-Scientific-QuantumFluids)'s
//! `src/quantumfluids/kinetic/vlasov.py`, used for that project's Landau-damping and plasma-echo
//! pre-registration (K1–K3 of `docs/designs/KINETIC_TDA_PREREG.md`).
//!
//! # The scheme
//!
//! Strang splitting. The `x`-advection `f(x - v dt, v)` is an **exact** Fourier shift (no
//! approximation: multiplying the Fourier coefficient by `exp(-i k v dt)` is the exact solution of
//! `df/dt + v df/dx = 0` for a periodic domain). The `v`-advection `f(x, v + E dt)` is a cubic
//! semi-Lagrangian interpolation, zero outside `[-vmax, vmax]`.
//!
//! **Spline note, an honest deviation from the Python original.** The Python solver uses
//! `scipy.interpolate.CubicSpline`'s default not-a-knot boundary condition; this crate uses a
//! natural cubic spline (zero second derivative at the two endpoints) to avoid depending on an
//! external linear-algebra crate for a small system. The two disagree only in the boundary
//! treatment of the two outermost intervals of the velocity grid, where — for the physical
//! (near-Maxwellian) distributions this solver is used on — `f` is already many orders of
//! magnitude below its peak. The tests below (Landau damping rate and frequency, ported from the
//! Python suite) tolerate 1–2% disagreement for exactly this reason: this is Landau-damping
//! physics, not a bit-exact numerical reproduction, and the boundary-spline choice is within that
//! tolerance. The one test that needs tight numerical agreement (`ballistic_echo_matches_closed_form`,
//! `1e-6` relative) runs with `field_on = false`, so it never calls the spline at all.
//!
//! Two failure modes of this scheme are pinned by tests rather than hidden:
//! * **Recurrence.** The velocity quadrature `rho = sum f dv` aliases the free-streaming phase
//!   `exp(-i k v t)` at `T_R = 2 pi / (k dv)`: the initial perturbation reappears. Not physical,
//!   and no resolution removes it, only postpones it.
//! * **Filamentation.** `f` develops `v`-structure of wavelength `2 pi / (k t)`; once that reaches
//!   a few `dv` the spline smooths it away. The plasma echo exists to test exactly this.

use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use std::f64::consts::PI;
use std::sync::Arc;

/// A periodic-in-`x`, bounded-in-`v` phase-space grid.
#[derive(Debug, Clone, Copy)]
pub struct Grid {
    pub l: f64,
    pub nx: usize,
    pub nv: usize,
    pub vmax: f64,
}

impl Grid {
    pub fn new(l: f64, nx: usize, nv: usize, vmax: f64) -> Self {
        Grid { l, nx, nv, vmax }
    }

    pub fn dx(&self) -> f64 {
        self.l / self.nx as f64
    }
    pub fn dv(&self) -> f64 {
        2.0 * self.vmax / self.nv as f64
    }
    pub fn x(&self, i: usize) -> f64 {
        i as f64 * self.dx()
    }
    /// Cell centres, symmetric about zero: `v_j = -vmax + (j + 1/2) dv`.
    pub fn v(&self, j: usize) -> f64 {
        -self.vmax + (j as f64 + 0.5) * self.dv()
    }
    pub fn v_grid(&self) -> Vec<f64> {
        (0..self.nv).map(|j| self.v(j)).collect()
    }
    /// `2 pi fftfreq(nx, dx)`, numpy's convention.
    pub fn kx(&self, i: usize) -> f64 {
        let n = self.nx;
        let signed = if i < n.div_ceil(2) {
            i as i64
        } else {
            i as i64 - n as i64
        };
        2.0 * PI * (signed as f64) / (n as f64 * self.dx())
    }

    /// The free-streaming recurrence time of Fourier mode `mode`: `T_R = 2 pi / (mode * kx_1 *
    /// dv)`, where `kx_1 = 2 pi / L` is the fundamental wavenumber.
    pub fn recurrence_time(&self, mode: i64) -> f64 {
        2.0 * PI / (mode as f64 * (2.0 * PI / self.l) * self.dv())
    }
}

pub fn maxwellian(v: f64, u: f64) -> f64 {
    (-0.5 * (v - u).powi(2)).exp() / (2.0 * PI).sqrt()
}

/// `rho(x) = int f(x, v) dv`, row-major `f[i * nv + j]`.
pub fn density(f: &[f64], g: &Grid) -> Vec<f64> {
    let dv = g.dv();
    f.chunks(g.nv)
        .map(|row| row.iter().sum::<f64>() * dv)
        .collect()
}

fn fft_pair(n: usize) -> (Arc<dyn Fft<f64>>, Arc<dyn Fft<f64>>) {
    let mut planner = FftPlanner::new();
    (planner.plan_fft_forward(n), planner.plan_fft_inverse(n))
}

/// `E(x)` solving `dE/dx = 1 - rho` via `i k E_k = -rho_k` (the background cancels the `k=0`
/// part), matching `np.fft.fft`/`ifft`'s unnormalized/normalized convention.
pub fn field_from(f: &[f64], g: &Grid) -> Vec<f64> {
    let rho = density(f, g);
    let n = g.nx;
    let mut rho_k: Vec<Complex64> = rho.iter().map(|&r| Complex64::new(r, 0.0)).collect();
    let (fwd, inv) = fft_pair(n);
    fwd.process(&mut rho_k);
    let mut e_k = vec![Complex64::new(0.0, 0.0); n];
    for i in 1..n {
        e_k[i] = Complex64::i() * rho_k[i] / g.kx(i);
    }
    inv.process(&mut e_k);
    let scale = 1.0 / n as f64;
    e_k.iter().map(|z| z.re * scale).collect()
}

/// Exact Fourier `x`-advection: multiplies mode `kx[i]` by `exp(-i kx[i] v[j] dt)`, per `v`
/// column.
pub fn advect_x(f: &[f64], g: &Grid, dt: f64) -> Vec<f64> {
    let (nx, nv) = (g.nx, g.nv);
    let (fwd, inv) = fft_pair(nx);
    let v = g.v_grid();
    let kx: Vec<f64> = (0..nx).map(|i| g.kx(i)).collect();
    let mut out = vec![0.0; nx * nv];
    let mut col = vec![Complex64::new(0.0, 0.0); nx];
    for j in 0..nv {
        for i in 0..nx {
            col[i] = Complex64::new(f[i * nv + j], 0.0);
        }
        fwd.process(&mut col);
        for i in 0..nx {
            col[i] *= (-Complex64::i() * kx[i] * v[j] * dt).exp();
        }
        inv.process(&mut col);
        let scale = 1.0 / nx as f64;
        for i in 0..nx {
            out[i * nv + j] = col[i].re * scale;
        }
    }
    out
}

/// Natural cubic spline (zero second derivative at both ends) on a uniform grid, evaluated at
/// `query` (arbitrary points); returns `None` for a query point outside `[y0's domain]`,
/// matching `CubicSpline(..., extrapolate=False)` before the caller substitutes `0.0`.
struct NaturalSpline {
    v0: f64,
    h: f64,
    n: usize,
    y: Vec<f64>,
    /// Second derivatives at each grid point.
    m: Vec<f64>,
}

impl NaturalSpline {
    fn new(v0: f64, h: f64, y: &[f64]) -> Self {
        let n = y.len();
        let mut m = vec![0.0; n];
        if n >= 3 {
            // Thomas algorithm for M_1..M_{n-2}: M_{i-1} + 4 M_i + M_{i+1} = 6/h^2 (y_{i+1} - 2y_i + y_{i-1}).
            let inner = n - 2;
            let mut c_prime = vec![0.0; inner];
            let mut d_prime = vec![0.0; inner];
            let rhs = |i: usize| 6.0 / (h * h) * (y[i + 1] - 2.0 * y[i] + y[i - 1]);
            c_prime[0] = 1.0 / 4.0;
            d_prime[0] = rhs(1) / 4.0;
            for k in 1..inner {
                let i = k + 1;
                let denom = 4.0 - c_prime[k - 1];
                c_prime[k] = 1.0 / denom;
                d_prime[k] = (rhs(i) - d_prime[k - 1]) / denom;
            }
            let mut sol = vec![0.0; inner];
            sol[inner - 1] = d_prime[inner - 1];
            for k in (0..inner - 1).rev() {
                sol[k] = d_prime[k] - c_prime[k] * sol[k + 1];
            }
            m[1..n - 1].copy_from_slice(&sol);
        }
        NaturalSpline {
            v0,
            h,
            n,
            y: y.to_vec(),
            m,
        }
    }

    /// `None` outside `[v0, v0 + (n-1) h]`.
    fn eval(&self, x: f64) -> Option<f64> {
        let last = self.v0 + (self.n - 1) as f64 * self.h;
        if x < self.v0 || x > last {
            return None;
        }
        let mut i = ((x - self.v0) / self.h).floor() as isize;
        if i < 0 {
            i = 0;
        }
        let i = (i as usize).min(self.n - 2);
        let vi = self.v0 + i as f64 * self.h;
        let vip1 = vi + self.h;
        let h = self.h;
        let (yi, yip1) = (self.y[i], self.y[i + 1]);
        let (mi, mip1) = (self.m[i], self.m[i + 1]);
        let a = vip1 - x;
        let b = x - vi;
        Some(
            mi * a.powi(3) / (6.0 * h)
                + mip1 * b.powi(3) / (6.0 * h)
                + (yi / h - mi * h / 6.0) * a
                + (yip1 / h - mip1 * h / 6.0) * b,
        )
    }
}

/// Cubic semi-Lagrangian `v`-advection: `f(x, v) -> f(x, v + E(x) dt)`, zero outside `[-vmax,
/// vmax]`. See the module docs for the natural-vs-not-a-knot spline note.
pub fn advect_v(f: &[f64], g: &Grid, e: &[f64], dt: f64) -> Vec<f64> {
    let (nx, nv) = (g.nx, g.nv);
    let v0 = g.v(0);
    let h = g.dv();
    let mut out = vec![0.0; nx * nv];
    for i in 0..nx {
        let row = &f[i * nv..(i + 1) * nv];
        let spline = NaturalSpline::new(v0, h, row);
        let shift = e[i] * dt;
        for j in 0..nv {
            let query = g.v(j) + shift;
            out[i * nv + j] = spline.eval(query).unwrap_or(0.0);
        }
    }
    out
}

/// One Strang split step.
pub fn step(f: &[f64], g: &Grid, dt: f64, field_on: bool) -> Vec<f64> {
    let f = advect_x(f, g, 0.5 * dt);
    let f = if field_on {
        let e = field_from(&f, g);
        advect_v(&f, g, &e, dt)
    } else {
        f
    };
    advect_x(&f, g, 0.5 * dt)
}

/// Complex Fourier amplitude `c_m` with `a(x) = sum_m c_m exp(i m k0 x)`; the cosine amplitude is
/// `2 Re c_m`.
pub fn mode_amplitude(a: &[f64], mode: usize) -> Complex64 {
    let n = a.len();
    let mut buf: Vec<Complex64> = a.iter().map(|&x| Complex64::new(x, 0.0)).collect();
    let mut planner = FftPlanner::new();
    planner.plan_fft_forward(n).process(&mut buf);
    buf[mode] / n as f64
}

/// `f * (1 + amp cos(mode * 2 pi / L * x))`, broadcast over `v`.
pub fn pulse(f: &[f64], g: &Grid, mode: usize, amp: f64) -> Vec<f64> {
    let (nx, nv) = (g.nx, g.nv);
    let mut out = vec![0.0; nx * nv];
    for i in 0..nx {
        let factor = 1.0 + amp * (mode as f64 * 2.0 * PI / g.l * g.x(i)).cos();
        for j in 0..nv {
            out[i * nv + j] = f[i * nv + j] * factor;
        }
    }
    out
}

pub fn energy(f: &[f64], g: &Grid) -> f64 {
    let (nx, nv) = (g.nx, g.nv);
    let dv = g.dv();
    let dx = g.dx();
    let mut kin = 0.0;
    for i in 0..nx {
        for j in 0..nv {
            kin += f[i * nv + j] * g.v(j).powi(2);
        }
    }
    kin *= 0.5 * dv * dx;
    let e = field_from(f, g);
    let field_energy: f64 = 0.5 * e.iter().map(|x| x * x).sum::<f64>() * dx;
    kin + field_energy
}

pub fn mass(f: &[f64], g: &Grid) -> f64 {
    f.iter().sum::<f64>() * g.dv() * g.dx()
}

/// Evolve `f` from `t=0` to `t_end` at fixed step `dt`, calling `observe(t, &f)` after every step
/// (and once at `t=0`) and applying `events` (each `(time, transform)`, sorted and applied once
/// `t` reaches it) in order. Returns the final state.
pub fn run<O: FnMut(f64, &[f64]), E: Fn(&[f64]) -> Vec<f64>>(
    mut f: Vec<f64>,
    g: &Grid,
    dt: f64,
    t_end: f64,
    mut observe: O,
    field_on: bool,
    mut events: Vec<(f64, E)>,
) -> Vec<f64> {
    let n = (t_end / dt).round() as usize;
    events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut pending = events;
    observe(0.0, &f);
    for j in 1..=n {
        f = step(&f, g, dt, field_on);
        let t = j as f64 * dt;
        while let Some((ev_t, _)) = pending.first() {
            if t >= *ev_t - 1e-12 {
                let (_, ev_fn) = pending.remove(0);
                f = ev_fn(&f);
            } else {
                break;
            }
        }
        observe(t, &f);
    }
    f
}

/// Damping rate and frequency from the local maxima of `|a(t)|` in `[t0, t1]`: `rate` is the
/// slope of `ln|a|` at the maxima (linear fit); `omega = pi / mean spacing of maxima`. Returns
/// `(rate, omega, peak_times)`.
pub fn peak_fit(t: &[f64], a: &[f64], t0: f64, t1: f64) -> (f64, f64, Vec<f64>) {
    let a: Vec<f64> = a.iter().map(|x| x.abs()).collect();
    let mut idx = Vec::new();
    for i in 1..a.len() - 1 {
        if a[i] > a[i - 1] && a[i] >= a[i + 1] && t[i] >= t0 && t[i] <= t1 {
            idx.push(i);
        }
    }
    let dt = t[1] - t[0];
    let mut tp = Vec::with_capacity(idx.len());
    let mut ap = Vec::with_capacity(idx.len());
    for &i in &idx {
        let (y0, y1, y2) = (a[i - 1].ln(), a[i].ln(), a[i + 1].ln());
        let d = 0.5 * (y0 - y2) / (y0 - 2.0 * y1 + y2);
        tp.push(t[i] + d * dt);
        ap.push(y1 - 0.25 * (y0 - y2) * d);
    }
    // Linear least-squares fit of ap vs tp: rate = slope.
    let n = tp.len() as f64;
    let mean_t = tp.iter().sum::<f64>() / n;
    let mean_a = ap.iter().sum::<f64>() / n;
    let (mut num, mut den) = (0.0, 0.0);
    for (ti, ai) in tp.iter().zip(&ap) {
        num += (ti - mean_t) * (ai - mean_a);
        den += (ti - mean_t).powi(2);
    }
    let rate = num / den;
    let mean_spacing: f64 = tp.windows(2).map(|w| w[1] - w[0]).sum::<f64>() / (tp.len() - 1) as f64;
    let omega = PI / mean_spacing;
    (rate, omega, tp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn landau_run(
        nv: usize,
        t_end: f64,
        field_on: bool,
        k: f64,
    ) -> (Grid, Vec<f64>, Vec<f64>, Vec<f64>) {
        let g = Grid::new(2.0 * PI / k, 32, nv, 6.0);
        let base: Vec<f64> = (0..g.nx * g.nv)
            .map(|idx| maxwellian(g.v(idx % g.nv), 0.0))
            .collect();
        let f0 = pulse(&base, &g, 1, 0.01);
        let mut ts = Vec::new();
        let mut rhos = Vec::new();
        let mut es = Vec::new();
        let observe = |t: f64, f: &[f64]| {
            ts.push(t);
            rhos.push(mode_amplitude(&density(f, &g), 1).norm());
            es.push(mode_amplitude(&field_from(f, &g), 1).norm());
        };
        run(
            f0,
            &g,
            0.1,
            t_end,
            observe,
            field_on,
            Vec::<(f64, fn(&[f64]) -> Vec<f64>)>::new(),
        );
        (g, ts, rhos, es)
    }

    #[test]
    fn landau_damping_rate_matches_certified_root() {
        // Certified root at k=0.5, from the Python kinetic.dispersion module (interval-Newton in
        // Arb ball arithmetic; matches Canosa 1973's -0.15336). Not reproduced here -- that
        // certification is arbitrary-precision ball arithmetic, a different kind of numerics
        // entirely; only the resulting reference values are used, to check this f64 PDE solver.
        const OMEGA_R: f64 = 1.415661888604536;
        const GAMMA: f64 = -0.153359466909605;

        let (_, t, _, e) = landau_run(128, 25.0, true, 0.5);
        let (rate, omega, _) = peak_fit(&t, &e, 5.0, 25.0);
        assert!(
            (rate - GAMMA).abs() / GAMMA.abs() < 0.02,
            "rate {rate} vs certified {GAMMA}"
        );
        assert!(
            (omega - OMEGA_R).abs() / OMEGA_R < 0.01,
            "omega {omega} vs certified {OMEGA_R}"
        );
    }

    #[test]
    fn recurrence_free_streaming_is_exact_and_field_shifts_it() {
        let (g, t, rho, _) = landau_run(32, 45.0, false, 0.5);
        let tr = g.recurrence_time(1);
        let in_window: Vec<usize> = (0..t.len())
            .filter(|&i| t[i] > 25.0 && t[i] < 45.0)
            .collect();
        let argmax = *in_window
            .iter()
            .max_by(|&&a, &&b| rho[a].partial_cmp(&rho[b]).unwrap())
            .unwrap();
        assert!(
            (t[argmax] - tr).abs() <= 0.1,
            "peak at {}, T_R = {tr}",
            t[argmax]
        );

        let closest = (0..t.len())
            .min_by(|&a, &b| (t[a] - tr).abs().partial_cmp(&(t[b] - tr).abs()).unwrap())
            .unwrap();
        assert!(
            (rho[closest] / rho[0] - 1.0).abs() < 2e-3,
            "recurrence amplitude ratio {}",
            rho[closest] / rho[0]
        );

        let (_, t2, _, e2) = landau_run(32, 45.0, true, 0.5);
        let in_window2: Vec<usize> = (0..t2.len())
            .filter(|&i| t2[i] > 25.0 && t2[i] < 45.0)
            .collect();
        let argmax2 = *in_window2
            .iter()
            .max_by(|&&a, &&b| e2[a].partial_cmp(&e2[b]).unwrap())
            .unwrap();
        assert!(
            t2[argmax2] > tr + 1.0,
            "field-driven peak at {} did not lag T_R = {tr}",
            t2[argmax2]
        );
    }

    #[test]
    fn ballistic_echo_matches_closed_form() {
        let (k0, tau, a) = (0.5, 10.0, 0.01);
        let g = Grid::new(2.0 * PI / k0, 32, 512, 6.0);
        let base: Vec<f64> = (0..g.nx * g.nv)
            .map(|idx| maxwellian(g.v(idx % g.nv), 0.0))
            .collect();
        let f0 = pulse(&base, &g, 1, a);

        let mut ts = Vec::new();
        let mut cs = Vec::new();
        let observe = |t: f64, f: &[f64]| {
            ts.push(t);
            cs.push(2.0 * mode_amplitude(&density(f, &g), 2).re);
        };
        let events = vec![(tau, move |f: &[f64]| pulse(f, &g, 3, a))];
        run(f0, &g, 0.1, 20.0, observe, false, events);

        let mut worst = 0.0f64;
        let mut formula_max = 0.0f64;
        let mut t_at_max = 0.0;
        let mut c_max = f64::NEG_INFINITY;
        for (&t, &c) in ts.iter().zip(&cs) {
            let s = 2.0 * k0 * t - 3.0 * k0 * tau;
            let formula = if t >= tau {
                0.5 * a * a * (-0.5 * s * s).exp()
            } else {
                0.0
            };
            worst = worst.max((c - formula).abs());
            formula_max = formula_max.max(formula);
            if c > c_max {
                c_max = c;
                t_at_max = t;
            }
        }
        assert!(
            worst < 1e-6 * formula_max,
            "worst diff {worst:.3e}, formula max {formula_max:.3e}"
        );
        assert!(
            (t_at_max - 15.0).abs() < 0.05,
            "echo peak at {t_at_max}, expected 15.0"
        );
    }
}
