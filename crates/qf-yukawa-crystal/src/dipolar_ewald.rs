//! Exact (Ewald-summed) energy of periodic dipolar-exciton configurations, for the untruncated potential.
//!
//! Plain spherical truncation of a `d^2/r^3` tail leaves a lattice-point-counting residual that decays only
//! algebraically and oscillates with the radius (measured in `references.jsonl`: about 4e-5 at R = 32..256), which
//! is far larger than the gaps one wants to resolve. This module removes it with an Ewald split of the integral
//! representation
//! `V(r) = 2 (1/r - 1/s) = (4/sqrt(pi)) int_0^inf exp(-r^2 t^2) (1 - exp(-d^2 t^2)) dt`, `s = sqrt(r^2 + d^2)`,
//! at `t = eta`:
//! * short range: `V_s(r) = 2 (erfc(eta r)/r - erfc(eta s)/s)` (Gaussian decay in `r`), summed over images in real space;
//! * long range, by the 2D Poisson formula `sum_n exp(-|r+n|^2 t^2) = (pi/(A t^2)) sum_G exp(-G^2/(4t^2)) cos(G.r)`:
//!   `(1/A) sum_G K(G) |S(G)|^2` with `K(G) = 4 sqrt(pi) int_0^eta t^-2 exp(-G^2/(4t^2)) (1 - exp(-d^2 t^2)) dt`,
//!   `S(G) = sum_j exp(i G.x_j)`, including `G = 0` (the potential is integrable, no neutralising background);
//! * minus the self term `V_l(0) = (4/sqrt(pi)) int_0^eta (1 - exp(-d^2 t^2)) dt` per particle.
//!
//! The `t`-integrals are done by composite Gauss-Legendre quadrature with `-expm1(-d^2 t^2)` (no cancellation).
//! Energies are returned in `U = V/d^2` units, per particle. The result must not depend on `eta` (tested).

/// Gauss-Legendre nodes and weights on [-1, 1] (Newton iteration on Legendre polynomials).
pub fn gauss_legendre(n: usize) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut x = (std::f64::consts::PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
        let mut dp = 0.0;
        for _ in 0..100 {
            let (mut p0, mut p1) = (1.0, x);
            for k in 2..=n {
                let p2 = ((2 * k - 1) as f64 * x * p1 - (k - 1) as f64 * p0) / k as f64;
                p0 = p1;
                p1 = p2;
            }
            dp = n as f64 * (x * p1 - p0) / (x * x - 1.0);
            let dx = p1 / dp;
            x -= dx;
            if dx.abs() < 1e-16 {
                break;
            }
        }
        out.push((x, 2.0 / ((1.0 - x * x) * dp * dp)));
    }
    out
}

/// `int_0^eta f(t) dt` by `panels` panels of `nodes`-point Gauss-Legendre.
fn quad<F: Fn(f64) -> f64>(f: F, eta: f64, gl: &[(f64, f64)], panels: usize) -> f64 {
    let h = eta / panels as f64;
    let mut s = 0.0;
    for p in 0..panels {
        let (lo, hi) = (p as f64 * h, (p + 1) as f64 * h);
        let (c, w) = (0.5 * (lo + hi), 0.5 * (hi - lo));
        for &(x, wt) in gl {
            s += wt * w * f(c + w * x);
        }
    }
    s
}

/// Ewald sum for `N` particles in the periodic cell spanned by `a1`, `a2`.
#[derive(Clone, Debug)]
pub struct DipolarEwald {
    pub d: f64,
    pub eta: f64,
    pub a1: [f64; 2],
    pub a2: [f64; 2],
    pub r_real: f64,
    area: f64,
    gvecs: Vec<(f64, f64, f64)>,
    k0: f64,
    vl0: f64,
}

impl DipolarEwald {
    /// `eta` sets the split; real space is summed to `r_real = 6.5/eta`, reciprocal space to `|G| < 13 eta`.
    pub fn new(d: f64, eta: f64, a1: [f64; 2], a2: [f64; 2]) -> Self {
        let area = (a1[0] * a2[1] - a1[1] * a2[0]).abs();
        let gl = gauss_legendre(40);
        let panels = 8;
        let sqpi = std::f64::consts::PI.sqrt();
        let one_minus = |t: f64| -(-(d * d * t * t)).exp_m1();
        // reciprocal basis: b_i . a_j = 2 pi delta_ij
        let det = a1[0] * a2[1] - a1[1] * a2[0];
        let tp = 2.0 * std::f64::consts::PI / det;
        let b1 = [tp * a2[1], -tp * a2[0]];
        let b2 = [-tp * a1[1], tp * a1[0]];
        let gcut = 13.0 * eta;
        let hb = area / (a1[0].hypot(a1[1])).max(a2[0].hypot(a2[1])); // smallest lattice-plane spacing (lower bound)
        let m = (gcut * hb.max(a1[0].hypot(a1[1])).max(a2[0].hypot(a2[1]))
            / (2.0 * std::f64::consts::PI))
            .ceil() as i64
            + 2;
        let mut gvecs = Vec::new();
        for i in -m..=m {
            for j in -m..=m {
                // half space (G and -G give the same |S|^2), weight 2
                if i < 0 || (i == 0 && j <= 0) {
                    continue;
                }
                let gx = i as f64 * b1[0] + j as f64 * b2[0];
                let gy = i as f64 * b1[1] + j as f64 * b2[1];
                let g2 = gx * gx + gy * gy;
                if g2.sqrt() >= gcut {
                    continue;
                }
                let a = 0.25 * g2;
                let k = 4.0
                    * sqpi
                    * quad(
                        |t| {
                            if t <= 0.0 {
                                0.0
                            } else {
                                (-a / (t * t)).exp() * one_minus(t) / (t * t)
                            }
                        },
                        eta,
                        &gl,
                        panels,
                    );
                gvecs.push((gx, gy, 2.0 * k));
            }
        }
        let k0 = 4.0
            * sqpi
            * quad(
                |t| {
                    if t <= 0.0 {
                        d * d
                    } else {
                        one_minus(t) / (t * t)
                    }
                },
                eta,
                &gl,
                panels,
            );
        let vl0 = 4.0 / sqpi * quad(one_minus, eta, &gl, panels);
        DipolarEwald {
            d,
            eta,
            a1,
            a2,
            r_real: 6.5 / eta,
            area,
            gvecs,
            k0,
            vl0,
        }
    }

    /// Short-range part `V_s(r)`.
    fn vs(&self, r: f64) -> f64 {
        let s = (r * r + self.d * self.d).sqrt();
        2.0 * (libm::erfc(self.eta * r) / r - libm::erfc(self.eta * s) / s)
    }

    /// Energy per particle of the periodic configuration `x` (flat `[x0, y0, x1, y1, ...]`), in `U = V/d^2` units.
    pub fn energy_per_particle(&self, x: &[f64]) -> f64 {
        let n = x.len() / 2;
        let det = self.a1[0] * self.a2[1] - self.a1[1] * self.a2[0];
        // fractional coordinates
        let frac = |dx: f64, dy: f64| -> (f64, f64) {
            (
                (dx * self.a2[1] - dy * self.a2[0]) / det,
                (-dx * self.a1[1] + dy * self.a1[0]) / det,
            )
        };
        let l1 = self.a1[0].hypot(self.a1[1]);
        let l2 = self.a2[0].hypot(self.a2[1]);
        let h = self.area / l1.max(l2);
        let m = (self.r_real / h).ceil() as i64 + 1;
        let rr2 = self.r_real * self.r_real;
        let mut real = 0.0;
        for i in 0..n {
            for j in i..n {
                let (mut fx, mut fy) = frac(x[2 * j] - x[2 * i], x[2 * j + 1] - x[2 * i + 1]);
                fx -= fx.round();
                fy -= fy.round();
                let bx = fx * self.a1[0] + fy * self.a2[0];
                let by = fx * self.a1[1] + fy * self.a2[1];
                let mut sij = 0.0;
                for p in -m..=m {
                    for q in -m..=m {
                        if i == j && p == 0 && q == 0 {
                            continue;
                        }
                        let dx = bx + p as f64 * self.a1[0] + q as f64 * self.a2[0];
                        let dy = by + p as f64 * self.a1[1] + q as f64 * self.a2[1];
                        let r2 = dx * dx + dy * dy;
                        if r2 < rr2 {
                            sij += self.vs(r2.sqrt());
                        }
                    }
                }
                real += if i == j { sij } else { 2.0 * sij };
            }
        }
        let mut recip = self.k0 * (n * n) as f64;
        for &(gx, gy, k) in &self.gvecs {
            let (mut re, mut im) = (0.0, 0.0);
            for j in 0..n {
                let ph = gx * x[2 * j] + gy * x[2 * j + 1];
                re += ph.cos();
                im += ph.sin();
            }
            recip += k * (re * re + im * im);
        }
        recip /= self.area;
        let total = real + recip - n as f64 * self.vl0;
        0.5 * total / n as f64 / (self.d * self.d)
    }
}

/// Exact per-particle energy (U units) of the density-1 triangular lattice.
pub fn triangular_exact(d: f64, eta: f64) -> f64 {
    let a = crate::spacing_density_one();
    DipolarEwald::new(d, eta, [a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a])
        .energy_per_particle(&[0.0, 0.0])
}

/// Exact per-particle energy (U units) of the density-1 square lattice.
pub fn square_exact(d: f64, eta: f64) -> f64 {
    DipolarEwald::new(d, eta, [1.0, 0.0], [0.0, 1.0]).energy_per_particle(&[0.0, 0.0])
}

/// Exact per-particle energy (U units) of the density-1 honeycomb structure.
pub fn honeycomb_exact(d: f64, eta: f64) -> f64 {
    let ah = (4.0 / 3.0_f64.sqrt()).sqrt();
    let a1 = [ah, 0.0];
    let a2 = [0.5 * ah, 0.5 * 3.0_f64.sqrt() * ah];
    let p = [(a1[0] + a2[0]) / 3.0, (a1[1] + a2[1]) / 3.0];
    DipolarEwald::new(d, eta, a1, a2).energy_per_particle(&[0.0, 0.0, p[0], p[1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dipolar::{honeycomb_energy_full, lattice_energy_full};
    use crate::{spacing_density_one, triangular};

    #[test]
    fn gauss_legendre_integrates_polynomials_and_gaussians() {
        let gl = gauss_legendre(40);
        let s: f64 = gl.iter().map(|&(x, w)| w * x.powi(10)).sum();
        assert!((s - 2.0 / 11.0).abs() < 1e-14);
        let e = quad(|t| (-t * t).exp(), 3.0, &gl, 8);
        let exact = 0.5 * std::f64::consts::PI.sqrt() * libm::erf(3.0);
        assert!((e - exact).abs() < 1e-14, "{e} vs {exact}");
    }

    #[test]
    fn energy_is_independent_of_eta() {
        for &d in &[0.0209, 0.3, 1.0] {
            let e1 = triangular_exact(d, 0.6);
            let e2 = triangular_exact(d, 0.9);
            let e3 = triangular_exact(d, 1.3);
            assert!((e1 - e2).abs() < 1e-11 * e1, "d={d}: {e1} {e2}");
            assert!((e1 - e3).abs() < 1e-11 * e1, "d={d}: {e1} {e3}");
        }
    }

    #[test]
    fn ewald_agrees_with_direct_sum_within_its_residual_and_box_with_primitive_cell() {
        let a = spacing_density_one();
        let b = [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]];
        for &d in &[0.066, 1.0] {
            let ew = triangular_exact(d, 0.8);
            let direct = lattice_energy_full(d, b, 256.0);
            assert!((ew - direct).abs() < 2e-5, "d={d}: {ew} vs {direct}");
            let hc = honeycomb_exact(d, 0.8);
            assert!((hc - honeycomb_energy_full(d, 256.0)).abs() < 2e-5);
            let boxed = DipolarEwald::new(d, 0.8, [16.0 * a, 0.0], [0.0, 9.0 * 3.0_f64.sqrt() * a])
                .energy_per_particle(&triangular(16, 18, a));
            assert!((boxed - ew).abs() < 1e-11 * ew, "box {boxed} vs cell {ew}");
        }
    }

    #[test]
    fn pure_dipolar_limit_and_ordering() {
        // d -> 0: U -> 1/r^3, triangular below square below honeycomb
        let t = triangular_exact(1e-4, 0.8);
        let s = square_exact(1e-4, 0.8);
        let h = honeycomb_exact(1e-4, 0.8);
        assert!(t < s && s < h, "{t} {s} {h}");
        assert!((triangular_exact(2e-4, 0.8) - t).abs() < 1e-6 * t);
    }
}
