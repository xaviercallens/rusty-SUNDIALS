//! Classical bilayer dipolar excitons: electron in one layer, hole in the other, interlayer separation `d`.
//!
//! The pair interaction of two such excitons (rigid, perpendicular, unscreened) is
//! `V(r) = 2 (1/r - 1/sqrt(r^2 + d^2)) = g_d(r^2)` with `g_d(t) = 2 (t^{-1/2} - (t + d^2)^{-1/2})`, the function whose
//! complete monotonicity and every-density triangular optimality are kernel-checked in AutoevolveAI
//! `formal_cert/Autoresearch/DipolarExcitonMultipolar.lean` (conditional on openai/math `universal_energy_minimum`).
//!
//! Everything here works with the rescaled potential `U = V / d^2 = 2 / (r s (r + s))`, `s = sqrt(r^2 + d^2)`, which has
//! the same minimisers, no cancellation for small `d`, and the pure dipolar limit `U -> 1/r^3` as `d -> 0`.
//! Lengths are in units of `n^{-1/2}` (density 1), so `d` is the dimensionless `d sqrt(n)`.
//!
//! Two versions of the energy are kept apart:
//! * the *dynamics* potential: `U` with a shifted-force cutoff at `rc` (continuous energy and force), minimum image,
//!   analytic Hessian; it is not completely monotone and only generates competitor configurations;
//! * the *full* potential: the untruncated `U`, evaluated as a periodic image sum to radius `R` plus the exact
//!   uniform-density tail `(n/2) int_R^inf 2 pi r U(r) dr = 2 pi n / (sqrt(R^2 + d^2) + R)` per particle. The tail
//!   is the same for every configuration at fixed density; what is left (lattice-minus-continuum beyond `R`) is
//!   measured by varying `R`.

use crate::{Relaxed, System};
use cvode::{Cvode, DenseMat, Method, Task};
use nvector::SerialVector;

/// Rescaled dipolar-exciton potential `U(r) = 2 / (r s (r + s))` and its first two derivatives.
pub fn u_pair(d: f64, r: f64) -> (f64, f64, f64) {
    let s = (r * r + d * d).sqrt();
    let den = r * s * (r + s);
    let dden = 2.0 * r * s + r * r * r / s + s * s + 2.0 * r * r;
    let d2den = 2.0 * s + 5.0 * r * r / s - r.powi(4) / s.powi(3) + 6.0 * r;
    let u = 2.0 / den;
    let du = -2.0 * dden / (den * den);
    let d2u = -2.0 * d2den / (den * den) + 4.0 * dden * dden / (den * den * den);
    (u, du, d2u)
}

/// Exact uniform-density tail per particle beyond radius `big_r` at density `rho`, in `U` units.
pub fn tail(d: f64, rho: f64, big_r: f64) -> f64 {
    2.0 * std::f64::consts::PI * rho / ((big_r * big_r + d * d).sqrt() + big_r)
}

/// Dipolar excitons in a periodic rectangle.
#[derive(Clone, Copy, Debug)]
pub struct Dipolar {
    pub d: f64,
    pub rc: f64,
    pub lx: f64,
    pub ly: f64,
}

impl Dipolar {
    pub fn new(d: f64, rc: f64, lx: f64, ly: f64) -> Self {
        assert!(d > 0.0, "d must be positive");
        assert!(
            rc <= 0.5 * lx.min(ly) + 1e-12,
            "rc must not exceed half the box (minimum image)"
        );
        Dipolar { d, rc, lx, ly }
    }

    /// The periodic geometry (for minimum-image distances, random starts and order analysis).
    pub fn geometry_system(&self) -> System {
        System {
            kappa: 0.0,
            rc: self.rc,
            geometry: crate::Geometry::Periodic {
                lx: self.lx,
                ly: self.ly,
            },
        }
    }

    /// Shifted-force dynamics potential and derivatives (zero for `r >= rc`).
    pub fn pair_sf(&self, r: f64) -> (f64, f64, f64) {
        if r >= self.rc {
            return (0.0, 0.0, 0.0);
        }
        let (uc, duc, _) = u_pair(self.d, self.rc);
        let (u, du, d2u) = u_pair(self.d, r);
        (u - uc - (r - self.rc) * duc, du - duc, d2u)
    }

    fn delta(&self, xi: f64, yi: f64, xj: f64, yj: f64) -> (f64, f64) {
        let (mut dx, mut dy) = (xi - xj, yi - yj);
        dx -= self.lx * (dx / self.lx).round();
        dy -= self.ly * (dy / self.ly).round();
        (dx, dy)
    }

    /// Total dynamics (shifted-force) energy.
    pub fn energy_sf(&self, x: &[f64]) -> f64 {
        let n = x.len() / 2;
        let mut e = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                e += self.pair_sf((dx * dx + dy * dy).sqrt()).0;
            }
        }
        e
    }

    /// Force `-grad E_sf`.
    pub fn force_sf(&self, x: &[f64], f: &mut [f64]) {
        let n = x.len() / 2;
        f.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let r = (dx * dx + dy * dy).sqrt();
                let (_, du, _) = self.pair_sf(r);
                if du != 0.0 {
                    let (fx, fy) = (-du * dx / r, -du * dy / r);
                    f[2 * i] += fx;
                    f[2 * i + 1] += fy;
                    f[2 * j] -= fx;
                    f[2 * j + 1] -= fy;
                }
            }
        }
    }

    /// Hessian of `E_sf` (dense, row-major `2n x 2n`).
    pub fn hessian_sf(&self, x: &[f64]) -> Vec<f64> {
        let n = x.len() / 2;
        let m = 2 * n;
        let mut h = vec![0.0; m * m];
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let r = (dx * dx + dy * dy).sqrt();
                if r >= self.rc {
                    continue;
                }
                let (_, du, d2u) = self.pair_sf(r);
                let u = [dx / r, dy / r];
                for a in 0..2 {
                    for b in 0..2 {
                        let id = if a == b { 1.0 } else { 0.0 };
                        let k = d2u * u[a] * u[b] + du / r * (id - u[a] * u[b]);
                        h[(2 * i + a) * m + 2 * i + b] += k;
                        h[(2 * j + a) * m + 2 * j + b] += k;
                        h[(2 * i + a) * m + 2 * j + b] -= k;
                        h[(2 * j + a) * m + 2 * i + b] -= k;
                    }
                }
            }
        }
        h
    }

    /// Full-potential energy per particle: image sum of the untruncated `U` over all pairs (including a particle's
    /// own periodic images) with `|r| < big_r`, plus the exact uniform tail beyond `big_r`.
    /// Returns `(sum part, tail part)`; the energy per particle is their sum.
    pub fn energy_full_parts(&self, x: &[f64], big_r: f64) -> (f64, f64) {
        let n = x.len() / 2;
        let px = (big_r / self.lx).ceil() as i64 + 1;
        let py = (big_r / self.ly).ceil() as i64 + 1;
        let r2max = big_r * big_r;
        let mut s = 0.0;
        for i in 0..n {
            for j in i..n {
                let (dx0, dy0) = self.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let mut sij = 0.0;
                for p in -px..=px {
                    let dx = dx0 + p as f64 * self.lx;
                    for q in -py..=py {
                        let dy = dy0 + q as f64 * self.ly;
                        let r2 = dx * dx + dy * dy;
                        if r2 >= r2max || (i == j && p == 0 && q == 0) {
                            continue;
                        }
                        sij += u_pair(self.d, r2.sqrt()).0;
                    }
                }
                // ordered pairs: (i, j) and (j, i) for i != j; i == j images counted once over all n
                s += if i == j { sij } else { 2.0 * sij };
            }
        }
        let rho = n as f64 / (self.lx * self.ly);
        (0.5 * s / n as f64, tail(self.d, rho, big_r))
    }

    /// Full-potential energy per particle (see [`Dipolar::energy_full_parts`]).
    pub fn energy_full(&self, x: &[f64], big_r: f64) -> f64 {
        let (a, b) = self.energy_full_parts(x, big_r);
        a + b
    }

    fn wrap(&self, x: &mut [f64]) {
        for i in 0..x.len() / 2 {
            x[2 * i] = x[2 * i].rem_euclid(self.lx);
            x[2 * i + 1] = x[2 * i + 1].rem_euclid(self.ly);
        }
    }

    /// Overdamped relaxation of the dynamics potential with CVODE (BDF, analytic Jacobian `-Hessian`) until
    /// `max |F| < ftol` or `t > tmax`.
    pub fn relax(&self, x0: &[f64], ftol: f64, tmax: f64) -> Result<Relaxed, String> {
        let (s1, s2) = (*self, *self);
        let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
            s1.force_sf(y, ydot);
            Ok(())
        };
        let jac = move |_t: f64, y: &[f64], j: &mut DenseMat| -> Result<(), String> {
            let h = s2.hessian_sf(y);
            let m = y.len();
            for c in 0..m {
                for r in 0..m {
                    j.cols[c][r] = -h[r * m + c];
                }
            }
            Ok(())
        };
        let mut solver = Cvode::builder(Method::Bdf)
            .rtol(1e-10)
            .atol(1e-12)
            .max_steps(2_000_000)
            .jacobian(jac)
            .build(rhs, 0.0, SerialVector::from_slice(x0))
            .map_err(|e| format!("CVODE setup failed: {e}"))?;
        let mut f = vec![0.0; x0.len()];
        let mut tout = 1.0;
        loop {
            let (t, y) = solver
                .solve(tout, Task::Normal)
                .map_err(|e| format!("CVODE failed at t = {tout}: {e}"))?;
            let y = y.to_vec();
            self.force_sf(&y, &mut f);
            let fmax = f.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
            if fmax < ftol || t >= tmax {
                let mut x = y;
                self.wrap(&mut x);
                return Ok(Relaxed {
                    x,
                    t,
                    max_force: fmax,
                    steps: solver.num_steps(),
                    rhs_evals: solver.num_rhs_evals(),
                    converged: fmax < ftol,
                });
            }
            tout *= 2.0;
        }
    }

    /// Overdamped Langevin annealing of the dynamics potential (Euler-Maruyama, geometric schedule `t0 -> t1`),
    /// each displacement capped at `cap`.
    #[allow(clippy::too_many_arguments)]
    pub fn anneal(
        &self,
        x0: &[f64],
        t0: f64,
        t1: f64,
        steps: usize,
        dt: f64,
        cap: f64,
        seed: u64,
    ) -> Vec<f64> {
        let mut rng = crate::Rng::new(seed);
        let mut x = x0.to_vec();
        let mut f = vec![0.0; x.len()];
        let ratio = (t1 / t0).powf(1.0 / steps as f64);
        let mut temp = t0;
        for _ in 0..steps {
            self.force_sf(&x, &mut f);
            let amp = (2.0 * temp * dt).sqrt();
            for k in 0..x.len() {
                x[k] += (f[k] * dt + amp * rng.gauss()).clamp(-cap, cap);
            }
            self.wrap(&mut x);
            temp *= ratio;
        }
        x
    }
}

/// Per-particle full-potential energy of an infinite Bravais lattice with basis vectors `b` (rows), by direct
/// summation over lattice vectors with `0 < |v| < big_r` plus the exact uniform tail.
pub fn lattice_energy_full(d: f64, b: [[f64; 2]; 2], big_r: f64) -> f64 {
    let area = (b[0][0] * b[1][1] - b[0][1] * b[1][0]).abs();
    let hmin = area / (b[0][0].hypot(b[0][1])).max(b[1][0].hypot(b[1][1]));
    let m = (big_r / hmin).ceil() as i64 + 2;
    let mut s = 0.0;
    for i in -m..=m {
        for j in -m..=m {
            if i == 0 && j == 0 {
                continue;
            }
            let vx = i as f64 * b[0][0] + j as f64 * b[1][0];
            let vy = i as f64 * b[0][1] + j as f64 * b[1][1];
            let r2 = vx * vx + vy * vy;
            if r2 < big_r * big_r {
                s += u_pair(d, r2.sqrt()).0;
            }
        }
    }
    0.5 * s + tail(d, 1.0 / area, big_r)
}

/// Per-particle full-potential energy of the honeycomb structure (two-point basis on a triangular Bravais lattice)
/// at density 1, by direct summation plus tail.
pub fn honeycomb_energy_full(d: f64, big_r: f64) -> f64 {
    // two particles per cell: cell area 2 -> triangular Bravais spacing ah with (sqrt3/2) ah^2 = 2
    let ah = (4.0 / 3.0_f64.sqrt()).sqrt();
    let b = [[ah, 0.0], [0.5 * ah, 0.5 * 3.0_f64.sqrt() * ah]];
    let shift = [(b[0][0] + b[1][0]) / 3.0, (b[0][1] + b[1][1]) / 3.0];
    let m = (big_r / (0.5 * 3.0_f64.sqrt() * ah)).ceil() as i64 + 2;
    let mut s = 0.0;
    for i in -m..=m {
        for j in -m..=m {
            let vx = i as f64 * b[0][0] + j as f64 * b[1][0];
            let vy = i as f64 * b[0][1] + j as f64 * b[1][1];
            // same sublattice
            let r2 = vx * vx + vy * vy;
            if (i != 0 || j != 0) && r2 < big_r * big_r {
                s += u_pair(d, r2.sqrt()).0;
            }
            // other sublattice
            let (wx, wy) = (vx + shift[0], vy + shift[1]);
            let w2 = wx * wx + wy * wy;
            if w2 < big_r * big_r {
                s += u_pair(d, w2.sqrt()).0;
            }
        }
    }
    0.5 * s + tail(d, 1.0, big_r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rng, spacing_density_one, triangular};

    fn tri(d: f64) -> (Dipolar, Vec<f64>) {
        let a = spacing_density_one();
        (
            Dipolar::new(d, 8.0, 16.0 * a, 9.0 * 3.0_f64.sqrt() * a),
            triangular(16, 18, a),
        )
    }

    #[test]
    fn u_matches_naive_formula_and_derivatives() {
        for &d in &[0.05_f64, 0.3, 1.0, 2.5] {
            for &r in &[0.4_f64, 1.0, 3.7] {
                let s = (r * r + d * d).sqrt();
                let naive = 2.0 * (1.0 / r - 1.0 / s) / (d * d);
                let (u, du, d2u) = u_pair(d, r);
                assert!((u - naive).abs() < 1e-9 * naive, "d={d} r={r}");
                let h = 1e-5;
                let fd1 = (u_pair(d, r + h).0 - u_pair(d, r - h).0) / (2.0 * h);
                let fd2 = (u_pair(d, r + h).1 - u_pair(d, r - h).1) / (2.0 * h);
                assert!(
                    (fd1 - du).abs() < 1e-6 * du.abs().max(1.0),
                    "du d={d} r={r}"
                );
                assert!(
                    (fd2 - d2u).abs() < 1e-6 * d2u.abs().max(1.0),
                    "d2u d={d} r={r}"
                );
            }
        }
        // pure dipolar limit
        let (u, _, _) = u_pair(1e-6, 2.0);
        assert!((u - 1.0 / 8.0).abs() < 1e-10);
    }

    #[test]
    fn force_and_hessian_match_finite_differences() {
        let (sys, mut x) = tri(0.3);
        let mut rng = Rng::new(7);
        x.iter_mut().for_each(|v| *v += 0.05 * rng.gauss());
        let m = x.len();
        let mut f = vec![0.0; m];
        sys.force_sf(&x, &mut f);
        let hess = sys.hessian_sf(&x);
        for &k in &[0usize, 17, 301] {
            let h = 1e-6;
            let (mut xp, mut xm) = (x.clone(), x.clone());
            xp[k] += h;
            xm[k] -= h;
            let fd = -(sys.energy_sf(&xp) - sys.energy_sf(&xm)) / (2.0 * h);
            assert!((fd - f[k]).abs() < 1e-5, "k={k} fd={fd} f={}", f[k]);
            let (mut fp, mut fm) = (vec![0.0; m], vec![0.0; m]);
            sys.force_sf(&xp, &mut fp);
            sys.force_sf(&xm, &mut fm);
            for r in [k, k + 1, 50] {
                let fdh = -(fp[r] - fm[r]) / (2.0 * h);
                assert!((fdh - hess[r * m + k]).abs() < 1e-4, "r={r} k={k}");
            }
        }
    }

    #[test]
    fn periodic_box_full_energy_equals_lattice_sum() {
        let a = spacing_density_one();
        let b = [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]];
        for &d in &[0.066, 1.0] {
            let (sys, x) = tri(d);
            let e_box = sys.energy_full(&x, 24.0);
            let e_lat = lattice_energy_full(d, b, 24.0);
            assert!((e_box - e_lat).abs() < 1e-11 * e_lat, "{e_box} vs {e_lat}");
            assert!(e_lat > 0.0);
        }
    }

    #[test]
    fn full_energy_converges_in_r_and_tail_matters() {
        let a = spacing_density_one();
        let b = [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]];
        let d = 0.066;
        let e32 = lattice_energy_full(d, b, 32.0);
        let e128 = lattice_energy_full(d, b, 128.0);
        assert!((e32 - e128).abs() < 1e-4 * e128, "{e32} vs {e128}");
        // without the tail the difference is the size of the tail itself
        let t32 = tail(d, 1.0, 32.0);
        assert!(t32 > 100.0 * (e32 - e128).abs());
    }

    #[test]
    fn square_and_honeycomb_cost_more_than_triangular() {
        let a = spacing_density_one();
        let b = [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]];
        for &d in &[0.05, 1.0] {
            let e_tri = lattice_energy_full(d, b, 64.0);
            let e_sq = lattice_energy_full(d, [[1.0, 0.0], [0.0, 1.0]], 64.0);
            let e_hc = honeycomb_energy_full(d, 64.0);
            assert!(e_sq > e_tri && e_hc > e_sq, "d={d}: {e_tri} {e_sq} {e_hc}");
        }
    }
}
