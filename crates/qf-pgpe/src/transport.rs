//! Estimators of vortex transport coefficients from tracked positions: a Rust port of
//! `exploration/pgpe/transport_estimators.py` of SocrateAI-Scientific-QuantumFluids (amendment A1 of
//! `PGPE_FRICTION_PREREG.md`, `PGPE_ALPHAPRIME_PREREG.md`, `PGPE_EINSTEIN_PREREG.md`), plus the synthetic generator of
//! gate G0.
//!
//! Model (`hbar = m = 1`, circulation `2 pi`, normal fluid at rest; `lean_src/DissipativeVortexDynamics.lean`):
//!
//! ```text
//! dr_i/dt = (1 - alpha') v_s,i - alpha q_i z x v_s,i + sqrt(2 eta) xi_i
//! ```
//!
//! with `v_s,i` the torus point-vortex velocity induced by the other vortices (Weiss-McWilliams pair function
//! `h ~ 2 ln r`, [`grad_h`], [`pv_velocity`]).
//!
//! | Python (`transport_estimators.py`) | Rust |
//! |---|---|
//! | `grad_h`, `h_pair`, `pv_velocity`, `pv_energy`, `unwrap`, `predictors` | same names |
//! | `analyse_tracks(tracks, L, lag, t_settle, d_valid, lags_eta)` | [`analyse_tracks`] with [`Options`] |
//! | `antiparallel`, `langevin`, `gate_G0` | [`vortex::antiparallel`], [`langevin`], [`gate_g0`] |
//!
//! # Numerical fidelity
//!
//! The port follows the Python expression order everywhere, and reproduces numpy's reductions where they matter:
//! `ndarray.sum()` is numpy's pairwise summation ([`np_sum`]), `cumsum` and the block sums are sequential, `np.round`
//! is round-half-to-even, `%` is the floored modulus ([`f64::rem_euclid`]), `np.polyfit(x, y, 1)` is an ordinary
//! least-squares line ([`polyfit1`]; numpy solves the same problem by an SVD of the column-scaled Vandermonde matrix)
//! and `np.linalg.solve` of the 2x2 normal equations is an LU solve with partial pivoting ([`solve2`]). What cannot be
//! identical is the libm vs numpy SIMD `sinh/cosh/sin/cos/log`: differences of a few ulps in the pair function, which
//! the regression propagates to a relative `~1e-13` on the estimates (see `tests/transport_crosscheck.rs`).
use crate::vortex::{self, Lcg, min_opposite_distance};
use std::f64::consts::PI;

const TWO_PI: f64 = 2.0 * PI;

/// Number of images in the `m`-sum of the pair function (`M = 4` of `grad_h` / `h_pair`).
const M_IMAGES: i32 = 4;

/// Reduce `x` to `(-pi, pi]` as numpy does: `(x + pi) % 2 pi - pi` (floored modulus).
fn reduce(x: f64) -> f64 {
    (x + PI).rem_euclid(TWO_PI) - PI
}

/// `grad_h`: gradient `(h_x, h_y)` of the Weiss-McWilliams pair function on the `2 pi` box (arguments are reduced to
/// `(-pi, pi]`; sum over `2 M_IMAGES + 1` images).
pub fn grad_h(x: f64, y: f64) -> (f64, f64) {
    let (x, y) = (reduce(x), reduce(y));
    let mut hx = -x / PI;
    let mut hy = 0.0;
    for m in -M_IMAGES..=M_IMAGES {
        let xm = x - TWO_PI * m as f64;
        let den = xm.cosh() - y.cos();
        hx += xm.sinh() / den;
        hy += y.sin() / den;
    }
    (hx, hy)
}

/// `h_pair`: the Weiss-McWilliams pair function itself (`~ 2 ln r` at short range).
pub fn h_pair(x: f64, y: f64) -> f64 {
    let (x, y) = (reduce(x), reduce(y));
    let mut s = -(x * x) / TWO_PI;
    for m in -M_IMAGES..=M_IMAGES {
        let xm = x - TWO_PI * m as f64;
        s += ((xm.cosh() - y.cos()) / (TWO_PI * m as f64).cosh()).ln();
    }
    s
}

/// numpy's pairwise summation (`DOUBLE_pairwise_sum`), as used by `ndarray.sum()` on a contiguous array.
pub fn np_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for &v in a {
            res += v;
        }
        res
    } else if n <= 128 {
        let mut r = [0.0; 8];
        r.copy_from_slice(&a[..8]);
        let tail = n - n % 8;
        let mut i = 8;
        while i < tail {
            for (j, rj) in r.iter_mut().enumerate() {
                *rj += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        for &v in &a[tail..] {
            res += v;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        np_sum(&a[..n2]) + np_sum(&a[n2..])
    }
}

/// `pv_velocity`: `v_s` at every vortex, `psi_i = -sum_j q_j h_ij / 2`, `v = (d psi/dy, -d psi/dx)`; positions in
/// box units of side `l`.
pub fn pv_velocity(pos: &[(f64, f64)], q: &[i32], l: f64) -> Vec<(f64, f64)> {
    let sc = TWO_PI / l;
    let n = q.len();
    let mut out = Vec::with_capacity(n);
    let mut tx = vec![0.0; n];
    let mut ty = vec![0.0; n];
    for i in 0..n {
        for j in 0..n {
            if i == j {
                tx[j] = 0.0;
                ty[j] = 0.0;
                continue;
            }
            let dx = (pos[i].0 - pos[j].0) * sc;
            let dy = (pos[i].1 - pos[j].1) * sc;
            let (hx, hy) = grad_h(dx, dy);
            tx[j] = hx * q[j] as f64;
            ty[j] = hy * q[j] as f64;
        }
        let psix = -0.5 * sc * np_sum(&tx);
        let psiy = -0.5 * sc * np_sum(&ty);
        out.push((psiy, -psix));
    }
    out
}

/// `pv_energy`: point-vortex energy `-sum_{i<j} q_i q_j h(r_ij)`.
pub fn pv_energy(pos: &[(f64, f64)], q: &[i32], l: f64) -> f64 {
    let sc = TWO_PI / l;
    let n = q.len();
    let mut terms = Vec::with_capacity(n * n.saturating_sub(1) / 2);
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = (pos[i].0 - pos[j].0) * sc;
            let dy = (pos[i].1 - pos[j].1) * sc;
            terms.push((q[i] * q[j]) as f64 * h_pair(dx, dy));
        }
    }
    -np_sum(&terms)
}

/// One tracked run: sample times, positions `r[time][vortex]` (box coordinates, wrapped or unwrapped) and the charges.
#[derive(Debug, Clone)]
pub struct Track {
    pub t: Vec<f64>,
    pub r: Vec<Vec<(f64, f64)>>,
    pub q: Vec<i32>,
}

/// `unwrap`: positions on the torus -> continuous trajectories (`np.diff`, `d -= L round(d/L)`, `cumsum`).
pub fn unwrap(r: &[Vec<(f64, f64)>], l: f64) -> Vec<Vec<(f64, f64)>> {
    let Some(first) = r.first() else {
        return Vec::new();
    };
    let mut acc = vec![(0.0, 0.0); first.len()];
    let mut out = Vec::with_capacity(r.len());
    out.push(first.clone());
    for k in 1..r.len() {
        let mut row = Vec::with_capacity(first.len());
        for (i, a) in acc.iter_mut().enumerate() {
            let mut dx = r[k][i].0 - r[k - 1][i].0;
            let mut dy = r[k][i].1 - r[k - 1][i].1;
            dx -= l * (dx / l).round_ties_even();
            dy -= l * (dy / l).round_ties_even();
            a.0 += dx;
            a.1 += dy;
            row.push((first[i].0 + a.0, first[i].1 + a.1));
        }
        out.push(row);
    }
    out
}

/// Output of [`predictors`]; the `ca`/`cb` arrays are flat with layout `[time][vortex][x, y]`.
#[derive(Debug, Clone)]
pub struct Predictors {
    /// `CA = int v_s dt` (cumulative trapezoid)
    pub ca: Vec<f64>,
    /// `CB = int (-q z x v_s) dt`
    pub cb: Vec<f64>,
    /// point-vortex energy `H(t)`
    pub h: Vec<f64>,
    /// `S(t) = sum_i |v_s,i|^2`
    pub s: Vec<f64>,
}

/// `predictors`: cumulative trapezoid integrals `CA = int v_s dt` and `CB = int (-q z x v_s) dt`, the point-vortex
/// energy `H(t)` and `S(t) = sum_i |v_s,i|^2` along a track (`r` unwrapped, `z x (a, b) = (-b, a)`).
pub fn predictors(t: &[f64], r: &[Vec<(f64, f64)>], q: &[i32], l: f64) -> Predictors {
    let nt = t.len();
    let n = q.len();
    let mut v = vec![0.0; nt * n * 2];
    let mut bv = vec![0.0; nt * n * 2];
    let mut h = vec![0.0; nt];
    let mut s = vec![0.0; nt];
    for k in 0..nt {
        let pk: Vec<(f64, f64)> = r[k]
            .iter()
            .map(|p| (p.0.rem_euclid(l), p.1.rem_euclid(l)))
            .collect();
        let vk = pv_velocity(&pk, q, l);
        h[k] = pv_energy(&pk, q, l);
        let mut sq = Vec::with_capacity(2 * n);
        for (i, w) in vk.iter().enumerate() {
            let b = (k * n + i) * 2;
            v[b] = w.0;
            v[b + 1] = w.1;
            let nq = -(q[i] as f64);
            bv[b] = nq * (-w.1);
            bv[b + 1] = nq * w.0;
            sq.push(w.0 * w.0);
            sq.push(w.1 * w.1);
        }
        s[k] = np_sum(&sq);
    }
    let cum = |a: &[f64]| -> Vec<f64> {
        let st = n * 2;
        let mut c = vec![0.0; a.len()];
        for k in 1..nt {
            let dt = t[k] - t[k - 1];
            for e in 0..st {
                c[k * st + e] =
                    c[(k - 1) * st + e] + 0.5 * (a[k * st + e] + a[(k - 1) * st + e]) * dt;
            }
        }
        c
    };
    Predictors {
        ca: cum(&v),
        cb: cum(&bv),
        h,
        s,
    }
}

/// Parameters of [`analyse_tracks`] (`lag`, `t_settle`, `d_valid`, `lags_eta` of the Python function; defaults are the
/// registered values).
#[derive(Debug, Clone)]
pub struct Options {
    /// lag (in samples) of the two-coefficient regression
    pub lag: usize,
    /// samples with `t < t_settle` are discarded
    pub t_settle: f64,
    /// the track is cut at the first sample where any vortex-antivortex pair is closer than this
    pub d_valid: f64,
    /// lags (in samples) of the residual-MSD fit for `eta`
    pub lags_eta: Vec<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            lag: 10,
            t_settle: 100.0,
            d_valid: 4.0,
            lags_eta: vec![5, 10, 20, 40, 80, 120, 160, 240, 320, 400],
        }
    }
}

/// Names of the six estimators, in the order of the Python `names` list.
pub const NAMES: [&str; 6] = [
    "one_minus_alpha_prime",
    "alpha_regression",
    "alpha_energy",
    "eta",
    "msd_exponent",
    "msd_offset",
];

/// Result of [`analyse_tracks`]: the Python dictionary (`<name>` and `<name>_se` for the six estimators, `n_blocks`,
/// `lag`, `lags_eta`, `msd`).
#[derive(Debug, Clone)]
pub struct Estimates {
    pub one_minus_alpha_prime: f64,
    pub alpha_regression: f64,
    pub alpha_energy: f64,
    pub eta: f64,
    pub msd_exponent: f64,
    pub msd_offset: f64,
    pub one_minus_alpha_prime_se: f64,
    pub alpha_regression_se: f64,
    pub alpha_energy_se: f64,
    pub eta_se: f64,
    pub msd_exponent_se: f64,
    pub msd_offset_se: f64,
    pub n_blocks: usize,
    pub lag: usize,
    pub lags_eta: Vec<usize>,
    /// mean squared residual per vortex at each lag of `lags_eta` (NaN where no block was long enough)
    pub msd: Vec<f64>,
}

impl Estimates {
    /// The six point estimates in the order of [`NAMES`].
    pub fn values(&self) -> [f64; 6] {
        [
            self.one_minus_alpha_prime,
            self.alpha_regression,
            self.alpha_energy,
            self.eta,
            self.msd_exponent,
            self.msd_offset,
        ]
    }

    /// The six block-jackknife standard errors in the order of [`NAMES`].
    pub fn errors(&self) -> [f64; 6] {
        [
            self.one_minus_alpha_prime_se,
            self.alpha_regression_se,
            self.alpha_energy_se,
            self.eta_se,
            self.msd_exponent_se,
            self.msd_offset_se,
        ]
    }
}

/// One jackknife block (half of a validity-cut, settled track); flat arrays `[time][vortex][x, y]`.
struct Block {
    t: Vec<f64>,
    stride: usize,
    u: Vec<f64>,
    ca: Vec<f64>,
    cb: Vec<f64>,
    h: Vec<f64>,
    s: Vec<f64>,
}

impl Block {
    /// `X[lg:] - X[:-lg]` for the flat array `x`.
    fn lagdiff(&self, x: &[f64], lg: usize) -> Vec<f64> {
        let off = lg * self.stride;
        if off >= x.len() {
            return Vec::new();
        }
        (0..x.len() - off).map(|i| x[i + off] - x[i]).collect()
    }
}

/// `reg_sums`: `[sum A A, sum A B, sum B B, sum A D, sum B D, sum D D, D.size]` at lag `lg`.
fn reg_sums(b: &Block, lg: usize) -> [f64; 7] {
    let d = b.lagdiff(&b.u, lg);
    let a = b.lagdiff(&b.ca, lg);
    let bb = b.lagdiff(&b.cb, lg);
    let prod = |x: &[f64], y: &[f64]| -> f64 {
        let p: Vec<f64> = x.iter().zip(y).map(|(u, v)| u * v).collect();
        np_sum(&p)
    };
    [
        prod(&a, &a),
        prod(&a, &bb),
        prod(&bb, &bb),
        prod(&a, &d),
        prod(&bb, &d),
        prod(&d, &d),
        d.len() as f64,
    ]
}

/// Solve the symmetric 2x2 system `[[a, b], [b, c]] x = (r0, r1)` by LU with partial pivoting (LAPACK `gesv`).
/// `None` if singular.
fn solve2(a: f64, b: f64, c: f64, r0: f64, r1: f64) -> Option<[f64; 2]> {
    // rows: (a, b | r0), (b, c | r1)
    let (m11, m12, s1, m21, m22, s2) = if b.abs() > a.abs() {
        (b, c, r1, a, b, r0)
    } else {
        (a, b, r0, b, c, r1)
    };
    if m11 == 0.0 {
        return None;
    }
    let l21 = m21 / m11;
    let u22 = m22 - l21 * m12;
    if u22 == 0.0 {
        return None;
    }
    let y2 = s2 - l21 * s1;
    let x2 = y2 / u22;
    let x1 = (s1 - m12 * x2) / m11;
    Some([x1, x2])
}

/// `np.polyfit(x, y, 1)` as ordinary least squares: `(slope, intercept)`.
pub fn polyfit1(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len() as f64;
    let xm = np_sum(x) / n;
    let ym = np_sum(y) / n;
    let sxx: Vec<f64> = x.iter().map(|v| (v - xm) * (v - xm)).collect();
    let sxy: Vec<f64> = x.iter().zip(y).map(|(u, v)| (u - xm) * (v - ym)).collect();
    let slope = np_sum(&sxy) / np_sum(&sxx);
    (slope, ym - slope * xm)
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// `eta_fit`: residual MSD at each lag with the regression coefficients `c`, then the line `MSD = 4 eta dt lag + c0`
/// over the lags `>= 20`, and the exponent of `MSD - c0`. Returns `(eta, c0, exponent, ms)`.
fn eta_fit(bl: &[&Block], c: [f64; 2], lags: &[usize]) -> (f64, f64, f64, Vec<f64>) {
    let mut ms = Vec::with_capacity(lags.len());
    for &lg in lags {
        let (mut num, mut den) = (0.0, 0.0);
        for b in bl {
            if b.t.len() <= lg + 2 {
                continue;
            }
            let d = b.lagdiff(&b.u, lg);
            let a = b.lagdiff(&b.ca, lg);
            let bb = b.lagdiff(&b.cb, lg);
            let e2: Vec<f64> = (0..d.len())
                .map(|i| {
                    let e = d[i] - c[0] * a[i] - c[1] * bb[i];
                    e * e
                })
                .collect();
            num += np_sum(&e2);
            den += (d.len() / 2) as f64; // e.shape[0] * e.shape[1] = rows x vortices
        }
        ms.push(if den != 0.0 { num / den } else { f64::NAN });
    }
    let sel: Vec<usize> = (0..lags.len())
        .filter(|&i| ms[i].is_finite() && lags[i] >= 20)
        .collect();
    if sel.len() < 4 {
        return (f64::NAN, f64::NAN, f64::NAN, ms);
    }
    let dt = {
        let t = &bl[0].t;
        let diffs: Vec<f64> = t.windows(2).map(|w| w[1] - w[0]).collect();
        median(&diffs)
    };
    let x: Vec<f64> = sel.iter().map(|&i| lags[i] as f64 * dt).collect();
    let y: Vec<f64> = sel.iter().map(|&i| ms[i]).collect();
    let (slope, c0) = polyfit1(&x, &y);
    let eta = slope / 4.0;
    let pos: Vec<usize> = (0..y.len()).filter(|&i| y[i] - c0 > 0.0).collect();
    let gam = if pos.len() >= 3 {
        let lx: Vec<f64> = pos.iter().map(|&i| (lags[sel[i]] as f64).ln()).collect();
        let ly: Vec<f64> = pos.iter().map(|&i| (y[i] - c0).ln()).collect();
        polyfit1(&lx, &ly).0
    } else {
        f64::NAN
    };
    (eta, c0, gam, ms)
}

/// `estimate` of the Python: the six point estimates (and the MSD vector) from a list of blocks.
fn estimate(bl: &[&Block], opts: &Options) -> Result<([f64; 6], Vec<f64>), String> {
    let mut sm = [0.0; 7];
    for b in bl {
        let r = reg_sums(b, opts.lag);
        for (s, v) in sm.iter_mut().zip(r) {
            *s += v;
        }
    }
    let c = solve2(sm[0], sm[1], sm[2], sm[3], sm[4]).ok_or("singular regression matrix")?;
    let mut es = [0.0; 2];
    for b in bl {
        // energy_sums: I = 2 int S dt (trapezoid), centred; (I0 H0).sum(), (I0 I0).sum()
        let nt = b.t.len();
        let mut i_int = vec![0.0; nt];
        for k in 1..nt {
            i_int[k] = i_int[k - 1] + 0.5 * (b.s[k] + b.s[k - 1]) * (b.t[k] - b.t[k - 1]);
        }
        i_int.iter_mut().for_each(|v| *v *= 2.0);
        let im = np_sum(&i_int) / nt as f64;
        let hm = np_sum(&b.h) / nt as f64;
        let i0: Vec<f64> = i_int.iter().map(|v| v - im).collect();
        let h0: Vec<f64> = b.h.iter().map(|v| v - hm).collect();
        let ih: Vec<f64> = i0.iter().zip(&h0).map(|(a, b)| a * b).collect();
        let ii: Vec<f64> = i0.iter().map(|a| a * a).collect();
        es[0] += np_sum(&ih);
        es[1] += np_sum(&ii);
    }
    let a_en = -es[0] / es[1];
    let (eta, c0, gam, ms) = eta_fit(bl, c, &opts.lags_eta);
    Ok(([c[0], c[1], a_en, eta, gam, c0], ms))
}

/// `analyse_tracks`: the pre-registered estimators with block-jackknife errors (blocks = tracks x halves).
///
/// Each track is cut at the first sample where any vortex and antivortex are closer than `opts.d_valid` (below about
/// 4 healing lengths a pair is a single solitary wave), samples with `t < opts.t_settle` are dropped, tracks with fewer
/// than `3 lag + 10` samples left are skipped, the rest is unwrapped and split in two halves. `Err("no usable track")`
/// replaces the Python `{"error": ...}` dictionary.
pub fn analyse_tracks(tracks: &[Track], l: f64, opts: &Options) -> Result<Estimates, String> {
    let mut blocks: Vec<Block> = Vec::new();
    for tr in tracks {
        let n_ok = (0..tr.t.len())
            .find(|&k| {
                let pk: Vec<(f64, f64)> = tr.r[k]
                    .iter()
                    .map(|p| (p.0.rem_euclid(l), p.1.rem_euclid(l)))
                    .collect();
                min_opposite_distance(&pk, &tr.q, l) < opts.d_valid
            })
            .unwrap_or(tr.t.len());
        let keep: Vec<usize> = (0..n_ok).filter(|&k| tr.t[k] >= opts.t_settle).collect();
        if keep.len() < 3 * opts.lag + 10 {
            continue;
        }
        let t: Vec<f64> = keep.iter().map(|&k| tr.t[k]).collect();
        let rr: Vec<Vec<(f64, f64)>> = keep.iter().map(|&k| tr.r[k].clone()).collect();
        let uw = unwrap(&rr, l);
        let pr = predictors(&t, &uw, &tr.q, l);
        let n = tr.q.len();
        let stride = 2 * n;
        let half = t.len() / 2;
        for (a, b) in [(0, half), (half, t.len())] {
            let mut u = Vec::with_capacity((b - a) * stride);
            for row in &uw[a..b] {
                for p in row {
                    u.push(p.0);
                    u.push(p.1);
                }
            }
            blocks.push(Block {
                t: t[a..b].to_vec(),
                stride,
                u,
                ca: pr.ca[a * stride..b * stride].to_vec(),
                cb: pr.cb[a * stride..b * stride].to_vec(),
                h: pr.h[a..b].to_vec(),
                s: pr.s[a..b].to_vec(),
            });
        }
    }
    if blocks.is_empty() {
        return Err("no usable track".to_string());
    }
    let all: Vec<&Block> = blocks.iter().collect();
    let (full, ms) = estimate(&all, opts)?;
    let nb = blocks.len();
    let se = if nb >= 3 {
        let mut jk: Vec<[f64; 6]> = Vec::with_capacity(nb);
        for i in 0..nb {
            let sub: Vec<&Block> = (0..nb).filter(|&j| j != i).map(|j| &blocks[j]).collect();
            jk.push(estimate(&sub, opts)?.0);
        }
        let mut se = [0.0; 6];
        for (c, s) in se.iter_mut().enumerate() {
            // np.nanmean / np.nansum over axis 0 (sequential over blocks, NaN -> 0)
            let (mut sum, mut cnt) = (0.0, 0.0);
            for row in &jk {
                if !row[c].is_nan() {
                    sum += row[c];
                    cnt += 1.0;
                }
            }
            let mean = if cnt > 0.0 { sum / cnt } else { f64::NAN };
            let mut acc = 0.0;
            for row in &jk {
                let d = (row[c] - mean) * (row[c] - mean);
                if !d.is_nan() {
                    acc += d;
                }
            }
            *s = ((nb - 1) as f64 / nb as f64 * acc).sqrt();
        }
        se
    } else {
        [f64::NAN; 6]
    };
    Ok(Estimates {
        one_minus_alpha_prime: full[0],
        alpha_regression: full[1],
        alpha_energy: full[2],
        eta: full[3],
        msd_exponent: full[4],
        msd_offset: full[5],
        one_minus_alpha_prime_se: se[0],
        alpha_regression_se: se[1],
        alpha_energy_se: se[2],
        eta_se: se[3],
        msd_exponent_se: se[4],
        msd_offset_se: se[5],
        n_blocks: nb,
        lag: opts.lag,
        lags_eta: opts.lags_eta.clone(),
        msd: ms,
    })
}

/// Parse the CSV written by [`vortex::csv`] (`t,x0,y0,...,n_det,Px,Py,Px_hi,Py_hi`) into a [`Track`] of
/// `q.len()` vortices (only the first `1 + 2 q.len()` columns are read; the header line is skipped).
pub fn read_csv_track(text: &str, q: &[i32]) -> Result<Track, String> {
    let n = q.len();
    let (mut t, mut r) = (Vec::new(), Vec::new());
    for (ln, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('t') {
            continue;
        }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() < 1 + 2 * n {
            return Err(format!(
                "line {}: {} columns, need at least {}",
                ln + 1,
                f.len(),
                1 + 2 * n
            ));
        }
        let num = |s: &str| {
            s.trim()
                .parse::<f64>()
                .map_err(|e| format!("line {}: {e}", ln + 1))
        };
        t.push(num(f[0])?);
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            row.push((num(f[1 + 2 * i])?, num(f[2 + 2 * i])?));
        }
        r.push(row);
    }
    Ok(Track {
        t,
        r,
        q: q.to_vec(),
    })
}

// ---- synthetic generator (gate G0) -------------------------------------------------------------------------------------

/// Standard normal deviate from the campaign LCG (Box-Muller, cosine branch; not numpy's ziggurat).
pub fn standard_normal(rng: &mut Lcg) -> f64 {
    let u1 = 1.0 - rng.uniform(); // (0, 1]
    let u2 = rng.uniform();
    (-2.0 * u1.ln()).sqrt() * (TWO_PI * u2).cos()
}

/// `langevin`: Euler-Maruyama integration of the model with sampling every `dt_sample`; stops as soon as ANY vortex
/// and antivortex come within `d_stop` (checked every substep). Returns `(t, r)` with `r[time][vortex]`.
#[allow(clippy::too_many_arguments)]
pub fn langevin(
    pos0: &[(f64, f64)],
    q: &[i32],
    l: f64,
    alpha: f64,
    alphap: f64,
    eta: f64,
    t_end: f64,
    rng: &mut Lcg,
    dt: f64,
    dt_sample: f64,
    d_stop: f64,
) -> (Vec<f64>, Vec<Vec<(f64, f64)>>) {
    let mut r = pos0.to_vec();
    let (mut out_t, mut out_r) = (vec![0.0], vec![r.clone()]);
    let nsub = (dt_sample / dt).round() as usize;
    let mut t = 0.0;
    let noise = (2.0 * eta * dt).sqrt();
    while t < t_end - 1e-9 {
        for _ in 0..nsub {
            let pk: Vec<(f64, f64)> = r
                .iter()
                .map(|p| (p.0.rem_euclid(l), p.1.rem_euclid(l)))
                .collect();
            let v = pv_velocity(&pk, q, l);
            for (i, p) in r.iter_mut().enumerate() {
                let aq = alpha * q[i] as f64;
                let (vx, vy) = v[i];
                p.0 += ((1.0 - alphap) * vx - aq * (-vy)) * dt + noise * standard_normal(rng);
                p.1 += ((1.0 - alphap) * vy - aq * vx) * dt + noise * standard_normal(rng);
            }
            if min_opposite_distance(&r, q, l) < d_stop {
                return (out_t, out_r);
            }
        }
        t += dt_sample;
        out_t.push(t);
        out_r.push(r.clone());
    }
    (out_t, out_r)
}

/// Result of [`gate_g0`] (`gate_G0` of the Python, without the JSON file).
#[derive(Debug, Clone)]
pub struct G0Report {
    /// `(alpha, alpha', eta)` of the generator
    pub truth: (f64, f64, f64),
    pub estimates: Estimates,
    /// `alpha_energy_within_15pct`, `one_minus_alphap_within_0.02`, `eta_within_25pct`
    pub checks: [bool; 3],
    pub pass: bool,
    pub track_lengths: Vec<f64>,
}

/// `gate_G0`: 8 synthetic tracks (two antiparallel dipoles, `d0 = 10`, `L = 64`, 2000 time units, detection noise
/// 0.2 per coordinate) with known `alpha = 0.02`, `alpha' = 0.10`, `eta = 2e-3`, analysed with the default
/// [`Options`]. The random stream is the campaign LCG, not numpy's PCG64, so only the pass criteria are comparable
/// with the Python run.
pub fn gate_g0(seed: u64) -> G0Report {
    let l = 64.0;
    let (alpha, alphap, eta) = (0.02, 0.10, 2e-3);
    let mut rng = Lcg::new(seed);
    let mut tracks = Vec::new();
    for _ in 0..8 {
        let (pos, q) = vortex::antiparallel(l, 10.0, &mut rng);
        let (t, mut r) = langevin(
            &pos, &q, l, alpha, alphap, eta, 2000.0, &mut rng, 0.05, 1.0, 2.0,
        );
        for row in r.iter_mut() {
            for p in row.iter_mut() {
                p.0 += 0.2 * standard_normal(&mut rng);
                p.1 += 0.2 * standard_normal(&mut rng);
            }
        }
        tracks.push(Track { t, r, q });
    }
    let est = analyse_tracks(&tracks, l, &Options::default()).expect("G0 tracks are usable");
    let checks = [
        (est.alpha_energy / alpha - 1.0).abs() <= 0.15,
        (est.one_minus_alpha_prime - (1.0 - alphap)).abs() <= 0.02,
        (est.eta / eta - 1.0).abs() <= 0.25,
    ];
    G0Report {
        truth: (alpha, alphap, eta),
        pass: checks.iter().all(|&c| c),
        checks,
        track_lengths: tracks.iter().map(|t| *t.t.last().unwrap()).collect(),
        estimates: est,
    }
}
