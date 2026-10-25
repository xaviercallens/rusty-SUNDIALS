//! Exact (untruncated) planar Yukawa energies with a controlled tail.
//!
//! The published study (`System::pair`) uses a shifted-force cut at `rc = 8`. That potential is not completely
//! monotone, and at `kappa = 1` the shift moves the energy per particle by about `1.8e-2`, more than the
//! square-triangular gap. This module evaluates the plain potential `V(r) = exp(-kappa r) / r` instead:
//!
//! * [`lattice_energy_exact`]: `(1/2) sum_{0 < |v| < R} V(|v|)` over a Bravais lattice, plus a rigorous upper bound on
//!   the omitted tail `(1/2) sum_{|v| >= R} V(|v|)` (Voronoi-cell comparison, see [`tail_bound`]).
//! * [`lattice_energy_ewald`]: an independent Ewald-type splitting of the same sum (Gaussian integral representation
//!   of `exp(-kappa r)/r`, closed-form real- and reciprocal-space terms with `erfc`), used as a cross-check.
//! * [`ExactBox`]: a periodic rectangle where every pair interacts with all its periodic images within `R` (not the
//!   minimum image only), including each particle's own images, so a perfect lattice in a commensurate box has exactly
//!   the infinite-lattice energy per particle. Energy, force and Hessian are consistent; [`relax_exact`] runs CVODE.
//!
//! All results are floating point (exploration tier); they do not prove anything.

use cvode::{Cvode, DenseMat, Method, Task};
use nvector::SerialVector;

/// `V(r) = exp(-kappa r) / r`.
pub fn yukawa(kappa: f64, r: f64) -> f64 {
    (-kappa * r).exp() / r
}

fn dyukawa(kappa: f64, r: f64) -> f64 {
    -(-kappa * r).exp() * (kappa * r + 1.0) / (r * r)
}

fn d2yukawa(kappa: f64, r: f64) -> f64 {
    (-kappa * r).exp() * (kappa * kappa * r * r + 2.0 * kappa * r + 2.0) / (r * r * r)
}

/// Area of the cell spanned by `basis`.
pub fn cell_area(basis: [[f64; 2]; 2]) -> f64 {
    (basis[0][0] * basis[1][1] - basis[0][1] * basis[1][0]).abs()
}

/// Circumradius of the Voronoi cell (largest distance from a lattice point to its cell). Computed from the vertices
/// of the Voronoi cell, which are circumcentres of triangles of nearby lattice points; brute force over a small patch.
pub fn voronoi_circumradius(basis: [[f64; 2]; 2]) -> f64 {
    // Points of the lattice near the origin.
    let mut pts = Vec::new();
    for i in -3i64..=3 {
        for j in -3i64..=3 {
            pts.push((
                i as f64 * basis[0][0] + j as f64 * basis[1][0],
                i as f64 * basis[0][1] + j as f64 * basis[1][1],
            ));
        }
    }
    // The Voronoi cell of 0 is {x : |x| <= |x - p| for all p}. Its farthest point from 0 is a vertex, i.e. the
    // intersection of two bisectors; maximise |x| over all feasible bisector intersections.
    let mut best: f64 = 0.0;
    let nz: Vec<(f64, f64)> = pts
        .iter()
        .copied()
        .filter(|&(x, y)| x * x + y * y > 1e-24)
        .collect();
    for (a, &(px, py)) in nz.iter().enumerate() {
        for &(qx, qy) in nz.iter().skip(a + 1) {
            // Bisector of 0 and p: x.p = |p|^2 / 2.
            let det = px * qy - py * qx;
            if det.abs() < 1e-12 {
                continue;
            }
            let (bp, bq) = (0.5 * (px * px + py * py), 0.5 * (qx * qx + qy * qy));
            let x = (bp * qy - bq * py) / det;
            let y = (px * bq - qx * bp) / det;
            let ok = nz
                .iter()
                .all(|&(sx, sy)| x * sx + y * sy <= 0.5 * (sx * sx + sy * sy) + 1e-12);
            if ok {
                best = best.max((x * x + y * y).sqrt());
            }
        }
    }
    best
}

/// Rigorous upper bound on `(1/2) sum_{|v| >= R} V(|v|)` per particle.
///
/// For decreasing `f` and every `x` in the Voronoi cell of `v`, `|x| >= |v| - delta`, hence `|v| >= |x| - delta` and
/// `f(|v|) <= f(|x| - delta)` once `|x| > delta`. Averaging over the cell (area `A`) and noting that cells of points
/// with `|v| >= R` lie in `|x| >= R - delta`:
/// `sum_{|v|>=R} f(|v|) <= (1/A) int_{|x| >= R - delta} f(|x| - delta) dx = (2 pi / A) int_{R-2delta}^inf (s + delta) f(s) ds`.
/// With `f(s) = e^{-kappa s}/s` and `(s + delta)/s <= 1 + delta/(R - 2 delta)` this is
/// `(2 pi / A) (1 + delta/(R - 2 delta)) e^{-kappa (R - 2 delta)} / kappa`; half of it per particle.
pub fn tail_bound(kappa: f64, area: f64, delta: f64, r: f64) -> f64 {
    assert!(
        r > 2.0 * delta,
        "R must exceed twice the Voronoi circumradius"
    );
    let s0 = r - 2.0 * delta;
    std::f64::consts::PI / area * (1.0 + delta / s0) * (-kappa * s0).exp() / kappa
}

/// Mean-field (continuum) estimate of the same tail: `(1/2) rho int_R^inf 2 pi r V(r) dr = pi rho e^{-kappa R}/kappa`.
pub fn tail_continuum(kappa: f64, area: f64, r: f64) -> f64 {
    std::f64::consts::PI / area * (-kappa * r).exp() / kappa
}

/// Lattice vectors `v != 0` with `|v| < r`, for a basis with `basis[0]` along the x axis.
fn lattice_vectors(basis: [[f64; 2]; 2], r: f64) -> Vec<(f64, f64)> {
    assert!(
        basis[0][1] == 0.0 && basis[0][0] > 0.0 && basis[1][1] != 0.0,
        "basis[0] must lie on the positive x axis"
    );
    let jm = (r / basis[1][1].abs()).ceil() as i64 + 1;
    let mut out = Vec::new();
    for j in -jm..=jm {
        let x0 = j as f64 * basis[1][0];
        let y = j as f64 * basis[1][1];
        let ilo = ((-r - x0) / basis[0][0]).floor() as i64 - 1;
        let ihi = ((r - x0) / basis[0][0]).ceil() as i64 + 1;
        for i in ilo..=ihi {
            if i == 0 && j == 0 {
                continue;
            }
            let x = x0 + i as f64 * basis[0][0];
            let rr = (x * x + y * y).sqrt();
            if rr < r {
                out.push((x, y));
            }
        }
    }
    out
}

/// Result of an exact lattice sum.
#[derive(Clone, Copy, Debug)]
pub struct LatticeSum {
    /// `(1/2) sum_{0<|v|<R} V(|v|)` (no tail added).
    pub partial: f64,
    /// Rigorous upper bound on the omitted tail (so the exact value lies in `[partial, partial + tail_bound]`).
    pub tail_bound: f64,
    /// Continuum estimate of the tail.
    pub tail_continuum: f64,
    /// Number of lattice vectors summed.
    pub terms: usize,
}

/// Exact per-particle Yukawa energy of the lattice, summed to radius `r` with the tail bound.
/// Terms are added in increasing `|v|` order with Neumaier compensation.
pub fn lattice_energy_exact(kappa: f64, basis: [[f64; 2]; 2], r: f64) -> LatticeSum {
    let mut vs: Vec<f64> = lattice_vectors(basis, r)
        .into_iter()
        .map(|(x, y)| (x * x + y * y).sqrt())
        .collect();
    vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
    // Sum smallest terms (largest |v|) first.
    let (mut s, mut c) = (0.0_f64, 0.0_f64);
    for &d in &vs {
        let t = yukawa(kappa, d);
        let u = s + t;
        if s.abs() >= t.abs() {
            c += (s - u) + t;
        } else {
            c += (t - u) + s;
        }
        s = u;
    }
    let area = cell_area(basis);
    LatticeSum {
        partial: 0.5 * (s + c),
        tail_bound: tail_bound(kappa, area, voronoi_circumradius(basis), r),
        tail_continuum: tail_continuum(kappa, area, r),
        terms: vs.len(),
    }
}

/// Per-particle energy with the published shifted-force potential at cut `rc` (same as `lattice_energy` in lib.rs,
/// but with a correct enumeration of lattice vectors for any `rc`).
pub fn lattice_energy_shifted_force(kappa: f64, basis: [[f64; 2]; 2], rc: f64) -> f64 {
    let (vc, dvc) = (yukawa(kappa, rc), dyukawa(kappa, rc));
    0.5 * lattice_vectors(basis, rc)
        .into_iter()
        .map(|(x, y)| {
            let d = (x * x + y * y).sqrt();
            yukawa(kappa, d) - vc - (d - rc) * dvc
        })
        .sum::<f64>()
}

/// Ewald-type splitting of `(1/2) sum_{v != 0} exp(-kappa |v|)/|v|`.
///
/// From `e^{-kappa r}/r = (2/sqrt pi) int_0^inf exp(-r^2 s^2 - kappa^2/(4 s^2)) ds`, splitting at `s = eta`:
/// real space `sum_{v!=0} [e^{kappa r} erfc(r eta + kappa/(2 eta)) + e^{-kappa r} erfc(r eta - kappa/(2 eta))]/(2r)`,
/// reciprocal space `(2 pi / A) sum_G erfc(q/(2 eta))/q` with `q = sqrt(|G|^2 + kappa^2)` (G = 0 included),
/// minus the self term `(2/sqrt pi)[eta e^{-kappa^2/(4 eta^2)} - (kappa sqrt(pi)/2) erfc(kappa/(2 eta))]`.
/// Real- and reciprocal-space sums are cut where the Gaussian factors are below `1e-17` relative.
pub fn lattice_energy_ewald(kappa: f64, basis: [[f64; 2]; 2], eta: f64) -> f64 {
    let area = cell_area(basis);
    let b = 0.5 * kappa / eta;
    // erfc(x) < 1e-17 for x > 5.9; the real-space cut needs r eta - b > 6.
    let r_real = (6.5 + b) / eta;
    let real: f64 = lattice_vectors(basis, r_real)
        .into_iter()
        .map(|(x, y)| {
            let r = (x * x + y * y).sqrt();
            // e^{kappa r} stays below e^{kappa r_real}, finite for the eta used here.
            let t1 = (kappa * r).exp() * libm::erfc(r * eta + b);
            let t2 = (-kappa * r).exp() * libm::erfc(r * eta - b);
            (t1 + t2) / (2.0 * r)
        })
        .sum();
    // Reciprocal lattice: G_i . a_j = 2 pi delta_ij.
    let det = basis[0][0] * basis[1][1] - basis[0][1] * basis[1][0];
    let g0 = [
        2.0 * std::f64::consts::PI * basis[1][1] / det,
        -2.0 * std::f64::consts::PI * basis[1][0] / det,
    ];
    let g1 = [
        -2.0 * std::f64::consts::PI * basis[0][1] / det,
        2.0 * std::f64::consts::PI * basis[0][0] / det,
    ];
    // erfc(q/(2 eta)) < 1e-17 for q > 12 eta.
    let q_cut = 13.0 * eta;
    let gm = (q_cut / g0[0].hypot(g0[1]).min(g1[0].hypot(g1[1]))).ceil() as i64 + 2;
    let mut recip = 0.0;
    for i in -gm..=gm {
        for j in -gm..=gm {
            let gx = i as f64 * g0[0] + j as f64 * g1[0];
            let gy = i as f64 * g0[1] + j as f64 * g1[1];
            let q = (gx * gx + gy * gy + kappa * kappa).sqrt();
            recip += libm::erfc(q / (2.0 * eta)) / q;
        }
    }
    recip *= 2.0 * std::f64::consts::PI / area;
    let sqrt_pi = std::f64::consts::PI.sqrt();
    let selfterm = 2.0 / sqrt_pi * (eta * (-b * b).exp() - 0.5 * kappa * sqrt_pi * libm::erfc(b));
    0.5 * (real + recip - selfterm)
}

/// Periodic rectangle with full image summation of the plain Yukawa potential to radius `rcut`.
#[derive(Clone, Debug)]
pub struct ExactBox {
    pub kappa: f64,
    pub rcut: f64,
    pub lx: f64,
    pub ly: f64,
    /// Image shifts `(p lx, q ly)` that can bring some pair within `rcut`.
    shifts: Vec<(f64, f64)>,
    /// `(1/2) sum_{n != 0, |n L| < rcut} V(|n L|)`: each particle's interaction with its own images.
    self_image: f64,
}

impl ExactBox {
    pub fn new(kappa: f64, rcut: f64, lx: f64, ly: f64) -> Self {
        let half_diag = 0.5 * (lx * lx + ly * ly).sqrt();
        let (pm, qm) = ((rcut / lx).ceil() as i64 + 1, (rcut / ly).ceil() as i64 + 1);
        let mut shifts = Vec::new();
        let mut self_image = 0.0;
        for p in -pm..=pm {
            for q in -qm..=qm {
                let (sx, sy) = (p as f64 * lx, q as f64 * ly);
                let d = (sx * sx + sy * sy).sqrt();
                if d < rcut + half_diag + 1e-9 {
                    shifts.push((sx, sy));
                }
                if (p != 0 || q != 0) && d < rcut {
                    self_image += yukawa(kappa, d);
                }
            }
        }
        ExactBox {
            kappa,
            rcut,
            lx,
            ly,
            shifts,
            self_image: 0.5 * self_image,
        }
    }

    pub fn self_image(&self) -> f64 {
        self.self_image
    }

    fn min_image(&self, xi: f64, yi: f64, xj: f64, yj: f64) -> (f64, f64) {
        let (mut dx, mut dy) = (xi - xj, yi - yj);
        dx -= self.lx * (dx / self.lx).round();
        dy -= self.ly * (dy / self.ly).round();
        (dx, dy)
    }

    /// Continuum estimate of the omitted tail per particle at density `N / (lx ly)`.
    pub fn tail_continuum(&self, n: usize) -> f64 {
        tail_continuum(self.kappa, self.lx * self.ly / n as f64, self.rcut)
    }

    /// Total energy (no tail correction). Pair sums are accumulated with Neumaier compensation.
    pub fn energy(&self, x: &[f64]) -> f64 {
        let n = x.len() / 2;
        let (mut e, mut c) = (n as f64 * self.self_image, 0.0_f64);
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.min_image(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let mut t = 0.0;
                for &(sx, sy) in &self.shifts {
                    let (ux, uy) = (dx + sx, dy + sy);
                    let r = (ux * ux + uy * uy).sqrt();
                    if r < self.rcut {
                        t += yukawa(self.kappa, r);
                    }
                }
                let u = e + t;
                if e.abs() >= t.abs() {
                    c += (e - u) + t;
                } else {
                    c += (t - u) + e;
                }
                e = u;
            }
        }
        e + c
    }

    /// Force `F = -grad E` into `f`.
    pub fn force(&self, x: &[f64], f: &mut [f64]) {
        let n = x.len() / 2;
        f.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.min_image(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let (mut fx, mut fy) = (0.0, 0.0);
                for &(sx, sy) in &self.shifts {
                    let (ux, uy) = (dx + sx, dy + sy);
                    let r = (ux * ux + uy * uy).sqrt();
                    if r < self.rcut {
                        let dv = dyukawa(self.kappa, r);
                        fx -= dv * ux / r;
                        fy -= dv * uy / r;
                    }
                }
                f[2 * i] += fx;
                f[2 * i + 1] += fy;
                f[2 * j] -= fx;
                f[2 * j + 1] -= fy;
            }
        }
    }

    /// Dense Hessian (row-major `2n x 2n`).
    pub fn hessian(&self, x: &[f64]) -> Vec<f64> {
        let n = x.len() / 2;
        let m = 2 * n;
        let mut h = vec![0.0; m * m];
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.min_image(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let mut k = [[0.0; 2]; 2];
                for &(sx, sy) in &self.shifts {
                    let (ux, uy) = (dx + sx, dy + sy);
                    let r = (ux * ux + uy * uy).sqrt();
                    if r < self.rcut {
                        let (dv, d2v) = (dyukawa(self.kappa, r), d2yukawa(self.kappa, r));
                        let u = [ux / r, uy / r];
                        for (a, ka) in k.iter_mut().enumerate() {
                            for (b, kab) in ka.iter_mut().enumerate() {
                                let id = if a == b { 1.0 } else { 0.0 };
                                *kab += d2v * u[a] * u[b] + dv / r * (id - u[a] * u[b]);
                            }
                        }
                    }
                }
                for (a, ka) in k.iter().enumerate() {
                    for (b, &kab) in ka.iter().enumerate() {
                        h[(2 * i + a) * m + 2 * i + b] += kab;
                        h[(2 * j + a) * m + 2 * j + b] += kab;
                        h[(2 * i + a) * m + 2 * j + b] -= kab;
                        h[(2 * j + a) * m + 2 * i + b] -= kab;
                    }
                }
            }
        }
        h
    }

    pub fn wrap(&self, x: &mut [f64]) {
        for i in 0..x.len() / 2 {
            x[2 * i] = x[2 * i].rem_euclid(self.lx);
            x[2 * i + 1] = x[2 * i + 1].rem_euclid(self.ly);
        }
    }
}

/// Overdamped CVODE relaxation under [`ExactBox`] (same protocol as `relax` in lib.rs).
pub fn relax_exact(
    sys: &ExactBox,
    x0: &[f64],
    ftol: f64,
    tmax: f64,
) -> Result<crate::Relaxed, String> {
    let s1 = sys.clone();
    let s2 = sys.clone();
    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        s1.force(y, ydot);
        Ok(())
    };
    let jac = move |_t: f64, y: &[f64], j: &mut DenseMat| -> Result<(), String> {
        let h = s2.hessian(y);
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
        sys.force(&y, &mut f);
        let fmax = f.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
        if fmax < ftol || t >= tmax {
            let mut x = y;
            sys.wrap(&mut x);
            return Ok(crate::Relaxed {
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

/// Triangular basis at density one.
pub fn tri_basis() -> [[f64; 2]; 2] {
    let a = crate::spacing_density_one();
    [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]]
}

/// Square basis at density one.
pub fn square_basis() -> [[f64; 2]; 2] {
    [[1.0, 0.0], [0.0, 1.0]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rng, square, triangular};

    #[test]
    fn direct_sum_matches_ewald_splitting() {
        for kappa in [1.0, 2.0] {
            for basis in [tri_basis(), square_basis()] {
                let d = lattice_energy_exact(kappa, basis, 40.0 / kappa);
                for eta in [0.5, 1.0, 2.0] {
                    let e = lattice_energy_ewald(kappa, basis, eta);
                    assert!(
                        (d.partial - e).abs() < 1e-12,
                        "kappa={kappa} eta={eta} direct={} ewald={e}",
                        d.partial
                    );
                }
                assert!(d.tail_bound < 1e-15 && d.tail_bound > d.tail_continuum);
            }
        }
    }

    #[test]
    fn voronoi_circumradius_known_values() {
        let a = crate::spacing_density_one();
        assert!((voronoi_circumradius(tri_basis()) - a / 3.0_f64.sqrt()).abs() < 1e-12);
        assert!((voronoi_circumradius(square_basis()) - 0.5_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn shifted_force_reproduces_published_reference() {
        // results/references.jsonl: kappa=1 e_tri_sum 1.607609217185270, kappa=2 e_square_sum 0.4009647698931363.
        let t = lattice_energy_shifted_force(1.0, tri_basis(), 8.0);
        let s = lattice_energy_shifted_force(2.0, square_basis(), 8.0);
        assert!((t - 1.607609217185270).abs() < 1e-12, "{t}");
        assert!((s - 0.4009647698931363).abs() < 1e-12, "{s}");
    }

    #[test]
    fn perfect_lattices_in_image_box_equal_lattice_sum() {
        let a = crate::spacing_density_one();
        for kappa in [1.0, 2.0] {
            let rcut = 40.0 / kappa;
            let tb = ExactBox::new(kappa, rcut, 16.0 * a, 9.0 * 3.0_f64.sqrt() * a);
            let et = tb.energy(&triangular(16, 18, a)) / 288.0;
            let lt = lattice_energy_exact(kappa, tri_basis(), rcut).partial;
            assert!((et - lt).abs() < 1e-13, "tri box {et} sum {lt}");
            let sb = ExactBox::new(kappa, rcut, 17.0, 17.0);
            let es = sb.energy(&square(17, 1.0)) / 289.0;
            let ls = lattice_energy_exact(kappa, square_basis(), rcut).partial;
            assert!((es - ls).abs() < 1e-13, "square box {es} sum {ls}");
            assert!(tb.self_image() > 0.0);
        }
    }

    #[test]
    fn exact_force_and_hessian_match_finite_differences() {
        let a = crate::spacing_density_one();
        let sys = ExactBox::new(1.0, 30.0, 16.0 * a, 9.0 * 3.0_f64.sqrt() * a);
        let mut x = triangular(16, 18, a);
        let mut rng = Rng::new(13);
        x.iter_mut().for_each(|v| *v += 0.05 * rng.gauss());
        let m = x.len();
        let mut f = vec![0.0; m];
        sys.force(&x, &mut f);
        let h = sys.hessian(&x);
        for &k in &[0usize, 77, 401] {
            let e = 1e-6;
            let (mut xp, mut xm) = (x.clone(), x.clone());
            xp[k] += e;
            xm[k] -= e;
            let fd = -(sys.energy(&xp) - sys.energy(&xm)) / (2.0 * e);
            assert!((fd - f[k]).abs() < 1e-6, "force k={k} fd={fd} f={}", f[k]);
            let (mut fp, mut fm) = (vec![0.0; m], vec![0.0; m]);
            sys.force(&xp, &mut fp);
            sys.force(&xm, &mut fm);
            for r in [k, k + 1, 50] {
                let hd = -(fp[r] - fm[r]) / (2.0 * e);
                assert!((hd - h[r * m + k]).abs() < 1e-5, "hess r={r} k={k}");
            }
        }
    }
}
