//! Self-consistent harmonic approximation (SCHA) at `T = 0` for the Gaussian pair potential (Tier X numerics).
//!
//! Units as in [`crate::phonon_spectral`]: `V(r) = exp(-pi a r^2)`, unit covolume, `m = 1`, and a dimensionless
//! quantum parameter `lam = hbar / sqrt(m eps l^2)`. The harmonic energy per particle is
//! `E_static + lam E_zp` with `E_zp = (1/2) <sqrt(lam_1) + sqrt(lam_2)>_BZ`.
//!
//! The SCHA trial state is the ground state of a harmonic crystal with dynamical matrix `D_eff(k)`. The displacement
//! difference `w_R = u_R - u_0` is Gaussian with covariance
//! `C_R = lam < (1 - cos k.R) D_eff(k)^{-1/2} >_BZ`, and for the Gaussian potential the averages are closed form:
//!
//! * `<V(R + w)> = det(I + 2 pi a C)^{-1/2} exp(-pi a R^T (I + 2 pi a C)^{-1} R)`;
//! * `<Hess V(R + w)>` is the Hessian of that smoothed function of `R`.
//!
//! The variational energy is `E = (lam/4) <tr D_eff^{1/2}> + (1/2) sum_{R != 0} <V(R + w_R)>`. It is stationary at
//! `D_eff(k) = sum_{R != 0} (1 - cos k.R) <Hess V(R + w_R)>`, which is iterated with linear mixing. As `lam -> 0` the
//! energy tends to the harmonic `E_static + lam E_zp`, with an `O(lam^2)` remainder.
//!
//! This is a second implementation, independent of the Python one in the B1 records: it uses a real-space dynamical
//! matrix and its own zone rule. Only the Gauss-Legendre nodes are shared, with [`crate::phonon_spectral`].

#![allow(clippy::needless_range_loop)]

use crate::phonon_spectral::gauss_legendre01;
use std::f64::consts::PI;

/// Outcome of an SCHA solve.
#[derive(Clone, Debug)]
pub struct SchaResult {
    /// variational energy per particle
    pub energy: f64,
    /// kinetic part `(lam/4) <tr D_eff^{1/2}>`
    pub kinetic: f64,
    /// `<|u|^2>` per particle, `(lam/2) <tr D_eff^{-1/2}>`
    pub u2: f64,
    /// number of iterations used
    pub iterations: usize,
    /// true when the iteration converged with `D_eff` positive definite on every node
    pub converged: bool,
    /// min over the nodes of `lam_min(D_eff(k)) / |k|^2`
    pub min_ratio: f64,
}

struct Problem {
    a: f64,
    r: Vec<[f64; 2]>,
    /// `(1 - cos k.R)` for node `q`, vector `R`: `omc[q * nr + j]`
    omc: Vec<f64>,
    w: Vec<f64>,
    k2: Vec<f64>,
}

impl Problem {
    fn new(a: f64, x: f64, y: f64, n: usize) -> Problem {
        let sy = y.sqrt();
        let a1 = [1.0 / sy, 0.0];
        let a2 = [x / sy, sy];
        // the smoothed potential is wider than V; keep R with pi a |R|^2 < 4 * 41
        let rmax = (4.0 * 41.0 / (PI * a)).sqrt() + 1.0;
        let n1 = a1[0].hypot(a1[1]);
        let n2 = a2[0].hypot(a2[1]);
        let mm = (rmax * n2) as i64 + 2;
        let ll = (rmax * n1) as i64 + 2;
        let mut r = Vec::new();
        for m in -mm..=mm {
            for l in -ll..=ll {
                if m == 0 && l == 0 {
                    continue;
                }
                let rx = m as f64 * a1[0] + l as f64 * a2[0];
                let ry = m as f64 * a1[1] + l as f64 * a2[1];
                if rx * rx + ry * ry < rmax * rmax {
                    r.push([rx, ry]);
                }
            }
        }
        // dual basis (rows of A^{-T}), absolute k = 2 pi (u xi_1 + v xi_2)
        let xi = [[sy, -x / sy], [0.0, 1.0 / sy]];
        let (s, ws) = gauss_legendre01(n);
        let mut kv = Vec::new();
        let mut w = Vec::new();
        for i in 0..n {
            for j in 0..n {
                let (si, tj) = (s[i], s[j]);
                let wt = ws[i] * ws[j] * si / 2.0;
                for (u, v) in [
                    (si / 2.0, si * (tj - 0.5)),
                    (-si / 2.0, si * (tj - 0.5)),
                    (si * (tj - 0.5), si / 2.0),
                    (si * (tj - 0.5), -si / 2.0),
                ] {
                    kv.push([
                        2.0 * PI * (u * xi[0][0] + v * xi[1][0]),
                        2.0 * PI * (u * xi[0][1] + v * xi[1][1]),
                    ]);
                    w.push(wt);
                }
            }
        }
        let nr = r.len();
        let mut omc = vec![0.0; kv.len() * nr];
        for (q, k) in kv.iter().enumerate() {
            for (j, rv) in r.iter().enumerate() {
                omc[q * nr + j] = 1.0 - (k[0] * rv[0] + k[1] * rv[1]).cos();
            }
        }
        let k2 = kv.iter().map(|k| k[0] * k[0] + k[1] * k[1]).collect();
        Problem { a, r, omc, w, k2 }
    }

    /// `<V>` and `<Hess V>` for every `R` given covariances `c[j] = [cxx, cxy, cyy]`.
    fn smoothed(&self, c: &[[f64; 3]]) -> (Vec<f64>, Vec<[f64; 3]>) {
        let a = self.a;
        let mut v = Vec::with_capacity(self.r.len());
        let mut h = Vec::with_capacity(self.r.len());
        for (j, rv) in self.r.iter().enumerate() {
            let axx = 1.0 + 2.0 * PI * a * c[j][0];
            let axy = 2.0 * PI * a * c[j][1];
            let ayy = 1.0 + 2.0 * PI * a * c[j][2];
            let det = axx * ayy - axy * axy;
            let (mxx, mxy, myy) = (ayy / det, -axy / det, axx / det);
            let q = mxx * rv[0] * rv[0] + 2.0 * mxy * rv[0] * rv[1] + myy * rv[1] * rv[1];
            let vc = (-PI * a * q).exp() / det.sqrt();
            let gx = 2.0 * PI * a * (mxx * rv[0] + mxy * rv[1]);
            let gy = 2.0 * PI * a * (mxy * rv[0] + myy * rv[1]);
            v.push(vc);
            h.push([
                vc * (gx * gx - 2.0 * PI * a * mxx),
                vc * (gx * gy - 2.0 * PI * a * mxy),
                vc * (gy * gy - 2.0 * PI * a * myy),
            ]);
        }
        (v, h)
    }

    fn d_from_h(&self, h: &[[f64; 3]]) -> Vec<[f64; 3]> {
        let nr = self.r.len();
        (0..self.w.len())
            .map(|q| {
                let mut d = [0.0; 3];
                for j in 0..nr {
                    let o = self.omc[q * nr + j];
                    d[0] += o * h[j][0];
                    d[1] += o * h[j][1];
                    d[2] += o * h[j][2];
                }
                d
            })
            .collect()
    }
}

/// `(tr D^{1/2}, D^{-1/2} as [xx, xy, yy], lam_min)` of a symmetric `2 x 2` matrix (`None` unless positive definite).
fn sqrt_parts(d: [f64; 3]) -> Option<(f64, [f64; 3], f64)> {
    let t = d[0] + d[2];
    let p = d[0] * d[2] - d[1] * d[1];
    let lo = 0.5 * t - (0.25 * (d[0] - d[2]).powi(2) + d[1] * d[1]).sqrt();
    if lo <= 0.0 || p <= 0.0 {
        return None;
    }
    let sd = p.sqrt();
    let s = (t + 2.0 * sd).sqrt();
    // D^{1/2} = (D + sqrt(det D) I)/s, and det D^{1/2} = sqrt(det D)
    let (sxx, sxy, syy) = ((d[0] + sd) / s, d[1] / s, (d[2] + sd) / s);
    Some((s, [syy / sd, -sxy / sd, sxx / sd], lo))
}

/// Harmonic energy `E_static + lam E_zp` on the same real-space sum and zone rule (the `lam -> 0` reference).
pub fn harmonic_energy(a: f64, x: f64, y: f64, lam: f64, n: usize) -> f64 {
    let p = Problem::new(a, x, y, n);
    let zero = vec![[0.0; 3]; p.r.len()];
    let (v, h) = p.smoothed(&zero);
    let d = p.d_from_h(&h);
    let ezp: f64 = d
        .iter()
        .zip(&p.w)
        .map(|(dq, wq)| {
            let t = dq[0] + dq[2];
            let pp = (dq[0] * dq[2] - dq[1] * dq[1]).max(0.0);
            wq * (t + 2.0 * pp.sqrt()).max(0.0).sqrt()
        })
        .sum::<f64>()
        * 0.5;
    0.5 * v.iter().sum::<f64>() + lam * ezp
}

/// Solve the SCHA for one shape. `n` is the number of Gauss nodes per Duffy direction (4 n^2 zone nodes).
pub fn scha_solve(
    a: f64,
    x: f64,
    y: f64,
    lam: f64,
    n: usize,
    tol: f64,
    max_iter: usize,
) -> SchaResult {
    let p = Problem::new(a, x, y, n);
    let nr = p.r.len();
    let (_, h0) = p.smoothed(&vec![[0.0; 3]; nr]);
    let mut d = p.d_from_h(&h0);
    let mix = 0.6;
    let mut e_old = f64::NAN;
    let mut res = SchaResult {
        energy: f64::NAN,
        kinetic: f64::NAN,
        u2: f64::NAN,
        iterations: 0,
        converged: false,
        min_ratio: f64::NAN,
    };
    for it in 0..max_iter {
        let mut c = vec![[0.0; 3]; nr];
        let mut kin = 0.0;
        let mut u2 = 0.0;
        let mut rmin = f64::INFINITY;
        for (q, dq) in d.iter().enumerate() {
            let Some((s, inv, lo)) = sqrt_parts(*dq) else {
                res.iterations = it;
                return res;
            };
            rmin = rmin.min(lo / p.k2[q]);
            kin += p.w[q] * s;
            u2 += p.w[q] * (inv[0] + inv[2]);
            for j in 0..nr {
                let f = lam * p.w[q] * p.omc[q * nr + j];
                c[j][0] += f * inv[0];
                c[j][1] += f * inv[1];
                c[j][2] += f * inv[2];
            }
        }
        let (v, h) = p.smoothed(&c);
        let e = 0.25 * lam * kin + 0.5 * v.iter().sum::<f64>();
        let dnew = p.d_from_h(&h);
        let dd = d
            .iter()
            .zip(&dnew)
            .map(|(u, w)| {
                (u[0] - w[0])
                    .abs()
                    .max((u[1] - w[1]).abs())
                    .max((u[2] - w[2]).abs())
            })
            .fold(0.0, f64::max);
        res = SchaResult {
            energy: e,
            kinetic: 0.25 * lam * kin,
            u2: 0.5 * lam * u2,
            iterations: it + 1,
            converged: false,
            min_ratio: rmin,
        };
        if (e - e_old).abs() < tol && dd < 1e-10 {
            res.converged = true;
            return res;
        }
        e_old = e;
        for (u, w) in d.iter_mut().zip(&dnew) {
            for i in 0..3 {
                u[i] = mix * w[i] + (1.0 - mix) * u[i];
            }
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phonon_spectral::ezp_gauss_duffy;

    const Y_RHO: f64 = 0.866_025_403_784_438_6;

    #[test]
    fn harmonic_reference_matches_poisson_zone_rule() {
        // E_zp from the real-space D must equal the reciprocal-space (Poisson) value of phonon_spectral
        let lam = 1.0;
        let e_stat = harmonic_energy(1.0, 0.5, Y_RHO, 0.0, 32);
        let e_h = harmonic_energy(1.0, 0.5, Y_RHO, lam, 32) - e_stat;
        let e_p = ezp_gauss_duffy(1.0, 0.5, Y_RHO, 32).e_zp;
        assert!((e_h - e_p).abs() < 1e-11, "{e_h} vs {e_p}");
        assert!((e_p - 1.503_352_352_2).abs() < 1e-9);
    }

    #[test]
    fn small_lam_limit_is_harmonic_with_lam_squared_remainder() {
        let e1 = scha_solve(1.0, 0.5, Y_RHO, 1e-3, 24, 1e-15, 500);
        let e2 = scha_solve(1.0, 0.5, Y_RHO, 2e-3, 24, 1e-15, 500);
        assert!(e1.converged && e2.converged);
        let d1 = e1.energy - harmonic_energy(1.0, 0.5, Y_RHO, 1e-3, 24);
        let d2 = e2.energy - harmonic_energy(1.0, 0.5, Y_RHO, 2e-3, 24);
        // remainder ~ c lam^2: doubling lam multiplies it by about 4
        let ratio = d2 / d1;
        assert!((ratio - 4.0).abs() < 0.3, "ratio {ratio}");
        assert!(d1.abs() < 1e-6);
    }

    #[test]
    fn square_lattice_is_rejected_and_large_lam_breaks_down() {
        // negative controls: the square lattice is dynamically unstable, so no SCHA trial exists at small lam
        let sq = scha_solve(1.0, 0.0, 1.0, 1e-3, 16, 1e-13, 200);
        assert!(!sq.converged);
        // and at lam = 1.6 the triangular SCHA iteration loses positive definiteness
        let big = scha_solve(1.0, 0.5, Y_RHO, 1.6, 16, 1e-13, 400);
        assert!(!big.converged);
        let ok = scha_solve(1.0, 0.5, Y_RHO, 0.1, 16, 1e-13, 400);
        assert!(ok.converged && ok.min_ratio > 0.0);
    }
}
