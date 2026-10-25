//! Spectrally accurate harmonic zero-point energies for Gaussian pair potentials (Tier X numerics).
//!
//! Companion of [`crate::phonon`]. Two independent ingredients replace the real-space sum and the uniform `k` grid:
//!
//! * **Reciprocal-space dynamical matrix.** For `V(r) = exp(-pi a r^2)` on the unit-covolume lattice `L` of shape
//!   `tau = x + i y`, Poisson summation gives, with `kappa = k/(2 pi)` and `L*` the dual lattice,
//!   `D(k) = (4 pi^2/a) sum_{xi in L*} [ (xi+kappa)(xi+kappa)^T e^{-pi |xi+kappa|^2/a} - xi xi^T e^{-pi |xi|^2/a} ]`.
//! * **Duffy-Gauss Brillouin-zone rule.** The reduced cell `[-1/2, 1/2]^2` is split into four triangles with apex at
//!   `Gamma`; the Duffy map `(s, t) -> s * edge(t)` has Jacobian `s/2`, which cancels the conical singularity
//!   `sqrt(D) ~ |k|`, so a tensor Gauss-Legendre rule converges spectrally (instead of the `n^-3` of a uniform grid).
//!   The integrand `sqrt(lam1) + sqrt(lam2) = sqrt(tr D + 2 sqrt(det D))` is smooth across branch crossings.
//!
//! With these, `E_zp` is converged to about `1e-14` with 4 x 64 x 64 nodes, and the hyperbolic Laplacian
//! `y^2 (d_xx + d_yy) E_zp` at the triangular point can be taken by finite differences. At `rho` it fixes the sign of
//! the (isotropic) curvature, `E_zp(tau) - E_zp(rho) = (Lap/4) |tau - rho|^2 / y^2 + O(|tau - rho|^3)`.

#![allow(clippy::needless_range_loop)]

use std::f64::consts::PI;

/// Gauss-Legendre nodes and weights on `[0, 1]` (Newton iteration on `P_n`).
pub fn gauss_legendre01(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut xs = vec![0.0; n];
    let mut ws = vec![0.0; n];
    for i in 0..n {
        let mut z = (PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
        let mut dp = 1.0;
        for _ in 0..100 {
            let (mut p0, mut p1) = (1.0, z);
            for k in 2..=n {
                let p2 = ((2 * k - 1) as f64 * z * p1 - (k - 1) as f64 * p0) / k as f64;
                p0 = p1;
                p1 = p2;
            }
            // p1 = P_n(z), p0 = P_{n-1}(z)
            dp = n as f64 * (z * p1 - p0) / (z * z - 1.0);
            let dz = p1 / dp;
            z -= dz;
            if dz.abs() < 1e-16 {
                break;
            }
        }
        xs[i] = 0.5 * (1.0 - z);
        ws[i] = 1.0 / ((1.0 - z * z) * dp * dp);
    }
    (xs, ws)
}

/// Reciprocal-space dynamical matrix of the Gaussian `exp(-pi a r^2)` for one lattice shape.
#[derive(Clone, Debug)]
pub struct GaussPoisson {
    pub a: f64,
    /// dual basis `xi_1, xi_2` (`xi_i . a_j = delta_ij`)
    pub xi: [[f64; 2]; 2],
    g: Vec<[f64; 2]>,
    c0: [f64; 3],
}

impl GaussPoisson {
    /// Dual-lattice vectors with `|xi| < sqrt(41 a/pi) + max |kappa|` (the omitted terms are below `e^{-41}`).
    pub fn new(a: f64, x: f64, y: f64) -> GaussPoisson {
        let sy = y.sqrt();
        // A = [[1/sy, 0], [x/sy, sy]] (rows a1, a2); xi = rows of A^{-T}
        let xi = [[sy, -x / sy], [0.0, 1.0 / sy]];
        let n1 = xi[0][0].hypot(xi[0][1]);
        let n2 = xi[1][0].hypot(xi[1][1]);
        let rmax = (41.0 * a / PI).sqrt() + 0.5 * (n1 + n2) + 1e-9;
        // |det xi| = 1, so |m| <= rmax |xi_2| and |l| <= rmax |xi_1|
        let mm = (rmax * n2) as i64 + 2;
        let ll = (rmax * n1) as i64 + 2;
        let mut g = Vec::new();
        let mut c0 = [0.0; 3];
        for m in -mm..=mm {
            for l in -ll..=ll {
                let gx = m as f64 * xi[0][0] + l as f64 * xi[1][0];
                let gy = m as f64 * xi[0][1] + l as f64 * xi[1][1];
                let r2 = gx * gx + gy * gy;
                if r2 < rmax * rmax {
                    g.push([gx, gy]);
                    let e = (-PI * r2 / a).exp();
                    c0[0] += e * gx * gx;
                    c0[1] += e * gx * gy;
                    c0[2] += e * gy * gy;
                }
            }
        }
        GaussPoisson { a, xi, g, c0 }
    }

    /// `kappa = u xi_1 + v xi_2` for reduced coordinates `(u, v)`.
    pub fn kappa(&self, u: f64, v: f64) -> [f64; 2] {
        [
            u * self.xi[0][0] + v * self.xi[1][0],
            u * self.xi[0][1] + v * self.xi[1][1],
        ]
    }

    /// `[Dxx, Dxy, Dyy]` at `kappa = k/(2 pi)`; `kappa` should have reduced coordinates in `[-1/2, 1/2]`.
    pub fn d(&self, kappa: [f64; 2]) -> [f64; 3] {
        let mut s = [0.0; 3];
        for gv in &self.g {
            let zx = kappa[0] + gv[0];
            let zy = kappa[1] + gv[1];
            let e = (-PI * (zx * zx + zy * zy) / self.a).exp();
            s[0] += e * zx * zx;
            s[1] += e * zx * zy;
            s[2] += e * zy * zy;
        }
        let pref = 4.0 * PI * PI / self.a;
        [
            pref * (s[0] - self.c0[0]),
            pref * (s[1] - self.c0[1]),
            pref * (s[2] - self.c0[2]),
        ]
    }

    /// `<tr D>_BZ = 4 pi a - (4 pi^2/a) sum_{xi in L*} |xi|^2 e^{-pi |xi|^2/a}` (exact sum rule, equal to
    /// `sum_{R != 0} Lap V(R)`).
    pub fn mean_trace_exact(&self) -> f64 {
        4.0 * PI * self.a - 4.0 * PI * PI / self.a * (self.c0[0] + self.c0[2])
    }
}

/// `sqrt(lam1) + sqrt(lam2)` of the symmetric `2 x 2` matrix `[[d0, d1], [d1, d2]]` (negative parts clipped).
pub fn omega_sum(d: [f64; 3]) -> f64 {
    let t = d[0] + d[2];
    let p = d[0] * d[2] - d[1] * d[1];
    (t + 2.0 * p.max(0.0).sqrt()).max(0.0).sqrt()
}

/// Result of a Duffy-Gauss zone integration.
#[derive(Clone, Debug)]
pub struct DuffyResult {
    /// zero-point energy per particle `(1/2) <sqrt(lam1) + sqrt(lam2)>`
    pub e_zp: f64,
    /// `<tr D>` by the same rule (compare with [`GaussPoisson::mean_trace_exact`])
    pub mean_trace: f64,
    /// minimum over the nodes of `lam_min(D(k)) / |k|^2` (negative: dynamically unstable)
    pub min_ratio: f64,
}

/// Duffy-Gauss `E_zp` of the Gaussian `exp(-pi a r^2)` with `4 n^2` nodes.
pub fn ezp_gauss_duffy(a: f64, x: f64, y: f64, n: usize) -> DuffyResult {
    let gp = GaussPoisson::new(a, x, y);
    let (s, w) = gauss_legendre01(n);
    let (mut e, mut tr, mut rmin) = (0.0, 0.0, f64::INFINITY);
    for i in 0..n {
        for j in 0..n {
            let (si, tj) = (s[i], s[j]);
            let wt = w[i] * w[j] * si / 2.0;
            for (u, v) in [
                (si / 2.0, si * (tj - 0.5)),
                (-si / 2.0, si * (tj - 0.5)),
                (si * (tj - 0.5), si / 2.0),
                (si * (tj - 0.5), -si / 2.0),
            ] {
                let kap = gp.kappa(u, v);
                let d = gp.d(kap);
                e += wt * omega_sum(d);
                tr += wt * (d[0] + d[2]);
                let lo = 0.5 * (d[0] + d[2]) - (0.25 * (d[0] - d[2]).powi(2) + d[1] * d[1]).sqrt();
                let k2 = 4.0 * PI * PI * (kap[0] * kap[0] + kap[1] * kap[1]);
                rmin = rmin.min(lo / k2);
            }
        }
    }
    DuffyResult {
        e_zp: 0.5 * e,
        mean_trace: tr,
        min_ratio: rmin,
    }
}

/// `c = Lap E_zp(rho) / 4` at the triangular point for the Gaussian `exp(-pi a r^2)` (central differences with steps
/// `h` and `h/2`, Richardson-combined; `n` Gauss nodes per Duffy direction). `c < 0` means the triangular lattice is a
/// local maximum of `E_zp`, `c > 0` a local minimum.
pub fn curvature_at_rho_gauss(a: f64, n: usize, h: f64) -> f64 {
    let (x0, y0) = (0.5, 0.75f64.sqrt());
    let e = |x: f64, y: f64| ezp_gauss_duffy(a, x, y, n).e_zp;
    let e0 = e(x0, y0);
    let lap = |hh: f64| -> f64 {
        y0 * y0 * (e(x0 + hh, y0) + e(x0 - hh, y0) + e(x0, y0 + hh) + e(x0, y0 - hh) - 4.0 * e0)
            / (hh * hh)
    };
    (4.0 * lap(h / 2.0) - lap(h)) / 3.0 / 4.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phonon::{Pot, analyze, dynamical_grid};

    const TRI_Y: f64 = 0.866_025_403_784_438_6;

    #[test]
    fn gauss_legendre_is_exact_for_degree_2n_minus_1() {
        let (x, w) = gauss_legendre01(8);
        let s: f64 = x.iter().zip(&w).map(|(xi, wi)| wi * xi.powi(15)).sum();
        assert!((s - 1.0 / 16.0).abs() < 1e-15, "{s}");
        let s14: f64 = x.iter().zip(&w).map(|(xi, wi)| wi * xi.powi(14)).sum();
        assert!((s14 - 1.0 / 15.0).abs() < 1e-15);
    }

    #[test]
    fn poisson_dynamical_matrix_matches_real_space_grid() {
        // the reciprocal-space formula and the folded real-space sum of phonon.rs agree at every grid point
        for &(a, x, y) in &[
            (1.0, 0.5, 0.966_025_403_784),
            (0.5, 0.3, 0.95),
            (2.0, 0.1, 1.3),
        ] {
            let n = 12;
            let dm = dynamical_grid(&Pot::Gaussian { a }, x, y, n, 400.0, true);
            let gp = GaussPoisson::new(a, x, y);
            let scale = dm
                .iter()
                .map(|d| d[0].abs().max(d[2].abs()))
                .fold(0.0, f64::max);
            for i in 0..n {
                for j in 0..n {
                    let u = (i as f64 / n as f64 + 0.5).rem_euclid(1.0) - 0.5;
                    let v = (j as f64 / n as f64 + 0.5).rem_euclid(1.0) - 0.5;
                    let d = gp.d(gp.kappa(u, v));
                    for c in 0..3 {
                        assert!(
                            (d[c] - dm[i * n + j][c]).abs() < 1e-12 * scale,
                            "a={a} ({x},{y}) [{i},{j}] c={c}: {} vs {}",
                            d[c],
                            dm[i * n + j][c]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn duffy_rule_converges_and_matches_extrapolated_grid() {
        let r16 = ezp_gauss_duffy(1.0, 0.5, TRI_Y, 16);
        let r32 = ezp_gauss_duffy(1.0, 0.5, TRI_Y, 32);
        let r48 = ezp_gauss_duffy(1.0, 0.5, TRI_Y, 48);
        // spectral convergence: the n = 16 error is ~1e-7, the n = 32 error below 1e-11
        assert!((r16.e_zp - r48.e_zp).abs() > 1e-9);
        assert!(
            (r32.e_zp - r48.e_zp).abs() < 1e-11,
            "{}",
            r32.e_zp - r48.e_zp
        );
        // exact sum rule for <tr D>
        let exact = GaussPoisson::new(1.0, 0.5, TRI_Y).mean_trace_exact();
        assert!((r48.mean_trace - exact).abs() < 1e-12 * exact);
        // uniform grids converge like n^-3: Richardson of n = 96, 192 agrees with the spectral value
        let pot = Pot::Gaussian { a: 1.0 };
        let e1 = analyze(&pot, 0.5, TRI_Y, 96, 400.0).e_zp;
        let e2 = analyze(&pot, 0.5, TRI_Y, 192, 400.0).e_zp;
        let rich = (8.0 * e2 - e1) / 7.0;
        assert!((rich - r48.e_zp).abs() < 2e-9, "{rich} vs {}", r48.e_zp);
    }

    #[test]
    fn qu1_counterexample_with_spectral_accuracy() {
        // gauss:1, rhombic tau = 0.5 + 0.966025 i is stable and E_zp is lower by 1.3074e-2 (P2 gate G2)
        let tri = ezp_gauss_duffy(1.0, 0.5, TRI_Y, 48);
        let riv = ezp_gauss_duffy(1.0, 0.5, 0.966_025_403_784, 48);
        assert!(tri.min_ratio > 0.0 && riv.min_ratio > 0.0);
        let d = riv.e_zp - tri.e_zp;
        assert!((d + 1.307_403e-2).abs() < 1e-7, "{d}");
        // negative control: the square lattice is dynamically unstable
        assert!(ezp_gauss_duffy(1.0, 0.0, 1.0, 32).min_ratio < -0.1);
    }

    #[test]
    fn curvature_sign_at_rho_changes_with_gaussian_width() {
        // local maximum of E_zp at rho for a = 1, local minimum for a = 2 and a = 0.25 (sign changes near
        // a = 0.377 and a = 1.581)
        assert!(curvature_at_rho_gauss(1.0, 32, 2e-3) < -0.9);
        assert!(curvature_at_rho_gauss(2.0, 32, 2e-3) > 0.4);
        assert!(curvature_at_rho_gauss(0.25, 32, 2e-3) > 0.15);
    }
}
