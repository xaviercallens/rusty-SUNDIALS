//! Harmonic lattice phonons of planar Bravais lattices of unit covolume (Tier X numerics).
//!
//! A lattice shape is a point `tau = x + i y` of the upper half plane. The basis is
//! `a1 = (1/sqrt y, 0)`, `a2 = (x/sqrt y, sqrt y)`, so the covolume is one. For a radial pair potential `V(r)` the
//! force-constant tensor is `H(R) = (V'/r) I + (V'' - V'/r) n n^T`, `n = R/|R|`, and the dynamical matrix
//! (units `hbar = m = 1`, one particle per cell) is
//! `D(k) = sum_{R != 0} (1 - cos k.R) H(R)`.
//! The harmonic zero-point energy per particle is `E_zp = (1/2) <sum_branches sqrt(eig D(k))>_BZ`.
//!
//! Method (no Ewald is needed for `D`, because `H` decays like `r^{-s-2}` for a Riesz `r^{-s}`, `s > 2`):
//! * `k = (i/n) b1 + (j/n) b2` with `b_i . a_j = 2 pi delta_ij`, so `k.R = 2 pi (i m + j l)/n` for `R = m a1 + l a2`.
//!   The tensors `H(R)` are folded once into `n x n` cells by `(m mod n, l mod n)` and `D(k)` for all grid points is a
//!   separable discrete cosine transform of the folded tensors, `O(n^3)` instead of `O(n^2 #R)`.
//! * the sum over `|R| < rc` is completed by the non-oscillating continuum tail `-pi rc V'(rc) I` (the oscillating part
//!   of the tail is `O(rc^{-4})` and is measured by varying `rc`).
//! * `k = 0` contributes zero (both acoustic branches vanish) and is counted in the average.
//!
//! Stability is classified, never clipped: a grid point with an eigenvalue below `-STABLE_TOL * scale` marks the lattice
//! dynamically unstable, and the zero-point energy of such a lattice is not defined (reported as `NaN`).
//!
//! Coulomb and Keldysh potentials are excluded: they need a neutralising background and plasmon treatment of the
//! `k -> 0` limit, which this module does not implement.

#![allow(clippy::needless_range_loop)]

use std::f64::consts::PI;

/// Relative tolerance below which a negative eigenvalue of `D(k)` is regarded as round-off.
pub const STABLE_TOL: f64 = 1e-9;

/// Radial pair potentials (and finite sums of them).
#[derive(Clone, Debug)]
pub enum Pot {
    /// `exp(-pi a r^2)`
    Gaussian { a: f64 },
    /// `exp(-kappa r)/r`
    Yukawa { kappa: f64 },
    /// `r^{-s}`, integer `s >= 3`
    Riesz { s: i32 },
    /// Bilayer dipolar exciton `U = V/d^2 = 2/(r s (r+s))`, `s = sqrt(r^2+d^2)`
    Dipolar { d: f64 },
    /// Gaussian ring `exp(-(r-r0)^2/(2 w^2))` (not completely monotone)
    Ring { r0: f64, w: f64 },
    /// Morse `exp(-2 alpha (r-r0)) - 2 exp(-alpha (r-r0))` (not completely monotone)
    Morse { alpha: f64, r0: f64 },
    /// `sum_i c_i V_i`
    Sum(Vec<(f64, Pot)>),
}

impl Pot {
    /// Parses `gauss:a`, `yukawa:kappa`, `riesz:s`, `dipolar:d`, `ring:r0,w`, `morse:alpha,r0`,
    /// `wells:r1,r2,w,c` (`-ring(r1,w) - c ring(r2,w)`), `lj:sigma` (4[(sigma/r)^12 - (sigma/r)^6]) or `gdiff:a1,a2,c` (`exp(-pi a1 r^2) - c exp(-pi a2 r^2)`).
    pub fn parse(spec: &str) -> Result<Pot, String> {
        let (name, args) = spec
            .split_once(':')
            .ok_or_else(|| format!("bad potential spec {spec}"))?;
        let v: Vec<f64> = args
            .split(',')
            .map(|t| t.parse::<f64>().map_err(|e| format!("{spec}: {e}")))
            .collect::<Result<_, _>>()?;
        let need = |n: usize| -> Result<(), String> {
            if v.len() == n {
                Ok(())
            } else {
                Err(format!("{spec}: expected {n} parameters"))
            }
        };
        match name {
            "gauss" => {
                need(1)?;
                Ok(Pot::Gaussian { a: v[0] })
            }
            "yukawa" => {
                need(1)?;
                Ok(Pot::Yukawa { kappa: v[0] })
            }
            "riesz" => {
                need(1)?;
                Ok(Pot::Riesz { s: v[0] as i32 })
            }
            "dipolar" => {
                need(1)?;
                Ok(Pot::Dipolar { d: v[0] })
            }
            "ring" => {
                need(2)?;
                Ok(Pot::Ring { r0: v[0], w: v[1] })
            }
            "morse" => {
                need(2)?;
                Ok(Pot::Morse {
                    alpha: v[0],
                    r0: v[1],
                })
            }
            "lj" => {
                need(1)?;
                Ok(Pot::Sum(vec![
                    (4.0 * v[0].powi(12), Pot::Riesz { s: 12 }),
                    (-4.0 * v[0].powi(6), Pot::Riesz { s: 6 }),
                ]))
            }
            "gdiff" => {
                need(3)?;
                Ok(Pot::Sum(vec![
                    (1.0, Pot::Gaussian { a: v[0] }),
                    (-v[2], Pot::Gaussian { a: v[1] }),
                ]))
            }
            "wells" => {
                need(4)?;
                Ok(Pot::Sum(vec![
                    (-1.0, Pot::Ring { r0: v[0], w: v[2] }),
                    (-v[3], Pot::Ring { r0: v[1], w: v[2] }),
                ]))
            }
            _ => Err(format!("unknown potential {name}")),
        }
    }

    /// `(V, V', V'')` at radius `r > 0`.
    pub fn radial(&self, r: f64) -> (f64, f64, f64) {
        match self {
            Pot::Gaussian { a } => {
                let v = (-PI * a * r * r).exp();
                (
                    v,
                    -2.0 * PI * a * r * v,
                    (-2.0 * PI * a + 4.0 * PI * PI * a * a * r * r) * v,
                )
            }
            Pot::Yukawa { kappa: k } => {
                let e = (-k * r).exp();
                (
                    e / r,
                    -e * (k * r + 1.0) / (r * r),
                    e * (k * k * r * r + 2.0 * k * r + 2.0) / (r * r * r),
                )
            }
            Pot::Riesz { s } => {
                let sf = f64::from(*s);
                let v = r.powi(-*s);
                (v, -sf * v / r, sf * (sf + 1.0) * v / (r * r))
            }
            Pot::Dipolar { d } => crate::dipolar::u_pair(*d, r),
            Pot::Ring { r0, w } => {
                let z = r - r0;
                let v = (-z * z / (2.0 * w * w)).exp();
                (
                    v,
                    -z / (w * w) * v,
                    (z * z / (w * w * w * w) - 1.0 / (w * w)) * v,
                )
            }
            Pot::Morse { alpha: al, r0 } => {
                let e1 = (-al * (r - r0)).exp();
                let e2 = e1 * e1;
                (
                    e2 - 2.0 * e1,
                    -2.0 * al * e2 + 2.0 * al * e1,
                    4.0 * al * al * e2 - 2.0 * al * al * e1,
                )
            }
            Pot::Sum(parts) => {
                let mut acc = (0.0, 0.0, 0.0);
                for (c, p) in parts {
                    let (a, b, d) = p.radial(r);
                    acc.0 += c * a;
                    acc.1 += c * b;
                    acc.2 += c * d;
                }
                acc
            }
        }
    }

    /// Radius beyond which the potential is below `1e-17`; `None` for power laws.
    pub fn natural_cut(&self) -> Option<f64> {
        match self {
            Pot::Gaussian { a } => Some((41.0 / (PI * a)).sqrt()),
            Pot::Yukawa { kappa } => Some(42.0 / kappa),
            Pot::Ring { r0, w } => Some(r0 + 9.0 * w),
            Pot::Morse { alpha, r0 } => Some(r0 + 42.0 / alpha),
            Pot::Riesz { .. } | Pot::Dipolar { .. } => None,
            Pot::Sum(parts) => {
                let mut m: f64 = 0.0;
                for (_, p) in parts {
                    m = m.max(p.natural_cut()?);
                }
                Some(m)
            }
        }
    }

    /// `(1/2) int_rc^inf 2 pi r V(r) dr` (energy per particle of the uniform continuum beyond `rc`, density one).
    pub fn tail_energy(&self, rc: f64) -> f64 {
        match self {
            Pot::Riesz { s } => PI * rc.powi(2 - *s) / f64::from(*s - 2),
            Pot::Dipolar { d } => crate::dipolar::tail(*d, 1.0, rc),
            Pot::Sum(parts) => parts.iter().map(|(c, p)| c * p.tail_energy(rc)).sum(),
            _ => 0.0,
        }
    }

    /// Continuum tail of `sum_R H(R)` beyond `rc`: `-pi rc V'(rc)` times the identity.
    pub fn tail_force_constant(&self, rc: f64) -> f64 {
        -PI * rc * self.radial(rc).1
    }
}

/// Lattice vectors `R = m a1 + l a2` with `0 < |R| < rc` for the shape `tau = x + i y`, as `(m, l, X, Y, r)`.
fn lattice_vectors(x: f64, y: f64, rc: f64) -> Vec<(i64, i64, f64, f64, f64)> {
    let sy = y.sqrt();
    let lmax = (rc / sy).floor() as i64;
    let mut out = Vec::new();
    for l in -lmax..=lmax {
        let yy = l as f64 * sy;
        let rem = rc * rc - yy * yy;
        if rem <= 0.0 {
            continue;
        }
        let xmax = rem.sqrt();
        let mlo = (-xmax * sy - l as f64 * x).ceil() as i64;
        let mhi = (xmax * sy - l as f64 * x).floor() as i64;
        for m in mlo..=mhi {
            let xx = (m as f64 + l as f64 * x) / sy;
            let r2 = xx * xx + yy * yy;
            if r2 == 0.0 || r2 >= rc * rc {
                continue;
            }
            out.push((m, l, xx, yy, r2.sqrt()));
        }
    }
    out
}

/// Result of one lattice analysis.
#[derive(Clone, Debug)]
pub struct Analysis {
    pub x: f64,
    pub y: f64,
    pub n: usize,
    pub rc: f64,
    /// Static energy per particle `(1/2) sum_{R != 0} V(R)` (truncated sum plus continuum tail).
    pub e_static: f64,
    /// Zero-point energy per particle; `NaN` when the lattice is dynamically unstable.
    pub e_zp: f64,
    /// Zero-point energy with imaginary frequencies set to zero. This is a lower bound for an unstable lattice (it
    /// favours the competitor) and equals `e_zp` for a stable one; it is never used to claim a minimum on its own.
    pub e_zp_clip: f64,
    /// Smallest eigenvalue of `D(k)` over the grid.
    pub lam_min: f64,
    /// Number of grid points with an eigenvalue below `-STABLE_TOL * scale`.
    pub n_unstable: usize,
}

/// Dynamical matrices `[Dxx, Dxy, Dyy]` on the `n x n` grid, index `i*n + j`, for the shape `tau = x + i y`.
/// `use_tail = false` leaves the sum truncated at `rc` (used to compare with a truncated direct supercell sum).
pub fn dynamical_grid(
    pot: &Pot,
    x: f64,
    y: f64,
    n: usize,
    rc_request: f64,
    use_tail: bool,
) -> Vec<[f64; 3]> {
    let rc = pot.natural_cut().map_or(rc_request, |c| c.min(rc_request));
    let vecs = lattice_vectors(x, y, rc);
    let mut fold = vec![[0.0f64; 3]; n * n];
    let mut total = [0.0f64; 3];
    for &(m, l, xx, yy, r) in &vecs {
        let (_, v1, v2) = pot.radial(r);
        let a = v1 / r;
        let b = v2 - a;
        let (nx, ny) = (xx / r, yy / r);
        let h = [a + b * nx * nx, b * nx * ny, a + b * ny * ny];
        let p = m.rem_euclid(n as i64) as usize;
        let q = l.rem_euclid(n as i64) as usize;
        for c in 0..3 {
            fold[p * n + q][c] += h[c];
            total[c] += h[c];
        }
    }
    if use_tail {
        let t = pot.tail_force_constant(rc);
        total[0] += t;
        total[2] += t;
    }
    let (cs, sn): (Vec<f64>, Vec<f64>) = (0..n)
        .map(|t| {
            let th = 2.0 * PI * t as f64 / n as f64;
            (th.cos(), th.sin())
        })
        .unzip();
    let mut dm = vec![[0.0f64; 3]; n * n];
    for c in 0..3 {
        // A[p][j] = sum_q f[p][q] exp(i 2 pi j q / n)
        let mut a_re = vec![0.0f64; n * n];
        let mut a_im = vec![0.0f64; n * n];
        for p in 0..n {
            for j in 0..n {
                let (mut re, mut im) = (0.0, 0.0);
                let mut idx = 0usize;
                for q in 0..n {
                    let f = fold[p * n + q][c];
                    re += f * cs[idx];
                    im += f * sn[idx];
                    idx += j;
                    if idx >= n {
                        idx %= n;
                    }
                }
                a_re[p * n + j] = re;
                a_im[p * n + j] = im;
            }
        }
        for i in 0..n {
            for j in 0..n {
                let mut re = 0.0;
                let mut idx = 0usize;
                for p in 0..n {
                    re += a_re[p * n + j] * cs[idx] - a_im[p * n + j] * sn[idx];
                    idx += i;
                    if idx >= n {
                        idx %= n;
                    }
                }
                dm[i * n + j][c] = total[c] - re;
            }
        }
    }
    dm[0] = [0.0; 3];
    dm
}

/// Eigenvalues `(lo, hi)` of the symmetric 2x2 matrix `[[a, b], [b, c]]`.
pub fn eig2(a: f64, b: f64, c: f64) -> (f64, f64) {
    let mid = 0.5 * (a + c);
    let rad = (0.25 * (a - c) * (a - c) + b * b).sqrt();
    (mid - rad, mid + rad)
}

/// Static energy per particle of the Bravais lattice (truncated sum plus uniform-continuum tail).
pub fn static_energy(pot: &Pot, x: f64, y: f64, rc_request: f64) -> f64 {
    let rc = pot.natural_cut().map_or(rc_request, |c| c.min(rc_request));
    let s: f64 = lattice_vectors(x, y, rc)
        .iter()
        .map(|v| pot.radial(v.4).0)
        .sum();
    0.5 * s + pot.tail_energy(rc)
}

/// Full analysis of the lattice `tau = x + i y` on an `n x n` Gamma-centred grid.
pub fn analyze(pot: &Pot, x: f64, y: f64, n: usize, rc: f64) -> Analysis {
    let dm = dynamical_grid(pot, x, y, n, rc, true);
    let scale = dm
        .iter()
        .map(|d| d[0] + d[2])
        .fold(0.0f64, f64::max)
        .max(1e-300);
    let mut sum = 0.0;
    let mut sum_clip = 0.0;
    let mut lam_min = f64::INFINITY;
    let mut n_unstable = 0usize;
    for d in &dm {
        let (lo, hi) = eig2(d[0], d[1], d[2]);
        lam_min = lam_min.min(lo);
        let w = lo.max(0.0).sqrt() + hi.max(0.0).sqrt();
        sum_clip += w;
        if lo < -STABLE_TOL * scale {
            n_unstable += 1;
        } else {
            sum += w;
        }
    }
    let e_zp = if n_unstable == 0 {
        0.5 * sum / (n * n) as f64
    } else {
        f64::NAN
    };
    Analysis {
        x,
        y,
        n,
        rc,
        e_static: static_energy(pot, x, y, rc),
        e_zp,
        e_zp_clip: 0.5 * sum_clip / (n * n) as f64,
        lam_min,
        n_unstable,
    }
}

/// Reciprocal basis `(b1, b2)` of the shape `tau` (`b_i . a_j = 2 pi delta_ij`).
pub fn reciprocal(x: f64, y: f64) -> ([f64; 2], [f64; 2]) {
    let sy = y.sqrt();
    (
        [2.0 * PI * sy, -2.0 * PI * x * sy / y],
        [0.0, 2.0 * PI / sy],
    )
}

/// Frozen-phonon second difference of the pair-sum energy of an `n x n` supercell whose atoms are displaced by
/// `eps * e * cos(k.R)`, `k = (i/n) b1 + (j/n) b2`, using only values of `V` (no derivatives), images within
/// `rc_img`. Returns `[E(eps) + E(-eps) - 2 E(0)] / eps^2` per particle, which equals `(1/2) e^T D(k) e` up to `O(eps^2)`.
pub fn frozen_phonon_pair_sum(
    pot: &Pot,
    (x, y): (f64, f64),
    n: usize,
    ij: (usize, usize),
    e: [f64; 2],
    eps: f64,
    rc_img: f64,
) -> f64 {
    let vecs = lattice_vectors(x, y, rc_img);
    let phase = |m: i64, l: i64| -> f64 {
        let t = (ij.0 as i64 * m.rem_euclid(n as i64) + ij.1 as i64 * l.rem_euclid(n as i64))
            .rem_euclid(n as i64);
        (2.0 * PI * t as f64 / n as f64).cos()
    };
    let energy = |amp: f64| -> f64 {
        let mut tot = 0.0;
        for m0 in 0..n as i64 {
            for l0 in 0..n as i64 {
                let c0 = phase(m0, l0);
                for &(dm, dl, xx, yy, _) in &vecs {
                    let c1 = phase(m0 + dm, l0 + dl);
                    let dx = xx + amp * e[0] * (c1 - c0);
                    let dy = yy + amp * e[1] * (c1 - c0);
                    tot += pot.radial(dx.hypot(dy)).0;
                }
            }
        }
        0.5 * tot / (n * n) as f64
    };
    (energy(eps) + energy(-eps) - 2.0 * energy(0.0)) / (eps * eps)
}

/// Same frozen-phonon second difference for the untruncated dipolar-exciton potential (`U` units), using the
/// Ewald energy of the commensurate supercell as the independent energy routine.
pub fn frozen_phonon_dipolar_ewald(
    d: f64,
    x: f64,
    y: f64,
    n: usize,
    ij: (usize, usize),
    e: [f64; 2],
    eps: f64,
) -> f64 {
    use crate::dipolar_ewald::DipolarEwald;
    let sy = y.sqrt();
    let a1 = [1.0 / sy, 0.0];
    let a2 = [x / sy, sy];
    let big = DipolarEwald::new(
        d,
        0.9,
        [n as f64 * a1[0], 0.0],
        [n as f64 * a2[0], n as f64 * a2[1]],
    );
    let energy = |amp: f64| -> f64 {
        let mut pos = Vec::with_capacity(2 * n * n);
        for m in 0..n {
            for l in 0..n {
                let t = (ij.0 * m + ij.1 * l) % n;
                let c = (2.0 * PI * t as f64 / n as f64).cos();
                pos.push(m as f64 * a1[0] + l as f64 * a2[0] + amp * e[0] * c);
                pos.push(m as f64 * a1[1] + l as f64 * a2[1] + amp * e[1] * c);
            }
        }
        big.energy_per_particle(&pos)
    };
    (energy(eps) + energy(-eps) - 2.0 * energy(0.0)) / (eps * eps)
}

/// Fundamental-domain test (`|Re tau| <= 1/2`, `|tau| >= 1`).
pub fn in_fundamental_domain(x: f64, y: f64) -> bool {
    x.abs() <= 0.5 + 1e-12 && x * x + y * y >= 1.0 - 1e-12 && y > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRI_Y: f64 = 0.866_025_403_784_438_6;

    #[test]
    fn eig2_matches_known_matrix() {
        let (lo, hi) = eig2(2.0, 1.0, 2.0);
        assert!((lo - 1.0).abs() < 1e-14 && (hi - 3.0).abs() < 1e-14);
    }

    #[test]
    fn lattice_vectors_square_count_and_symmetry() {
        let v = lattice_vectors(0.0, 1.0, 10.0);
        let mut brute = 0usize;
        for a in -11i64..=11 {
            for b in -11i64..=11 {
                let r2 = a * a + b * b;
                if r2 > 0 && r2 < 100 {
                    brute += 1;
                }
            }
        }
        assert_eq!(v.len(), brute);
        let sum_x: f64 = v.iter().map(|t| t.2).sum();
        assert!(sum_x.abs() < 1e-9);
    }

    #[test]
    fn mean_trace_equals_folded_sum() {
        // BZ mean of Tr D = sum_{R not = 0 mod n} Tr H(R) (+ tail), an identity of the transform
        let pot = Pot::Gaussian { a: 0.7 };
        let n = 12;
        let dm = dynamical_grid(&pot, 0.2, 1.3, n, 1e9, true);
        let mean: f64 = dm.iter().map(|d| d[0] + d[2]).sum::<f64>() / (n * n) as f64;
        let rc = pot.natural_cut().unwrap();
        let mut expect = pot.tail_force_constant(rc) * 2.0;
        for &(m, l, _, _, r) in &lattice_vectors(0.2, 1.3, rc) {
            if m.rem_euclid(n as i64) != 0 || l.rem_euclid(n as i64) != 0 {
                let (_, v1, v2) = pot.radial(r);
                expect += v2 + v1 / r;
            }
        }
        assert!((mean - expect).abs() < 1e-11, "{mean} vs {expect}");
        assert!(dm[0] == [0.0; 3] && dm[1][0].abs() + dm[1][2].abs() > 0.0);
    }

    #[test]
    fn gaussian_static_energy_matches_theta_functions() {
        // square lattice: E = (theta3(e^{-pi a})^2 - 1)/2; hexagonal self-duality: theta_L(a) = theta_L(1/a)/a
        let a: f64 = 0.8;
        let q = (-PI * a).exp();
        let th3 = 1.0 + 2.0 * (1..40).map(|k| q.powi(k * k)).sum::<f64>();
        let e = static_energy(&Pot::Gaussian { a }, 0.0, 1.0, 1e9);
        assert!((e - 0.5 * (th3 * th3 - 1.0)).abs() < 1e-13);
        let tri = |a: f64| 1.0 + 2.0 * static_energy(&Pot::Gaussian { a }, 0.5, TRI_Y, 1e9);
        assert!((tri(0.8) - tri(1.0 / 0.8) / 0.8).abs() < 1e-12);
    }

    #[test]
    fn modular_invariance_of_phonon_energy() {
        // E(tau) = E(tau + 1) = E(-1/tau): same lattice, different basis, different cell and fold
        let pot = Pot::Yukawa { kappa: 1.0 };
        let (x, y) = (0.46, 0.93);
        let a = analyze(&pot, x, y, 24, 1e9);
        let b = analyze(&pot, x + 1.0, y, 24, 1e9);
        let nr = x * x + y * y;
        let c = analyze(&pot, -x / nr, y / nr, 24, 1e9);
        assert!(a.e_zp.is_finite());
        assert!((a.e_zp - c.e_zp).abs() < 3e-3 * a.e_zp);
        assert!((a.e_static - b.e_static).abs() < 1e-12);
        assert!((a.e_static - c.e_static).abs() < 1e-12);
        // the zero-point energy converges as n^-3 so compare at fine grid
        let a2 = analyze(&pot, x, y, 96, 1e9);
        let b2 = analyze(&pot, x + 1.0, y, 96, 1e9);
        let c2 = analyze(&pot, -x / nr, y / nr, 96, 1e9);
        assert!((a2.e_zp - b2.e_zp).abs() < 1e-5 * a2.e_zp, "{a2:?} {b2:?}");
        assert!((a2.e_zp - c2.e_zp).abs() < 1e-5 * a2.e_zp, "{a2:?} {c2:?}");
    }

    #[test]
    fn frozen_phonon_matches_dynamical_matrix_gaussian_and_yukawa() {
        let n = 5;
        let ij = (1usize, 2usize);
        let (x, y) = (0.5, TRI_Y);
        for pot in [Pot::Gaussian { a: 0.6 }, Pot::Yukawa { kappa: 1.5 }] {
            let rc = pot.natural_cut().unwrap().min(14.0);
            let dm = dynamical_grid(&pot, x, y, n, rc, false);
            let d = dm[ij.0 * n + ij.1];
            for e in [[1.0, 0.0], [0.0, 1.0], [0.6, 0.8]] {
                let exact =
                    0.5 * (e[0] * e[0] * d[0] + 2.0 * e[0] * e[1] * d[1] + e[1] * e[1] * d[2]);
                let f1 = frozen_phonon_pair_sum(&pot, (x, y), n, ij, e, 2e-3, rc);
                let f2 = frozen_phonon_pair_sum(&pot, (x, y), n, ij, e, 4e-3, rc);
                let rich = (4.0 * f1 - f2) / 3.0;
                assert!(
                    (rich - exact).abs() < 1e-7 * exact.abs().max(1.0),
                    "{pot:?} {e:?}: {rich} vs {exact}"
                );
            }
        }
    }

    #[test]
    fn frozen_phonon_matches_dynamical_matrix_dipolar_ewald() {
        let (n, ij, d) = (5usize, (1usize, 2usize), 0.5);
        let (x, y) = (0.5, TRI_Y);
        let pot = Pot::Dipolar { d };
        let dm = dynamical_grid(&pot, x, y, n, 400.0, true);
        let dd = dm[ij.0 * n + ij.1];
        let e = [0.6, 0.8];
        let exact = 0.5 * (e[0] * e[0] * dd[0] + 2.0 * e[0] * e[1] * dd[1] + e[1] * e[1] * dd[2]);
        let f1 = frozen_phonon_dipolar_ewald(d, x, y, n, ij, e, 2e-3);
        let f2 = frozen_phonon_dipolar_ewald(d, x, y, n, ij, e, 4e-3);
        let rich = (4.0 * f1 - f2) / 3.0;
        assert!(
            (rich - exact).abs() < 1e-6 * exact.abs(),
            "{rich} vs {exact}"
        );
    }

    #[test]
    fn riesz_static_energy_matches_epstein_closed_forms() {
        // Z^2, s=4: sum' (m^2+n^2)^-2 = 4 zeta(2) beta(2); s=6: 4 zeta(3) beta(3), beta(3) = pi^3/32
        let cat = 0.915_965_594_177_219_0;
        let z2 = PI * PI / 6.0;
        let z3 = 1.202_056_903_159_594_3;
        let e4 = static_energy(&Pot::Riesz { s: 4 }, 0.0, 1.0, 400.0);
        assert!((e4 - 0.5 * 4.0 * z2 * cat).abs() < 1e-9, "{e4}");
        let e6 = static_energy(&Pot::Riesz { s: 6 }, 0.0, 1.0, 400.0);
        assert!(
            (e6 - 0.5 * 4.0 * z3 * PI.powi(3) / 32.0).abs() < 1e-11,
            "{e6}"
        );
    }

    #[test]
    fn unstable_lattice_is_flagged_not_clipped() {
        // a Gaussian ring with the triangular nearest-neighbour distance as radius makes the triangular lattice a
        // static maximum-like structure; whatever the eigenvalues are, a negative one must give NaN, never a number
        let pot = Pot::Ring {
            r0: 1.07457,
            w: 0.05,
        };
        let mut flagged = 0;
        for k in 0..20 {
            let y = 0.9 + 0.05 * f64::from(k);
            let a = analyze(&pot, 0.0, y, 24, 50.0);
            if a.n_unstable > 0 {
                assert!(a.e_zp.is_nan());
                flagged += 1;
            } else {
                assert!(a.e_zp.is_finite());
            }
        }
        assert!(
            flagged > 0,
            "the ring potential must destabilise some lattice"
        );
    }

    #[test]
    fn qu1_gate_gaussian_counterexample_is_stable_and_lower_while_statics_obey_montgomery() {
        // Gaussian exp(-pi r^2) is completely monotone in r^2. A dynamically stable rhombic lattice has a LOWER
        // harmonic zero-point energy than the triangular one, while its static energy is higher (Montgomery).
        let pot = Pot::Gaussian { a: 1.0 };
        let tri = analyze(&pot, 0.5, TRI_Y, 96, 400.0);
        let rival = analyze(&pot, 0.5, 0.966_025_403_784, 96, 400.0);
        assert_eq!((tri.n_unstable, rival.n_unstable), (0, 0));
        assert!(
            rival.e_zp < tri.e_zp - 1e-2,
            "{} vs {}",
            rival.e_zp,
            tri.e_zp
        );
        assert!(rival.e_static > tri.e_static);
    }

    #[test]
    fn qu1_gate_narrow_gaussian_keeps_triangular_locally_minimal() {
        // opposite curvature at a narrow Gaussian: stable neighbours of the triangular lattice have higher E_zp
        let pot = Pot::Gaussian { a: 4.0 };
        let tri = analyze(&pot, 0.5, TRI_Y, 96, 400.0);
        for (x, y) in [
            (0.45, TRI_Y),
            (0.5, TRI_Y + 0.05),
            (0.55, TRI_Y),
            (0.5, TRI_Y - 0.05),
        ] {
            let r = analyze(&pot, x, y, 96, 400.0);
            assert_eq!(r.n_unstable, 0);
            assert!(r.e_zp > tri.e_zp, "({x},{y}): {} vs {}", r.e_zp, tri.e_zp);
        }
    }

    #[test]
    fn qu1_gate_negative_control_search_finds_counterexamples_for_non_cm_potential() {
        // two attractive Gaussian wells at the square nearest/next-nearest distances: the square lattice is the
        // stable static minimiser and a stable rhombic lattice has a lower zero-point energy than the triangular one
        let pot = Pot::parse("wells:1.0,1.414213562373,0.12,0.5").expect("spec");
        let tri = analyze(&pot, 0.5, TRI_Y, 48, 400.0);
        let square = analyze(&pot, 0.0, 1.0, 48, 400.0);
        let rhombic = analyze(&pot, 0.15, 1.188_685_996_664, 48, 400.0);
        assert_eq!(
            (tri.n_unstable, square.n_unstable, rhombic.n_unstable),
            (0, 0, 0)
        );
        assert!(square.e_static < tri.e_static - 0.4);
        assert!(rhombic.e_zp < tri.e_zp - 1.0);
    }
}
