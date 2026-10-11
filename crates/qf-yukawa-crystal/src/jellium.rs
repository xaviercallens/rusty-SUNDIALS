//! Two-dimensional one-component plasma with `1/r` interaction (classical 2D Wigner crystal, Riesz `s = 1`) and a
//! uniform neutralising background, in a periodic rectangle, by Ewald summation.
//!
//! Energy (with `A = Lx Ly`, `S(k) = sum_j exp(i k.x_j)`):
//! `E = 1/2 sum'_{i,j} erfc(a r_ij)/r_ij + (pi/A) sum_{k != 0} erfc(k/(2a))/k |S(k)|^2 - N a/sqrt(pi) - N^2 sqrt(pi)/(a A)`,
//! with the real-space sum over minimum images inside `rc` (requires `rc <= min(Lx, Ly)/2`).
//! The last term is what remains of the particle-background and background-background interactions after the
//! divergent `k = 0` parts cancel. This is the renormalized (jellium) energy of upstream's Section 8 for `s = 1`, up to
//! upstream's normalisation constant.

use crate::{Geometry, Relaxed, System};
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

/// Ewald-summed 1/r jellium in a periodic rectangle.
#[derive(Clone, Debug)]
pub struct Jellium {
    pub lx: f64,
    pub ly: f64,
    pub alpha: f64,
    pub rc: f64,
    kvecs: Vec<(f64, f64, f64)>,
}

impl Jellium {
    /// `kcut` bounds `|k|` in reciprocal space; `rc` bounds the real-space minimum-image sum.
    pub fn new(lx: f64, ly: f64, alpha: f64, rc: f64, kcut: f64) -> Self {
        assert!(
            rc <= 0.5 * lx.min(ly) + 1e-12,
            "rc must not exceed half the box"
        );
        let (mx, my) = (
            (kcut * lx / (2.0 * std::f64::consts::PI)).ceil() as i64,
            (kcut * ly / (2.0 * std::f64::consts::PI)).ceil() as i64,
        );
        let mut kvecs = Vec::new();
        // Half space (k and -k give the same |S|^2): m > 0, or m = 0 and n > 0; weight 2.
        for m in 0..=mx {
            for n in -my..=my {
                if m == 0 && n <= 0 {
                    continue;
                }
                let (kx, ky) = (
                    2.0 * std::f64::consts::PI * m as f64 / lx,
                    2.0 * std::f64::consts::PI * n as f64 / ly,
                );
                let k = (kx * kx + ky * ky).sqrt();
                if k < kcut {
                    let c = 2.0 * libm::erfc(k / (2.0 * alpha)) / k;
                    kvecs.push((kx, ky, c));
                }
            }
        }
        Jellium {
            lx,
            ly,
            alpha,
            rc,
            kvecs,
        }
    }

    /// The periodic geometry, for minimum-image distances and order analysis.
    pub fn geometry_system(&self) -> System {
        System {
            kappa: 0.0,
            rc: self.rc,
            geometry: Geometry::Periodic {
                lx: self.lx,
                ly: self.ly,
            },
        }
    }

    fn structure(&self, x: &[f64], kx: f64, ky: f64) -> (f64, f64) {
        let (mut re, mut im) = (0.0, 0.0);
        for j in 0..x.len() / 2 {
            let ph = kx * x[2 * j] + ky * x[2 * j + 1];
            re += ph.cos();
            im += ph.sin();
        }
        (re, im)
    }

    /// Total energy.
    pub fn energy(&self, x: &[f64]) -> f64 {
        let n = x.len() / 2;
        let geo = self.geometry_system();
        let area = self.lx * self.ly;
        let mut real = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = geo.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let r = (dx * dx + dy * dy).sqrt();
                if r < self.rc {
                    real += libm::erfc(self.alpha * r) / r;
                }
            }
        }
        let mut recip = 0.0;
        for &(kx, ky, c) in &self.kvecs {
            let (re, im) = self.structure(x, kx, ky);
            recip += c * (re * re + im * im);
        }
        recip *= std::f64::consts::PI / area;
        let nf = n as f64;
        real + recip
            - nf * self.alpha / std::f64::consts::PI.sqrt()
            - nf * nf * std::f64::consts::PI.sqrt() / (self.alpha * area)
    }

    /// Force `F = -grad E`.
    pub fn force(&self, x: &[f64], f: &mut [f64]) {
        let n = x.len() / 2;
        let geo = self.geometry_system();
        let area = self.lx * self.ly;
        f.iter_mut().for_each(|v| *v = 0.0);
        let c2 = 2.0 * self.alpha / std::f64::consts::PI.sqrt();
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = geo.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let r = (dx * dx + dy * dy).sqrt();
                if r < self.rc {
                    let mag = libm::erfc(self.alpha * r) / (r * r)
                        + c2 * (-(self.alpha * r).powi(2)).exp() / r;
                    let (fx, fy) = (mag * dx / r, mag * dy / r);
                    f[2 * i] += fx;
                    f[2 * i + 1] += fy;
                    f[2 * j] -= fx;
                    f[2 * j + 1] -= fy;
                }
            }
        }
        let pref = 2.0 * std::f64::consts::PI / area;
        for &(kx, ky, c) in &self.kvecs {
            let (sre, sim) = self.structure(x, kx, ky);
            for i in 0..n {
                let ph = kx * x[2 * i] + ky * x[2 * i + 1];
                // Im(conj(S) e^{i ph}) = sre sin(ph) - sim cos(ph)
                let im = sre * ph.sin() - sim * ph.cos();
                let g = pref * c * im;
                f[2 * i] += g * kx;
                f[2 * i + 1] += g * ky;
            }
        }
    }

    /// Hessian by central finite differences of the analytic force (symmetrised), row-major `2n x 2n`.
    pub fn hessian_fd(&self, x: &[f64], h: f64) -> Vec<f64> {
        let m = x.len();
        let mut hess = vec![0.0; m * m];
        let (mut fp, mut fm) = (vec![0.0; m], vec![0.0; m]);
        let mut xp = x.to_vec();
        for k in 0..m {
            xp[k] = x[k] + h;
            self.force(&xp, &mut fp);
            xp[k] = x[k] - h;
            self.force(&xp, &mut fm);
            xp[k] = x[k];
            for r in 0..m {
                hess[r * m + k] = -(fp[r] - fm[r]) / (2.0 * h);
            }
        }
        for a in 0..m {
            for b in (a + 1)..m {
                let s = 0.5 * (hess[a * m + b] + hess[b * m + a]);
                hess[a * m + b] = s;
                hess[b * m + a] = s;
            }
        }
        hess
    }

    fn wrap(&self, x: &mut [f64]) {
        for i in 0..x.len() / 2 {
            x[2 * i] = x[2 * i].rem_euclid(self.lx);
            x[2 * i + 1] = x[2 * i + 1].rem_euclid(self.ly);
        }
    }

    /// Overdamped relaxation with CVODE (BDF, finite-difference Jacobian) until `max |F| < ftol` or `t > tmax`.
    pub fn relax(&self, x0: &[f64], ftol: f64, tmax: f64) -> Result<Relaxed, String> {
        let me = self.clone();
        let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
            me.force(y, ydot);
            Ok(())
        };
        let mut solver = Cvode::builder(Method::Bdf)
            .rtol(1e-10)
            .atol(1e-12)
            .max_steps(2_000_000)
            .build(rhs, 0.0, SerialVector::from_slice(x0))
            .map_err(|e| format!("CVODE setup failed: {e}"))?;
        let mut f = vec![0.0; x0.len()];
        let mut tout = 1.0;
        loop {
            let (t, y) = solver
                .solve(tout, Task::Normal)
                .map_err(|e| format!("CVODE failed at t = {tout}: {e}"))?;
            let y = y.to_vec();
            self.force(&y, &mut f);
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

    /// Overdamped Langevin annealing (Euler-Maruyama), as `crate::anneal` but with the jellium force.
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
            self.force(&x, &mut f);
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

/// Literature Madelung energies (Hartree x r_s per electron): Bonsall & Maradudin, Phys. Rev. B 15, 1959 (1977), as
/// recalculated in arXiv:2103.01622. At density 1, `r_s = 1/sqrt(pi)`, so the energy per particle is `eta sqrt(pi)`.
pub const ETA_TRIANGULAR: f64 = -1.106102587;
pub const ETA_SQUARE: f64 = -1.100244420;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rng, spacing_density_one, square, triangular};

    fn tri() -> (Jellium, Vec<f64>) {
        let a = spacing_density_one();
        let (lx, ly) = (16.0 * a, 9.0 * 3.0_f64.sqrt() * a);
        (Jellium::new(lx, ly, 0.66, 8.0, 7.0), triangular(16, 18, a))
    }

    #[test]
    fn erfc_reference_values() {
        for (x, v) in [
            (0.5, 0.4795001221869535),
            (1.0, 0.15729920705028513),
            (3.0, 2.209049699858544e-05),
            (5.0, 1.537_459_794_428_035e-12),
        ] {
            assert!(
                (libm::erfc(x) - v).abs() <= 1e-15 * v.max(1e-300) * 10.0,
                "erfc({x})"
            );
        }
    }

    #[test]
    fn triangular_madelung_matches_literature() {
        let (j, x) = tri();
        let e = j.energy(&x) / 288.0;
        let lit = ETA_TRIANGULAR * std::f64::consts::PI.sqrt();
        assert!(((e - lit) / lit).abs() < 1e-8, "e = {e}, literature {lit}");
    }

    #[test]
    fn square_madelung_matches_literature() {
        let j = Jellium::new(17.0, 17.0, 0.66, 8.0, 7.0);
        let e = j.energy(&square(17, 1.0)) / 289.0;
        let lit = ETA_SQUARE * std::f64::consts::PI.sqrt();
        assert!(((e - lit) / lit).abs() < 1e-8, "e = {e}, literature {lit}");
    }

    #[test]
    fn energy_independent_of_alpha() {
        let (_, mut x) = tri();
        let a = spacing_density_one();
        let (lx, ly) = (16.0 * a, 9.0 * 3.0_f64.sqrt() * a);
        let mut rng = Rng::new(9);
        x.iter_mut().for_each(|v| *v += 0.2 * rng.gauss());
        // A 4x range of the splitting parameter, each with its k-space cutoff scaled so both truncations stay ~1e-13.
        let e: Vec<f64> = [0.66, 1.32, 2.64]
            .iter()
            .map(|&al| Jellium::new(lx, ly, al, 8.0, 10.6 * al).energy(&x))
            .collect();
        for v in &e[1..] {
            assert!(((v - e[0]) / e[0]).abs() < 1e-10, "{:?}", e);
        }
    }

    #[test]
    fn force_matches_finite_difference() {
        let (j, mut x) = tri();
        let mut rng = Rng::new(4);
        x.iter_mut().for_each(|v| *v += 0.1 * rng.gauss());
        let mut f = vec![0.0; x.len()];
        j.force(&x, &mut f);
        for &k in &[0usize, 77, 401] {
            let h = 1e-5;
            let (mut xp, mut xm) = (x.clone(), x.clone());
            xp[k] += h;
            xm[k] -= h;
            let fd = -(j.energy(&xp) - j.energy(&xm)) / (2.0 * h);
            assert!((fd - f[k]).abs() < 1e-6, "k={k} fd={fd} f={}", f[k]);
        }
    }
}
