//! Numerical check of planar universal optimality for the 2D Yukawa interaction.
//!
//! Particles in the plane interact through `V(r) = exp(-kappa r) / r`, truncated with a shifted-force cutoff at
//! `rc` (so energy and force are continuous at `rc`). Configurations live either in a periodic rectangle (minimum
//! image) or in an open disk (harmonic bowl) / closed disk (steep soft wall). Overdamped relaxation
//! `dx/dt = -grad E` is integrated with this repository's CVODE (BDF) and an analytic Jacobian `-Hessian`.
//!
//! With `t = r^2`, `exp(-kappa sqrt t) / sqrt t` is completely monotone, so the planar universal-optimality theorem
//! predicts that no density-1 configuration has a lower energy per particle than the triangular lattice. The
//! truncation makes the potential not exactly completely monotone beyond `rc`; its size is `~exp(-kappa rc)`.

use cvode::{Cvode, DenseMat, Method, Task};
use nvector::SerialVector;

/// Triangular lattice spacing at density 1.
pub fn spacing_density_one() -> f64 {
    (2.0 / 3.0_f64.sqrt()).sqrt()
}

/// Confinement of the configuration.
#[derive(Clone, Copy, Debug)]
pub enum Geometry {
    /// Periodic rectangle `[0, lx) x [0, ly)` with minimum-image convention.
    Periodic { lx: f64, ly: f64 },
    /// Open disk: harmonic bowl `U = k r^2 / 2` per particle.
    Bowl { k: f64 },
    /// Closed disk: steep soft wall `U = eps (r - R)^4` for `r > R`.
    Wall { radius: f64, eps: f64 },
}

/// The interacting system.
#[derive(Clone, Copy, Debug)]
pub struct System {
    pub kappa: f64,
    pub rc: f64,
    pub geometry: Geometry,
}

impl System {
    fn v(&self, r: f64) -> f64 {
        (-self.kappa * r).exp() / r
    }
    fn dv(&self, r: f64) -> f64 {
        -(-self.kappa * r).exp() * (self.kappa * r + 1.0) / (r * r)
    }
    fn d2v(&self, r: f64) -> f64 {
        let k = self.kappa;
        (-k * r).exp() * (k * k * r * r + 2.0 * k * r + 2.0) / (r * r * r)
    }
    /// Shifted-force pair potential and its first two derivatives (zero for `r >= rc`).
    pub fn pair(&self, r: f64) -> (f64, f64, f64) {
        if r >= self.rc {
            return (0.0, 0.0, 0.0);
        }
        let (vc, dvc) = (self.v(self.rc), self.dv(self.rc));
        (self.v(r) - vc - (r - self.rc) * dvc, self.dv(r) - dvc, self.d2v(r))
    }

    fn delta(&self, xi: f64, yi: f64, xj: f64, yj: f64) -> (f64, f64) {
        let (mut dx, mut dy) = (xi - xj, yi - yj);
        if let Geometry::Periodic { lx, ly } = self.geometry {
            dx -= lx * (dx / lx).round();
            dy -= ly * (dy / ly).round();
        }
        (dx, dy)
    }

    /// External radial potential `U(r)`, `U'(r)`, `U''(r)`.
    fn external(&self, r: f64) -> (f64, f64, f64) {
        match self.geometry {
            Geometry::Periodic { .. } => (0.0, 0.0, 0.0),
            Geometry::Bowl { k } => (0.5 * k * r * r, k * r, k),
            Geometry::Wall { radius, eps } => {
                if r > radius {
                    let s = r - radius;
                    (eps * s.powi(4), 4.0 * eps * s.powi(3), 12.0 * eps * s * s)
                } else {
                    (0.0, 0.0, 0.0)
                }
            }
        }
    }

    /// Total energy.
    pub fn energy(&self, x: &[f64]) -> f64 {
        let n = x.len() / 2;
        let mut e = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                e += self.pair((dx * dx + dy * dy).sqrt()).0;
            }
            let r = (x[2 * i] * x[2 * i] + x[2 * i + 1] * x[2 * i + 1]).sqrt();
            e += self.external(r).0;
        }
        e
    }

    /// Force `F = -grad E` written into `f`.
    pub fn force(&self, x: &[f64], f: &mut [f64]) {
        let n = x.len() / 2;
        f.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let r = (dx * dx + dy * dy).sqrt();
                let (_, dv, _) = self.pair(r);
                if dv != 0.0 {
                    let (fx, fy) = (-dv * dx / r, -dv * dy / r);
                    f[2 * i] += fx;
                    f[2 * i + 1] += fy;
                    f[2 * j] -= fx;
                    f[2 * j + 1] -= fy;
                }
            }
            let (xi, yi) = (x[2 * i], x[2 * i + 1]);
            let r = (xi * xi + yi * yi).sqrt();
            let (_, du, _) = self.external(r);
            if du != 0.0 && r > 0.0 {
                f[2 * i] -= du * xi / r;
                f[2 * i + 1] -= du * yi / r;
            } else if let Geometry::Bowl { k } = self.geometry {
                f[2 * i] -= k * xi;
                f[2 * i + 1] -= k * yi;
            }
        }
    }

    /// Hessian of `E` (dense, row-major, `2n x 2n`).
    pub fn hessian(&self, x: &[f64]) -> Vec<f64> {
        let n = x.len() / 2;
        let m = 2 * n;
        let mut h = vec![0.0; m * m];
        let mut add = |a: usize, b: usize, v: f64| h[a * m + b] += v;
        for i in 0..n {
            for j in (i + 1)..n {
                let (dx, dy) = self.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                let r = (dx * dx + dy * dy).sqrt();
                let (_, dv, d2v) = self.pair(r);
                if r >= self.rc {
                    continue;
                }
                let u = [dx / r, dy / r];
                for a in 0..2 {
                    for b in 0..2 {
                        let id = if a == b { 1.0 } else { 0.0 };
                        let k = d2v * u[a] * u[b] + dv / r * (id - u[a] * u[b]);
                        add(2 * i + a, 2 * i + b, k);
                        add(2 * j + a, 2 * j + b, k);
                        add(2 * i + a, 2 * j + b, -k);
                        add(2 * j + a, 2 * i + b, -k);
                    }
                }
            }
            let p = [x[2 * i], x[2 * i + 1]];
            let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
            let (_, du, d2u) = self.external(r);
            match self.geometry {
                Geometry::Bowl { k } => {
                    add(2 * i, 2 * i, k);
                    add(2 * i + 1, 2 * i + 1, k);
                }
                Geometry::Wall { .. } if du != 0.0 => {
                    let u = [p[0] / r, p[1] / r];
                    for a in 0..2 {
                        for b in 0..2 {
                            let id = if a == b { 1.0 } else { 0.0 };
                            add(2 * i + a, 2 * i + b, d2u * u[a] * u[b] + du / r * (id - u[a] * u[b]));
                        }
                    }
                }
                _ => {}
            }
        }
        h
    }

    /// Wrap coordinates into the periodic cell (no-op for disks).
    pub fn wrap(&self, x: &mut [f64]) {
        if let Geometry::Periodic { lx, ly } = self.geometry {
            for i in 0..x.len() / 2 {
                x[2 * i] = x[2 * i].rem_euclid(lx);
                x[2 * i + 1] = x[2 * i + 1].rem_euclid(ly);
            }
        }
    }
}

/// Perfect triangular lattice `nx x ny` (ny even) with spacing `a`, in the cell `nx a x ny a sqrt(3)/2`.
pub fn triangular(nx: usize, ny: usize, a: f64) -> Vec<f64> {
    let mut x = Vec::with_capacity(2 * nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let shift = if j % 2 == 1 { 0.5 } else { 0.0 };
            x.push((i as f64 + shift + 0.25) * a);
            x.push((j as f64 + 0.5) * a * 3.0_f64.sqrt() / 2.0);
        }
    }
    x
}

/// Perfect square lattice `n x n` with spacing `a`.
pub fn square(n: usize, a: f64) -> Vec<f64> {
    let mut x = Vec::with_capacity(2 * n * n);
    for j in 0..n {
        for i in 0..n {
            x.push((i as f64 + 0.5) * a);
            x.push((j as f64 + 0.5) * a);
        }
    }
    x
}

/// Per-particle energy of an infinite lattice by direct summation, `(1/2) sum_{v != 0, |v| < rc} V_sf(|v|)`.
pub fn lattice_energy(sys: &System, basis: [[f64; 2]; 2]) -> f64 {
    let m = (sys.rc / basis[0][0].abs().min(basis[1][1].abs()) + 3.0).ceil() as i64;
    let mut s = 0.0;
    for i in -m..=m {
        for j in -m..=m {
            if i == 0 && j == 0 {
                continue;
            }
            let vx = i as f64 * basis[0][0] + j as f64 * basis[1][0];
            let vy = i as f64 * basis[0][1] + j as f64 * basis[1][1];
            s += sys.pair((vx * vx + vy * vy).sqrt()).0;
        }
    }
    0.5 * s
}

/// Outcome of a CVODE relaxation.
#[derive(Clone, Debug)]
pub struct Relaxed {
    pub x: Vec<f64>,
    pub t: f64,
    pub max_force: f64,
    pub steps: usize,
    pub rhs_evals: usize,
    pub converged: bool,
}

/// Overdamped relaxation `dx/dt = F(x)` with CVODE (BDF, analytic Jacobian) until `max |F| < ftol` or `t > tmax`.
pub fn relax(sys: &System, x0: &[f64], ftol: f64, tmax: f64) -> Result<Relaxed, String> {
    let s1 = *sys;
    let s2 = *sys;
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
        let (t, y) = solver.solve(tout, Task::Normal).map_err(|e| format!("CVODE failed at t = {tout}: {e}"))?;
        let y = y.to_vec();
        sys.force(&y, &mut f);
        let fmax = f.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
        if fmax < ftol || t >= tmax {
            let mut x = y;
            sys.wrap(&mut x);
            return Ok(Relaxed { x, t, max_force: fmax, steps: solver.num_steps(), rhs_evals: solver.num_rhs_evals(), converged: fmax < ftol });
        }
        tout *= 2.0;
    }
}

/// Eigenvalues of a symmetric matrix (row-major `m x m`) by cyclic Jacobi rotations, ascending.
pub fn symmetric_eigenvalues(a: &[f64], m: usize) -> Vec<f64> {
    let mut a = a.to_vec();
    for _sweep in 0..100 {
        let off: f64 = (0..m).flat_map(|i| (0..m).filter(move |&j| j != i).map(move |j| (i, j))).map(|(i, j)| a[i * m + j] * a[i * m + j]).sum();
        let diag: f64 = (0..m).map(|i| a[i * m + i] * a[i * m + i]).sum();
        if off <= 1e-26 * diag.max(1e-300) {
            break;
        }
        for p in 0..m {
            for q in (p + 1)..m {
                let apq = a[p * m + q];
                if apq.abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q * m + q] - a[p * m + p]) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..m {
                    let akp = a[k * m + p];
                    let akq = a[k * m + q];
                    a[k * m + p] = c * akp - s * akq;
                    a[k * m + q] = s * akp + c * akq;
                }
                for k in 0..m {
                    let apk = a[p * m + k];
                    let aqk = a[q * m + k];
                    a[p * m + k] = c * apk - s * aqk;
                    a[q * m + k] = s * apk + c * aqk;
                }
            }
        }
    }
    let mut ev: Vec<f64> = (0..m).map(|i| a[i * m + i]).collect();
    ev.sort_by(|x, y| x.partial_cmp(y).unwrap());
    ev
}

/// Order parameters.
#[derive(Clone, Debug)]
pub struct Order {
    /// Particles whose coordination differs from 6 (interior only, for disks).
    pub defects: usize,
    /// Particles counted (all for periodic; interior for disks).
    pub counted: usize,
    /// `|mean psi6|` over counted particles.
    pub psi6_global: f64,
    /// mean `|psi6_i|` over counted particles.
    pub psi6_local: f64,
    /// Median nearest-neighbour distance.
    pub d0: f64,
}

/// Coordination with neighbours within `1.35 d0` (d0 = median nearest-neighbour distance) and psi6.
/// For disks, particles farther than `edge` from the centre of mass are excluded from the counts.
pub fn order(sys: &System, x: &[f64], edge: Option<f64>) -> Order {
    let n = x.len() / 2;
    let mut nn = vec![f64::INFINITY; n];
    for i in 0..n {
        for j in 0..n {
            if i != j {
                let (dx, dy) = sys.delta(x[2 * i], x[2 * i + 1], x[2 * j], x[2 * j + 1]);
                nn[i] = nn[i].min((dx * dx + dy * dy).sqrt());
            }
        }
    }
    let mut s = nn.clone();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let d0 = s[n / 2];
    let cut = 1.35 * d0;
    let (cx, cy) = ((0..n).map(|i| x[2 * i]).sum::<f64>() / n as f64, (0..n).map(|i| x[2 * i + 1]).sum::<f64>() / n as f64);
    let (mut defects, mut counted, mut re, mut im, mut loc) = (0usize, 0usize, 0.0, 0.0, 0.0);
    for i in 0..n {
        if let Some(rmax) = edge {
            if ((x[2 * i] - cx).powi(2) + (x[2 * i + 1] - cy).powi(2)).sqrt() > rmax {
                continue;
            }
        }
        let (mut c, mut pr, mut pi) = (0usize, 0.0, 0.0);
        for j in 0..n {
            if i == j {
                continue;
            }
            let (dx, dy) = sys.delta(x[2 * j], x[2 * j + 1], x[2 * i], x[2 * i + 1]);
            if (dx * dx + dy * dy).sqrt() < cut {
                c += 1;
                let th = dy.atan2(dx);
                pr += (6.0 * th).cos();
                pi += (6.0 * th).sin();
            }
        }
        counted += 1;
        if c != 6 {
            defects += 1;
        }
        if c > 0 {
            let (pr, pi) = (pr / c as f64, pi / c as f64);
            re += pr;
            im += pi;
            loc += (pr * pr + pi * pi).sqrt();
        }
    }
    let k = counted.max(1) as f64;
    Order { defects, counted, psi6_global: ((re / k).powi(2) + (im / k).powi(2)).sqrt(), psi6_local: loc / k, d0 }
}

/// Small deterministic RNG (xorshift64*), with Gaussian samples by Box-Muller.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    pub fn uniform(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn gauss(&mut self) -> f64 {
        let (u1, u2) = (self.uniform().max(1e-300), self.uniform());
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Uniform random configuration with minimum separation `dmin` (periodic box or disk of radius `rdisk`).
pub fn uniform_config(sys: &System, n: usize, dmin: f64, rdisk: Option<f64>, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut x: Vec<f64> = Vec::with_capacity(2 * n);
    while x.len() < 2 * n {
        let (px, py) = match (sys.geometry, rdisk) {
            (Geometry::Periodic { lx, ly }, _) => (rng.uniform() * lx, rng.uniform() * ly),
            (_, Some(r)) => {
                let (rr, th) = (r * rng.uniform().sqrt(), 2.0 * std::f64::consts::PI * rng.uniform());
                (rr * th.cos(), rr * th.sin())
            }
            _ => unreachable!("disk geometry needs rdisk"),
        };
        let ok = (0..x.len() / 2).all(|j| {
            let (dx, dy) = sys.delta(px, py, x[2 * j], x[2 * j + 1]);
            dx * dx + dy * dy >= dmin * dmin
        });
        if ok {
            x.push(px);
            x.push(py);
        }
    }
    x
}

/// Overdamped Langevin annealing (Euler-Maruyama), geometric temperature schedule `t0 -> t1` over `steps`;
/// each step's displacement is capped at `cap` to survive close encounters.
pub fn anneal(sys: &System, x0: &[f64], t0: f64, t1: f64, steps: usize, dt: f64, cap: f64, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut x = x0.to_vec();
    let mut f = vec![0.0; x.len()];
    let ratio = (t1 / t0).powf(1.0 / steps as f64);
    let mut temp = t0;
    for _ in 0..steps {
        sys.force(&x, &mut f);
        let amp = (2.0 * temp * dt).sqrt();
        for k in 0..x.len() {
            let d = (f[k] * dt + amp * rng.gauss()).clamp(-cap, cap);
            x[k] += d;
        }
        sys.wrap(&mut x);
        temp *= ratio;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_system(kappa: f64) -> (System, Vec<f64>) {
        let a = spacing_density_one();
        let sys = System { kappa, rc: 8.0, geometry: Geometry::Periodic { lx: 16.0 * a, ly: 9.0 * 3.0_f64.sqrt() * a } };
        (sys, triangular(16, 18, a))
    }

    #[test]
    fn force_matches_finite_difference_of_energy() {
        let (sys, mut x) = tri_system(2.0);
        let mut rng = Rng::new(7);
        x.iter_mut().for_each(|v| *v += 0.05 * rng.gauss());
        let mut f = vec![0.0; x.len()];
        sys.force(&x, &mut f);
        for &k in &[0usize, 17, 301] {
            let h = 1e-6;
            let (mut xp, mut xm) = (x.clone(), x.clone());
            xp[k] += h;
            xm[k] -= h;
            let fd = -(sys.energy(&xp) - sys.energy(&xm)) / (2.0 * h);
            assert!((fd - f[k]).abs() < 1e-6, "k={k} fd={fd} f={}", f[k]);
        }
    }

    #[test]
    fn hessian_matches_finite_difference_of_force() {
        let (sys, mut x) = tri_system(2.0);
        let mut rng = Rng::new(11);
        x.iter_mut().for_each(|v| *v += 0.05 * rng.gauss());
        let h = sys.hessian(&x);
        let m = x.len();
        for &k in &[3usize, 100] {
            let e = 1e-6;
            let (mut xp, mut xm) = (x.clone(), x.clone());
            xp[k] += e;
            xm[k] -= e;
            let (mut fp, mut fm) = (vec![0.0; m], vec![0.0; m]);
            sys.force(&xp, &mut fp);
            sys.force(&xm, &mut fm);
            for r in [k, k + 1, 50] {
                let fd = -(fp[r] - fm[r]) / (2.0 * e);
                assert!((fd - h[r * m + k]).abs() < 1e-5, "r={r} k={k} fd={fd} h={}", h[r * m + k]);
            }
        }
    }

    #[test]
    fn periodic_lattice_energy_equals_direct_lattice_sum() {
        let (sys, x) = tri_system(2.0);
        let a = spacing_density_one();
        let e_box = sys.energy(&x) / (x.len() / 2) as f64;
        let e_sum = lattice_energy(&sys, [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]]);
        assert!((e_box - e_sum).abs() < 1e-12 * e_sum.abs(), "box {e_box} vs sum {e_sum}");
        assert!(e_sum > 0.0);
    }

    #[test]
    fn jacobi_recovers_known_spectrum() {
        // tridiagonal (2, -1): eigenvalues 2 - 2 cos(k pi / (m+1))
        let m = 12;
        let mut a = vec![0.0; m * m];
        for i in 0..m {
            a[i * m + i] = 2.0;
            if i + 1 < m {
                a[i * m + i + 1] = -1.0;
                a[(i + 1) * m + i] = -1.0;
            }
        }
        let ev = symmetric_eigenvalues(&a, m);
        for (k, v) in ev.iter().enumerate() {
            let exact = 2.0 - 2.0 * ((k + 1) as f64 * std::f64::consts::PI / (m + 1) as f64).cos();
            assert!((v - exact).abs() < 1e-10, "{v} vs {exact}");
        }
    }

    #[test]
    fn square_lattice_costs_more_than_triangular() {
        let a = spacing_density_one();
        let sys = System { kappa: 2.0, rc: 8.0, geometry: Geometry::Bowl { k: 0.0 } };
        let e_tri = lattice_energy(&sys, [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]]);
        let e_sq = lattice_energy(&sys, [[1.0, 0.0], [0.0, 1.0]]);
        assert!(e_sq > e_tri, "square {e_sq} triangular {e_tri}");
    }
}
