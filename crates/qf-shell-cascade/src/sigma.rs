//! The `sigma`-regulated conservative shell model and its second invariant, integrated with this
//! repository's CVODE (BDF).
//!
//! # The model
//!
//! Shells `n = 0..=n_max` (`M = n_max + 1` shells), `k_n = 2^n`, truncation `v_{n_max+1} = 0`:
//!
//! ```text
//! dv_n/dt = k_{n-1} v_{n-1}^2 - k_n conj(v_n) v_{n+1} - i omega_n v_n,   omega_n = D k_n^sigma
//! ```
//!
//! This is [`crate::nonlinear_conj`] plus a dispersive rotation with exponent `sigma` (the
//! crate's [`crate::quantum_pressure`] is the `sigma = 2` case). It matches the Lean model `F k
//! omega v n = shellBc k n v - I * omega n * v n` of AutoevolveAI's
//! `results/autoresearch/2026-10-25/H7/contribution/Autoresearch_EnstrophyFlow.lean` with
//! Lean's `N + 2` shells equal to `M` here (Lean `N = n_max - 1`).
//!
//! # Invariants monitored
//!
//! * `S = sum |v_n|^2` (`E = S/2`).
//! * `H_sigma = sum_{n<M} 2^{-n} D k_n^sigma |v_n|^2 + sum_{n<M-1} 2^{-n} k_n Im(conj(v_n)^2
//!   v_{n+1})`. For `sigma = 3` its quadratic part is exactly `2 D Omega`, with `Omega = 1/2 sum
//!   k_n^2 |v_n|^2`, which is what gives the Lean bound `2 D Omega(t) <= H_3(0) + S(0)^{3/2}`
//!   along solutions.
//!
//! # Scope
//!
//! Floating-point exploration (Tier X): it illustrates and stress-tests the kernel-checked
//! statement on trajectories; it does not support it.

use crate::k_shells;
use cvode::{Cvode, Method, Task};
use num_complex::Complex64;
use nvector::SerialVector;

/// Parameters of one `sigma`-regulated run.
#[derive(Debug, Clone, Copy)]
pub struct SigmaModel {
    /// Highest shell index (`M = n_max + 1` shells).
    pub n_max: usize,
    /// Dispersion strength `D`.
    pub d: f64,
    /// Dispersion exponent `sigma` (`omega_n = D k_n^sigma`).
    pub sigma: f64,
    /// Optional negative-control defect: the outgoing coupling `k_j` in `B_j` is multiplied by
    /// `1 + eps` (the matching incoming coupling of shell `j + 1` is left alone), which breaks
    /// the exact cancellation behind both invariants.
    pub perturb: Option<(usize, f64)>,
}

impl SigmaModel {
    /// Dispersion frequencies `omega_n = D k_n^sigma`.
    pub fn omega(&self) -> Vec<f64> {
        k_shells(self.n_max)
            .iter()
            .map(|&k| self.d * k.powf(self.sigma))
            .collect()
    }

    /// Right-hand side on complex amplitudes.
    pub fn rhs(&self, v: &[Complex64], out: &mut [Complex64]) {
        let m = self.n_max + 1;
        assert_eq!(v.len(), m, "state length {} != {}", v.len(), m);
        assert_eq!(out.len(), m, "output length {} != {}", out.len(), m);
        let i = Complex64::new(0.0, 1.0);
        let zero = Complex64::new(0.0, 0.0);
        for n in 0..m {
            let kn = 2f64.powi(n as i32);
            let incoming = if n >= 1 {
                0.5 * kn * v[n - 1] * v[n - 1]
            } else {
                zero
            };
            let mut kout = kn;
            if let Some((j, eps)) = self.perturb
                && j == n
            {
                kout *= 1.0 + eps;
            }
            let outgoing = if n + 1 < m {
                kout * v[n].conj() * v[n + 1]
            } else {
                zero
            };
            let omega = self.d * kn.powf(self.sigma);
            out[n] = incoming - outgoing - i * omega * v[n];
        }
    }

    /// Right-hand side on the packed real state `[Re v_0, Im v_0, Re v_1, ...]`.
    pub fn rhs_real(&self, y: &[f64], ydot: &mut [f64]) {
        let v = unpack(y);
        let mut out = vec![Complex64::new(0.0, 0.0); v.len()];
        self.rhs(&v, &mut out);
        for (n, z) in out.iter().enumerate() {
            ydot[2 * n] = z.re;
            ydot[2 * n + 1] = z.im;
        }
    }

    /// Analytical Jacobian of [`Self::rhs_real`], column-major `cols[j][i] = d ydot_i / d y_j`.
    pub fn jacobian_real(&self, y: &[f64], cols: &mut [Vec<f64>]) {
        let m = self.n_max + 1;
        for c in cols.iter_mut() {
            for e in c.iter_mut() {
                *e = 0.0;
            }
        }
        let v = unpack(y);
        // f_n = 1/2 k_n v_{n-1}^2 - kout_n conj(v_n) v_{n+1} - i w_n v_n  (k_{n-1} = k_n / 2).
        // For a holomorphic-in-z term g(z) = c z: d/dx -> c, d/dy -> i c.
        // For an antiholomorphic factor conj(z): d/dx conj(z) = 1, d/dy conj(z) = -i.
        let i = Complex64::new(0.0, 1.0);
        let mut put = |row: usize, col_z: usize, dx: Complex64, dy: Complex64| {
            cols[2 * col_z][2 * row] += dx.re;
            cols[2 * col_z][2 * row + 1] += dx.im;
            cols[2 * col_z + 1][2 * row] += dy.re;
            cols[2 * col_z + 1][2 * row + 1] += dy.im;
        };
        for n in 0..m {
            let kn = 2f64.powi(n as i32);
            if n >= 1 {
                let c = kn * v[n - 1]; // d/dz (1/2 k_n z^2) = k_n z
                put(n, n - 1, c, i * c);
            }
            let mut kout = kn;
            if let Some((j, eps)) = self.perturb
                && j == n
            {
                kout *= 1.0 + eps;
            }
            if n + 1 < m {
                // -kout conj(v_n) v_{n+1}
                let a = -kout * v[n + 1]; // coefficient of conj(v_n)
                put(n, n, a, -i * a);
                let b = -kout * v[n].conj(); // coefficient of v_{n+1}
                put(n, n + 1, b, i * b);
            }
            let w = -i * self.d * kn.powf(self.sigma);
            put(n, n, w, i * w);
        }
    }
}

/// Packed real state to complex amplitudes.
pub fn unpack(y: &[f64]) -> Vec<Complex64> {
    y.as_chunks::<2>()
        .0
        .iter()
        .map(|&[re, im]| Complex64::new(re, im))
        .collect()
}

/// Complex amplitudes to packed real state.
pub fn pack(v: &[Complex64]) -> Vec<f64> {
    v.iter().flat_map(|z| [z.re, z.im]).collect()
}

/// `S = sum |v_n|^2`.
pub fn mass(v: &[Complex64]) -> f64 {
    v.iter().map(|z| z.norm_sqr()).sum()
}

/// `Omega = 1/2 sum k_n^2 |v_n|^2`.
pub fn enstrophy(v: &[Complex64]) -> f64 {
    0.5 * v
        .iter()
        .enumerate()
        .map(|(n, z)| 4f64.powi(n as i32) * z.norm_sqr())
        .sum::<f64>()
}

/// The cubic part `C = sum_{n<M-1} 2^{-n} k_n Im(conj(v_n)^2 v_{n+1})` (`2^{-n} k_n = 1`, kept
/// literally as in the Lean `ham3`).
pub fn cubic(v: &[Complex64]) -> f64 {
    (0..v.len().saturating_sub(1))
        .map(|n| {
            let w = 0.5f64.powi(n as i32) * 2f64.powi(n as i32);
            w * (v[n].conj() * v[n].conj() * v[n + 1]).im
        })
        .sum()
}

/// The quadratic part `Q = sum_{n<M} 2^{-n} D k_n^sigma |v_n|^2`.
pub fn quadratic(v: &[Complex64], d: f64, sigma: f64) -> f64 {
    v.iter()
        .enumerate()
        .map(|(n, z)| 0.5f64.powi(n as i32) * d * 2f64.powi(n as i32).powf(sigma) * z.norm_sqr())
        .sum()
}

/// `H_sigma = Q + C`.
pub fn ham(v: &[Complex64], d: f64, sigma: f64) -> f64 {
    quadratic(v, d, sigma) + cubic(v)
}

/// First variation of `H_sigma` at `v` in direction `w` (the Lean `dHam`, product rule written
/// out): `sum 2 q_n Re(conj(v_n) w_n) + sum g_n Im(2 conj(v_n) conj(w_n) v_{n+1} + conj(v_n)^2
/// w_{n+1})` with `q_n = 2^{-n} D k_n^sigma`, `g_n = 2^{-n} k_n`.
pub fn ham_rate(v: &[Complex64], w: &[Complex64], d: f64, sigma: f64) -> f64 {
    assert_eq!(v.len(), w.len(), "shape mismatch");
    let mut r = 0.0;
    for n in 0..v.len() {
        let q = 0.5f64.powi(n as i32) * d * 2f64.powi(n as i32).powf(sigma);
        r += 2.0 * q * (v[n].conj() * w[n]).re;
        if n + 1 < v.len() {
            let g = 0.5f64.powi(n as i32) * 2f64.powi(n as i32);
            r += g
                * (2.0 * v[n].conj() * w[n].conj() * v[n + 1]
                    + v[n].conj() * v[n].conj() * w[n + 1])
                    .im;
        }
    }
    r
}

/// The scale `|Q| + sum |cubic terms|` against which `H` drift is measured (a relative drift of
/// `H` alone is meaningless when `Q` and `C` nearly cancel).
pub fn ham_scale(v: &[Complex64], d: f64, sigma: f64) -> f64 {
    let c_abs: f64 = (0..v.len().saturating_sub(1))
        .map(|n| (v[n].conj() * v[n].conj() * v[n + 1]).im.abs())
        .sum();
    quadratic(v, d, sigma).abs() + c_abs
}

/// Integration settings.
#[derive(Debug, Clone, Copy)]
pub struct CvodeSettings {
    pub rtol: f64,
    pub atol: f64,
    pub max_steps: usize,
    /// Use the analytical Jacobian (otherwise CVODE's finite differences).
    pub analytic_jacobian: bool,
    /// Keep every `trace_every`-th internal step in the trace.
    pub trace_every: usize,
}

/// One trajectory's monitored record.
#[derive(Debug, Clone)]
pub struct SigmaRun {
    pub t_reached: f64,
    pub steps: usize,
    pub rhs_evals: usize,
    pub s0: f64,
    pub h0: f64,
    pub h_scale0: f64,
    pub omega0: f64,
    /// `H(0) + S(0)^{3/2}`: the right-hand side of the Lean bound (with `H = H_sigma`).
    pub bound_rhs: f64,
    pub max_rel_ds: f64,
    /// `max |H(t) - H(0)| / h_scale0`.
    pub max_rel_dh: f64,
    pub sup_two_d_omega: f64,
    pub sup_omega: f64,
    /// `sup_t 2 D Omega(t) / bound_rhs` (<= 1 is the Lean bound for `sigma = 3`).
    pub sup_ratio: f64,
    /// `(t, S, H, Omega)` every `trace_every` internal steps (and the last step).
    pub trace: Vec<(f64, f64, f64, f64)>,
}

/// Integrate the `sigma` model from `v0` to `t_end` with CVODE BDF in one-step mode, evaluating
/// the invariants at every internal step (no interpolation).
pub fn integrate_cvode(
    model: SigmaModel,
    v0: &[Complex64],
    t_end: f64,
    set: CvodeSettings,
) -> Result<SigmaRun, String> {
    if v0.len() != model.n_max + 1 {
        return Err(format!(
            "v0 has length {}, expected {}",
            v0.len(),
            model.n_max + 1
        ));
    }
    let (d, sigma) = (model.d, model.sigma);
    let s0 = mass(v0);
    let h0 = ham(v0, d, sigma);
    let h_scale0 = ham_scale(v0, d, sigma);
    let omega0 = enstrophy(v0);
    let bound_rhs = h0 + s0.powf(1.5);
    let m_rhs = model;
    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        m_rhs.rhs_real(y, ydot);
        Ok(())
    };
    let mut builder = Cvode::builder(Method::Bdf)
        .rtol(set.rtol)
        .atol(set.atol)
        .max_steps(set.max_steps);
    if set.analytic_jacobian {
        let m_jac = model;
        builder = builder.jacobian(move |_t, y, j| {
            m_jac.jacobian_real(y, &mut j.cols);
            Ok(())
        });
    }
    let mut solver = builder
        .build(rhs, 0.0, SerialVector::from_slice(&pack(v0)))
        .map_err(|e| format!("CVODE setup failed: {e}"))?;

    let mut run = SigmaRun {
        t_reached: 0.0,
        steps: 0,
        rhs_evals: 0,
        s0,
        h0,
        h_scale0,
        omega0,
        bound_rhs,
        max_rel_ds: 0.0,
        max_rel_dh: 0.0,
        sup_two_d_omega: 2.0 * d * omega0,
        sup_omega: omega0,
        sup_ratio: 2.0 * d * omega0 / bound_rhs,
        trace: vec![(0.0, s0, h0, omega0)],
    };
    let mut nstep = 0usize;
    loop {
        let (t, y) = solver
            .solve(t_end, Task::OneStep)
            .map_err(|e| format!("CVODE step failed at t = {}: {e}", run.t_reached))?;
        let v = unpack(y);
        nstep += 1;
        if !v.iter().all(|z| z.re.is_finite() && z.im.is_finite()) {
            return Err(format!("non-finite state at t = {t}"));
        }
        let s = mass(&v);
        let h = ham(&v, d, sigma);
        let om = enstrophy(&v);
        run.max_rel_ds = run.max_rel_ds.max((s - s0).abs() / s0);
        run.max_rel_dh = run.max_rel_dh.max((h - h0).abs() / h_scale0);
        run.sup_omega = run.sup_omega.max(om);
        run.sup_two_d_omega = run.sup_two_d_omega.max(2.0 * d * om);
        run.sup_ratio = run.sup_ratio.max(2.0 * d * om / bound_rhs);
        run.t_reached = t;
        let last = t >= t_end;
        if last || (set.trace_every != 0 && nstep.is_multiple_of(set.trace_every)) {
            run.trace.push((t, s, h, om));
        }
        if last {
            break;
        }
        if nstep >= set.max_steps {
            return Err(format!(
                "reached max_steps = {} at t = {t} < t_end = {t_end}",
                set.max_steps
            ));
        }
    }
    run.steps = solver.num_steps();
    run.rhs_evals = solver.num_rhs_evals();
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(m: usize, seed: u64) -> Vec<Complex64> {
        let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            ((x >> 11) as f64 / (1u64 << 53) as f64) - 0.5
        };
        (0..m)
            .map(|n| Complex64::new(next(), next()) * 0.5f64.powi(n as i32))
            .collect()
    }

    fn rates(model: &SigmaModel, v: &[Complex64]) -> (f64, f64) {
        // dS/dt = 2 sum Re(conj v dv); dH/dt = dHam(v, dv), normalised by the size of its terms.
        let mut dv = vec![Complex64::new(0.0, 0.0); v.len()];
        model.rhs(v, &mut dv);
        let ds: f64 = 2.0
            * v.iter()
                .zip(&dv)
                .map(|(a, b)| (a.conj() * b).re)
                .sum::<f64>();
        let dh = ham_rate(v, &dv, model.d, model.sigma);
        (ds, dh)
    }

    #[test]
    fn ham_rate_matches_central_difference() {
        let v = sample(5, 21);
        let w = sample(5, 22);
        let (d, s) = (0.07, 3.0);
        let h = 1e-6;
        let vp: Vec<_> = v.iter().zip(&w).map(|(a, b)| a + h * b).collect();
        let vm: Vec<_> = v.iter().zip(&w).map(|(a, b)| a - h * b).collect();
        let fd = (ham(&vp, d, s) - ham(&vm, d, s)) / (2.0 * h);
        let exact = ham_rate(&v, &w, d, s);
        assert!(
            (fd - exact).abs() < 1e-8 * (1.0 + exact.abs()),
            "{fd} vs {exact}"
        );
        assert!(exact.abs() > 1e-6, "direction chosen gives a trivial rate");
    }

    #[test]
    fn sigma2_dispersion_matches_crate_quantum_pressure() {
        let model = SigmaModel {
            n_max: 6,
            d: 0.03,
            sigma: 2.0,
            perturb: None,
        };
        let v = sample(7, 11);
        let mut ours = vec![Complex64::new(0.0, 0.0); 7];
        model.rhs(&v, &mut ours);
        let crate_rhs = crate::rhs(&v, &k_shells(6), 0.0, 0.03);
        for (a, b) in ours.iter().zip(&crate_rhs) {
            assert!((a - b).norm() < 1e-14, "{a} vs {b}");
        }
    }

    #[test]
    fn sigma3_rates_vanish_and_perturbed_coupling_does_not() {
        let good = SigmaModel {
            n_max: 6,
            d: 0.05,
            sigma: 3.0,
            perturb: None,
        };
        let bad = SigmaModel {
            perturb: Some((2, 0.1)),
            ..good
        };
        let mut worst_good = 0.0f64;
        let mut least_bad = f64::INFINITY;
        for seed in 0..50 {
            let v = sample(7, seed);
            let (ds, dh) = rates(&good, &v);
            worst_good = worst_good.max(ds.abs()).max(dh.abs());
            let (ds_b, _) = rates(&bad, &v);
            least_bad = least_bad.min(ds_b.abs());
        }
        assert!(worst_good < 1e-7, "sigma=3 rates {worst_good:.3e}");
        assert!(least_bad > 1e-6, "perturbed coupling still conserves S");
    }

    #[test]
    fn analytic_jacobian_matches_finite_differences() {
        let model = SigmaModel {
            n_max: 4,
            d: 0.2,
            sigma: 3.0,
            perturb: Some((1, 0.3)),
        };
        let y = pack(&sample(5, 3));
        let n = y.len();
        let mut cols = vec![vec![0.0; n]; n];
        model.jacobian_real(&y, &mut cols);
        let mut f0 = vec![0.0; n];
        let mut f1 = vec![0.0; n];
        let mut worst = 0.0f64;
        for j in 0..n {
            let h = 1e-7;
            let mut yp = y.clone();
            yp[j] += h;
            let mut ym = y.clone();
            ym[j] -= h;
            model.rhs_real(&yp, &mut f1);
            model.rhs_real(&ym, &mut f0);
            for i in 0..n {
                let fd = (f1[i] - f0[i]) / (2.0 * h);
                worst = worst.max((fd - cols[j][i]).abs());
            }
        }
        assert!(worst < 1e-6, "Jacobian mismatch {worst:.3e}");
    }

    #[test]
    fn cvode_conserves_sigma3_invariants_and_respects_the_bound() {
        let model = SigmaModel {
            n_max: 4,
            d: 0.05,
            sigma: 3.0,
            perturb: None,
        };
        let v0 = sample(5, 7)
            .into_iter()
            .map(|z| z * 4.0)
            .collect::<Vec<_>>();
        let set = CvodeSettings {
            rtol: 1e-10,
            atol: 1e-13,
            max_steps: 2_000_000,
            analytic_jacobian: true,
            trace_every: 0,
        };
        let run = integrate_cvode(model, &v0, 2.0, set).unwrap();
        assert!(run.t_reached >= 2.0);
        assert!(run.max_rel_ds < 1e-6, "S drift {:.3e}", run.max_rel_ds);
        assert!(run.max_rel_dh < 1e-6, "H drift {:.3e}", run.max_rel_dh);
        assert!(run.sup_ratio <= 1.0, "bound ratio {}", run.sup_ratio);
    }
}
