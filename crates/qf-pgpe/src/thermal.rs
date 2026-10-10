//! Thermal-state toolkit of SocrateAI-Scientific-QuantumFluids (`exploration/pgpe/observables.py`, the thermal
//! part of `round2.py`, `pgpe.PGPE.random_state`, and `make_friction_bases.py`), ported to the pure-Rust
//! [`ComplexField2D`]:
//!
//! * [`Rng`] -- a small seeded generator (xoshiro256** + Box-Muller). **It is not numpy's generator**: a field
//!   built with it ([`random_state`], [`heat`]) is a different realisation of the same ensemble than numpy's, so
//!   cross-checks of *constructions* are statistical, while *measurements on a given field*
//!   ([`thermometer`], [`condensate_fraction`], [`current_correlators`], [`n_vortices`], [`resample`]) and the
//!   dynamics are exact (rounding level, tested to `1e-9` against Python in `tests/thermal_crosscheck.rs`).
//! * [`random_state`] -- random-phase Gaussian spectrum `|c_k| ~ exp(-k^2/2 s^2)`, norm `n0 L^2`, energy per
//!   particle `e_target` by bisection on `s` (`PGPE.random_state`).
//! * [`heat`] -- add `eps P[eta]`, restore the norm, bisect `eps` so that `E = E_target` (`round2.heat`).
//! * [`thermometer`] -- classical-equipartition least-squares fit `1/<n_k> = (eps_k + 2gn - mu)/T`.
//! * [`run_blocks`] -- `round2.run_blocks`: equilibrate, sample every `every_t`, aggregate per block and over the
//!   whole window. **Not ported** (declared): the `g1(r)` correlation and its fit (`g1_radial`, `fit_g1`,
//!   `fit_g1_window`), the vortex-dipole matching `Q` (`dipole_matching`, needs an assignment solver), pairing
//!   and the Onsager dipole; the summary carries `T`, the low-window `T`, condensate fraction, raw vortex count,
//!   `J_L`, `J_T`, `n_s/n = 1 - J_T/J_L`, `R_L`, `R_T`.
//! * [`make_base`] -- heat + equilibrate + measure, as `make_friction_bases.py` / `make_transport_bases.py`.
//!
//! # Statistical cross-check (end to end), recorded numbers
//!
//! `N = 64, L = 32, e = 0.60, dt = 0.01`: `random_state` -> 200 time units of equilibration -> 300 of measurement
//! (`T` of `round2.run_blocks` on `[0.6, 1.0] k_cut`). Seeds 1..=9 on each side (different generators, so the
//! seeds pair nothing); `#[ignore]`d test `statistical_base_matches_python`
//! (`cargo test --release -p qf-pgpe --test thermal_crosscheck -- --ignored --nocapture`).
//!
//! ```text
//!                 T (mean, per-seed sd)   n_s/n    raw n_v   condensate
//! Python, 3 seeds     0.0855                0.954    0.13      0.863
//! Rust,   3 seeds     0.0765                0.915    0.42      0.833     (T differs by 10.5 %)
//! Python, 9 seeds     0.0865 (0.0101)       0.963    0.12      0.867
//! Rust,   9 seeds     0.0757 (0.0176)       0.911    0.50      0.827     (T differs by 12.5 %, z = 1.6)
//! ```
//!
//! The nominal "within 10 %" is not met by this sample (10.5 % at 3 seeds, 12.5 % at 9) and is not a usable
//! criterion: the per-seed scatter of `T` is 12-23 % (the raw vortex count is heavy-tailed: 0 to 1.5 per snapshot),
//! so the standard error of a 9-v-9 comparison is about 8 %. The test therefore asserts `|z| < 2.5` instead.
//! To separate noise from a systematic difference, 60 seeds each of a cheaper case (`N = 32, L = 16, e = 0.8`,
//! `random_state` then 100 time units, one snapshot) were compared: raw vortex count 0.67 (Rust) vs 0.57 (Python),
//! condensate fraction 0.739 +- 0.009 vs 0.727 +- 0.014, median single-snapshot `T` 0.037 vs 0.040 -- not
//! distinguishable. (The N = 64 vortex-count excess in the Rust sample, 0.50 vs 0.12, is within that noise.)

use crate::ComplexField2D;
use crate::vortex::detect;
use num_complex::Complex64;
use rustfft::FftPlanner;
use std::f64::consts::PI;
use std::io::{Read, Write};
use std::path::Path;

// ---------------------------------------------------------------------------------------------------------
// RNG
// ---------------------------------------------------------------------------------------------------------

/// xoshiro256** (Blackman-Vigna) seeded through splitmix64, with Box-Muller standard normals.
/// Deterministic per seed, **not numpy-identical**.
#[derive(Clone, Debug)]
pub struct Rng {
    s: [u64; 4],
    spare: Option<f64>,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            x ^ (x >> 31)
        };
        Self {
            s: [next(), next(), next(), next()],
            spare: None,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    /// Uniform in `[0, 1)` with 53 random bits.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Standard normal (Box-Muller, both outputs used).
    pub fn normal(&mut self) -> f64 {
        if let Some(z) = self.spare.take() {
            return z;
        }
        let u1 = 1.0 - self.uniform(); // (0, 1]
        let u2 = self.uniform();
        let r = (-2.0 * u1.ln()).sqrt();
        let (s, c) = (2.0 * PI * u2).sin_cos();
        self.spare = Some(r * s);
        r * c
    }
}

// ---------------------------------------------------------------------------------------------------------
// FFT and raw I/O helpers
// ---------------------------------------------------------------------------------------------------------

/// Unprojected `n x n` 2D FFT of a row-major buffer, numpy conventions (`inverse` divides by `n^2`).
fn fft2(data: &mut [Complex64], n: usize, inverse: bool) {
    let mut planner = FftPlanner::new();
    let fft = if inverse {
        planner.plan_fft_inverse(n)
    } else {
        planner.plan_fft_forward(n)
    };
    for row in data.chunks_mut(n) {
        fft.process(row);
    }
    let mut col = vec![Complex64::new(0.0, 0.0); n];
    for j in 0..n {
        for i in 0..n {
            col[i] = data[i * n + j];
        }
        fft.process(&mut col);
        for i in 0..n {
            data[i * n + j] = col[i];
        }
    }
    if inverse {
        let s = 1.0 / (n * n) as f64;
        data.iter_mut().for_each(|v| *v *= s);
    }
}

/// Read `n2` little-endian complex128 values (`numpy.complex128.tofile` / `vortex_transport` layout).
pub fn read_raw(path: &Path, n2: usize) -> std::io::Result<Vec<Complex64>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    if buf.len() != 16 * n2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("raw field has {} bytes, expected {}", buf.len(), 16 * n2),
        ));
    }
    Ok((0..n2)
        .map(|k| {
            let re = f64::from_le_bytes(buf[16 * k..16 * k + 8].try_into().unwrap());
            let im = f64::from_le_bytes(buf[16 * k + 8..16 * k + 16].try_into().unwrap());
            Complex64::new(re, im)
        })
        .collect())
}

/// Write complex128 little endian (the layout [`read_raw`] and `vortex_transport` read).
pub fn write_raw(path: &Path, c: &[Complex64]) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(16 * c.len());
    for v in c {
        buf.extend_from_slice(&v.re.to_le_bytes());
        buf.extend_from_slice(&v.im.to_le_bytes());
    }
    std::fs::File::create(path)?.write_all(&buf)
}

// ---------------------------------------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------------------------------------

/// `PGPE.random_state`: random-phase spectrum `exp(-k^2/(2 s^2))` on the projector, norm `n0 L^2`, energy per
/// particle `e_target` (bisection on `s` in `[0.02, k_cut]`, `iters` halvings).
pub fn random_state(
    f: &ComplexField2D,
    n0: f64,
    e_target: f64,
    rng: &mut Rng,
    iters: usize,
) -> Vec<Complex64> {
    let phase: Vec<Complex64> = (0..f.n * f.n)
        .map(|_| Complex64::from_polar(1.0, 2.0 * PI * rng.uniform()))
        .collect();
    let ntot = n0 * f.l * f.l;
    let make = |s: f64| -> Vec<Complex64> {
        let mut c: Vec<Complex64> = (0..phase.len())
            .map(|i| {
                if f.mask[i] {
                    phase[i] * (-f.k2[i] / (2.0 * s * s)).exp()
                } else {
                    Complex64::new(0.0, 0.0)
                }
            })
            .collect();
        let scale = (ntot / f.norm(&c)).sqrt();
        c.iter_mut().for_each(|v| *v *= scale);
        c
    };
    let (mut lo, mut hi) = (0.02, f.kcut);
    for _ in 0..iters {
        let mid = 0.5 * (lo + hi);
        if f.energy(&make(mid)) / ntot < e_target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    make(0.5 * (lo + hi))
}

/// `round2.heat`: `c + eps*eta` with `eta` complex Gaussian noise on `band * k_cut` (inside the projector),
/// renormalised to the norm of `c`, `eps` bisected (`iters` halvings) so that `E = e_target_total`.
/// Errors if the target is below the energy of `c`.
pub fn heat(
    f: &ComplexField2D,
    c: &[Complex64],
    e_target_total: f64,
    rng: &mut Rng,
    band: (f64, f64),
    iters: usize,
) -> Result<Vec<Complex64>, String> {
    let m = c.len();
    let in_band: Vec<bool> = (0..m)
        .map(|i| {
            let kk = f.k2[i].sqrt();
            kk >= band.0 * f.kcut && kk <= band.1 * f.kcut && f.mask[i]
        })
        .collect();
    let re: Vec<f64> = (0..m).map(|_| rng.normal()).collect();
    let im: Vec<f64> = (0..m).map(|_| rng.normal()).collect();
    let mut eta: Vec<Complex64> = (0..m)
        .map(|i| {
            if in_band[i] {
                Complex64::new(re[i], im[i])
            } else {
                Complex64::new(0.0, 0.0)
            }
        })
        .collect();
    let n_c = f.norm(c);
    let scale = (n_c / f.norm(&eta)).sqrt();
    eta.iter_mut().for_each(|v| *v *= scale);
    let make = |eps: f64| -> Vec<Complex64> {
        let mut c2: Vec<Complex64> = c.iter().zip(&eta).map(|(a, b)| a + b * eps).collect();
        let s = (n_c / f.norm(&c2)).sqrt();
        c2.iter_mut().for_each(|v| *v *= s);
        c2
    };
    if f.energy(&make(0.0)) > e_target_total {
        return Err("target energy below the base energy".into());
    }
    let (mut lo, mut hi) = (0.0, 0.05);
    let mut guard = 0;
    while f.energy(&make(hi)) < e_target_total {
        hi *= 2.0;
        guard += 1;
        if guard > 200 {
            return Err("heat: target energy unreachable".into());
        }
    }
    for _ in 0..iters {
        let mid = 0.5 * (lo + hi);
        if f.energy(&make(mid)) < e_target_total {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(make(0.5 * (lo + hi)))
}

/// Fourier zero-padding / truncation of a periodic `n x n` field to `n_new x n_new` on the same box
/// (`make_friction_bases.resample`; both sizes even). Preserves each signed frequency; the factor
/// `(n_new/n)^2` keeps the pointwise values (`psi` is a field, not a sum).
pub fn resample(psi: &[Complex64], n_new: usize) -> Vec<Complex64> {
    let n = (psi.len() as f64).sqrt().round() as usize;
    assert_eq!(n * n, psi.len(), "resample: field is not square");
    if n == n_new {
        return psi.to_vec();
    }
    assert!(
        n.is_multiple_of(2) && n_new.is_multiple_of(2),
        "resample: even sizes required"
    );
    let mut fk = psi.to_vec();
    fft2(&mut fk, n, false);
    let mut g = vec![Complex64::new(0.0, 0.0); n_new * n_new];
    let half = (n.min(n_new) / 2) as i64;
    for m1 in -half..half {
        for m2 in -half..half {
            let a = (m1.rem_euclid(n as i64) as usize) * n + m2.rem_euclid(n as i64) as usize;
            let b = (m1.rem_euclid(n_new as i64) as usize) * n_new
                + m2.rem_euclid(n_new as i64) as usize;
            g[b] = fk[a];
        }
    }
    fft2(&mut g, n_new, true);
    let s = (n_new as f64 / n as f64).powi(2);
    g.iter_mut().for_each(|v| *v *= s);
    g
}

// ---------------------------------------------------------------------------------------------------------
// Observables
// ---------------------------------------------------------------------------------------------------------

/// Equipartition fit `1/<n_k> = (eps_k + 2gn - mu)/T` on `k in [lo, hi] * k_cut` (inside the projector, `occ > 0`);
/// `occ` is the occupation `|c|^2 dx^2/N^2` (or its time average). Returns `(T, 2gn - mu)`. Least squares in
/// centred form (same minimiser as numpy's `lstsq`, better conditioned).
pub fn thermometer(f: &ComplexField2D, occ: &[f64], lo: f64, hi: f64) -> (f64, f64) {
    let (mut sx, mut sy, mut cnt) = (0.0, 0.0, 0usize);
    let sel: Vec<(f64, f64)> = (0..occ.len())
        .filter(|&i| {
            let kk = f.k2[i].sqrt();
            kk >= lo * f.kcut && kk <= hi * f.kcut && f.mask[i] && occ[i] > 0.0
        })
        .map(|i| (0.5 * f.k2[i], 1.0 / occ[i]))
        .collect();
    for &(x, y) in &sel {
        sx += x;
        sy += y;
        cnt += 1;
    }
    if cnt < 2 {
        return (f64::NAN, f64::NAN);
    }
    let (mx, my) = (sx / cnt as f64, sy / cnt as f64);
    let (mut sxx, mut sxy) = (0.0, 0.0);
    for &(x, y) in &sel {
        sxx += (x - mx) * (x - mx);
        sxy += (x - mx) * (y - my);
    }
    let slope = sxy / sxx;
    let icpt = my - slope * mx;
    (1.0 / slope, icpt / slope)
}

/// Condensate fraction `|c_0|^2 dx^2/N^2 / norm`.
pub fn condensate_fraction(f: &ComplexField2D, c: &[Complex64]) -> f64 {
    c[0].norm_sqr() * f.dx * f.dx / (f.n * f.n) as f64 / f.norm(c)
}

/// Raw phase-winding plaquette count (`observables.vortices`; no pairing, no refinement).
pub fn n_vortices(f: &ComplexField2D, c: &[Complex64]) -> usize {
    detect(f, c).len()
}

/// The shells (units of `2 pi / L`) of `observables.current_correlators`.
pub const SHELLS: [f64; 3] = [1.0, std::f64::consts::SQRT_2, 2.0];

/// `<|J_L(k)|^2>, <|J_T(k)|^2>` on one shell.
#[derive(Clone, Copy, Debug)]
pub struct ShellCorr {
    pub shell: f64,
    pub jl: f64,
    pub jt: f64,
}

/// `observables.current_correlators`: `J = Im(psi* grad psi)`, `J_k = fft2(J) dx^2`, averaged over the modes
/// with `| |k| - sh*2pi/L | < 1e-9` for `sh` in [`SHELLS`]; longitudinal/transverse projections on `k_hat`.
pub fn current_correlators(f: &ComplexField2D, c: &[Complex64]) -> [ShellCorr; 3] {
    let m = c.len();
    let psi = f.psi(c);
    let i_kx: Vec<Complex64> = (0..m)
        .map(|i| Complex64::new(0.0, f.kx[i]) * c[i])
        .collect();
    let i_ky: Vec<Complex64> = (0..m)
        .map(|i| Complex64::new(0.0, f.ky[i]) * c[i])
        .collect();
    let gx = f.psi(&i_kx);
    let gy = f.psi(&i_ky);
    let mut jkx: Vec<Complex64> = (0..m)
        .map(|i| Complex64::new((psi[i].conj() * gx[i]).im, 0.0))
        .collect();
    let mut jky: Vec<Complex64> = (0..m)
        .map(|i| Complex64::new((psi[i].conj() * gy[i]).im, 0.0))
        .collect();
    fft2(&mut jkx, f.n, false);
    fft2(&mut jky, f.n, false);
    let dx2 = f.dx * f.dx;
    let dk = 2.0 * PI / f.l;
    let mut out = [ShellCorr {
        shell: 0.0,
        jl: f64::NAN,
        jt: f64::NAN,
    }; 3];
    for (o, &sh) in out.iter_mut().zip(&SHELLS) {
        let (mut sl, mut st, mut cnt) = (0.0, 0.0, 0usize);
        for i in 0..m {
            let kk = f.k2[i].sqrt();
            if (kk - sh * dk).abs() < 1e-9 {
                let (kxh, kyh) = (f.kx[i] / kk, f.ky[i] / kk);
                let (ax, ay) = (jkx[i] * dx2, jky[i] * dx2);
                sl += (ax * kxh + ay * kyh).norm_sqr();
                st += (ay * kxh - ax * kyh).norm_sqr();
                cnt += 1;
            }
        }
        *o = ShellCorr {
            shell: sh,
            jl: sl / cnt as f64,
            jt: st / cnt as f64,
        };
    }
    out
}

// ---------------------------------------------------------------------------------------------------------
// Block-averaged trajectory
// ---------------------------------------------------------------------------------------------------------

/// One block of `round2.run_blocks` (without `g1`/`Q`).
#[derive(Clone, Debug)]
pub struct BlockSummary {
    pub t0: f64,
    pub t_thermo: f64,
    pub cond: f64,
    pub n_v: f64,
    pub jl: f64,
    pub jt: f64,
    pub ns_over_n: f64,
}

/// Whole-window summary (`round2.run_blocks`' `whole`, minus the declared-unported `g1`/`Q` entries).
#[derive(Clone, Debug)]
pub struct ThermoSummary {
    /// Thermometer on `[0.6, 1.0] k_cut` of the time-averaged occupation.
    pub t_thermo: f64,
    /// `2gn - mu` of that fit.
    pub offset_thermo: f64,
    /// Thermometer on `[0.4, 0.6] k_cut`.
    pub t_lowwindow: f64,
    pub cond: f64,
    pub n_v: f64,
    pub jl: f64,
    pub jt: f64,
    pub ns_over_n: f64,
    pub r_l: f64,
    pub r_t: f64,
    pub n_samples: usize,
    pub blocks: Vec<BlockSummary>,
}

/// `round2.run_blocks`: evolve `t_tr` (no sampling), then to `t_end`, sampling every `every_t` and aggregating
/// per `block` and over the whole window. Returns the final field and the summary.
pub fn run_blocks(
    f: &ComplexField2D,
    c: &[Complex64],
    t_end: f64,
    t_tr: f64,
    block: f64,
    every_t: f64,
) -> (Vec<Complex64>, ThermoSummary) {
    let mut c = if t_tr > 0.0 {
        f.run(c, t_tr)
    } else {
        c.to_vec()
    };
    let nb = (((t_end - t_tr) / block).round() as usize).max(1);
    let per = ((block / every_t).round() as usize).max(1);
    let every = ((every_t / f.dt).round() as usize).max(1);
    let nsteps = ((t_end - t_tr) / f.dt).round() as usize;
    let m = f.n * f.n;
    let w = f.dx * f.dx / m as f64;
    #[derive(Clone)]
    struct Acc {
        occ: Vec<f64>,
        cond: Vec<f64>,
        nv: Vec<f64>,
        jl: Vec<f64>,
        jt: Vec<f64>,
    }
    let mut blocks = vec![
        Acc {
            occ: vec![0.0; m],
            cond: vec![],
            nv: vec![],
            jl: vec![],
            jt: vec![],
        };
        nb
    ];
    let mut k = 0usize;
    for i in 1..=nsteps {
        c = f.step(&c);
        if i % every == 0 {
            let b = &mut blocks[(k / per).min(nb - 1)];
            k += 1;
            for (o, v) in b.occ.iter_mut().zip(&c) {
                *o += v.norm_sqr() * w;
            }
            b.cond.push(condensate_fraction(f, &c));
            b.nv.push(n_vortices(f, &c) as f64);
            let cr = current_correlators(f, &c);
            b.jl.push(cr.iter().map(|s| s.jl).sum::<f64>() / 3.0);
            b.jt.push(cr.iter().map(|s| s.jt).sum::<f64>() / 3.0);
        }
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let out: Vec<BlockSummary> = blocks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let n = b.cond.len() as f64;
            let occ: Vec<f64> = b.occ.iter().map(|x| x / n).collect();
            let (jl, jt) = (mean(&b.jl), mean(&b.jt));
            BlockSummary {
                t0: t_tr + i as f64 * block,
                t_thermo: thermometer(f, &occ, 0.6, 1.0).0,
                cond: mean(&b.cond),
                n_v: mean(&b.nv),
                jl,
                jt,
                ns_over_n: 1.0 - jt / jl,
            }
        })
        .collect();
    let ntot: usize = blocks.iter().map(|b| b.cond.len()).sum();
    let mut occ = vec![0.0; m];
    for b in &blocks {
        for (o, x) in occ.iter_mut().zip(&b.occ) {
            *o += x;
        }
    }
    occ.iter_mut().for_each(|x| *x /= ntot as f64);
    let all = |sel: fn(&Acc) -> &Vec<f64>| -> Vec<f64> {
        blocks.iter().flat_map(|b| sel(b).iter().copied()).collect()
    };
    let (t, off) = thermometer(f, &occ, 0.6, 1.0);
    let t_lo = thermometer(f, &occ, 0.4, 0.6).0;
    let (jl, jt) = (mean(&all(|b| &b.jl)), mean(&all(|b| &b.jt)));
    let summary = ThermoSummary {
        t_thermo: t,
        offset_thermo: off,
        t_lowwindow: t_lo,
        cond: mean(&all(|b| &b.cond)),
        n_v: mean(&all(|b| &b.nv)),
        jl,
        jt,
        ns_over_n: 1.0 - jt / jl,
        r_l: jl / (t * f.l * f.l),
        r_t: jt / (t * f.l * f.l),
        n_samples: ntot,
        blocks: out,
    };
    (c, summary)
}

/// A thermal base state with its provenance.
#[derive(Clone, Debug)]
pub struct Base {
    pub c: Vec<Complex64>,
    /// Energy per particle of the input field, before heating.
    pub e_start: f64,
    /// Energy per particle right after heating.
    pub e_per_particle: f64,
    /// `|E_end - E_heated| / E_heated` over the equilibration + measurement.
    pub drift_e: f64,
    pub summary: ThermoSummary,
}

/// `make_friction_bases.py` / `make_transport_bases.py`: heat `c0` to `e_target` (energy per particle) with the
/// phonon construction ([`heat`], band `[0.2, 1.0]`, 80 halvings, an [`Rng`] seeded by `seed`), evolve `t_tr`
/// (new modes thermalise) and measure over `t_end - t_tr` ([`run_blocks`], blocks of 100, samples every 10).
pub fn make_base(
    f: &ComplexField2D,
    c0: &[Complex64],
    e_target: f64,
    t_tr: f64,
    t_end: f64,
    seed: u64,
) -> Result<Base, String> {
    let mut rng = Rng::new(seed);
    let n0 = f.norm(c0);
    let e_start = f.energy(c0) / n0;
    let c = heat(f, c0, e_target * n0, &mut rng, (0.2, 1.0), 80)?;
    let e0 = f.energy(&c);
    let (c, summary) = run_blocks(f, &c, t_end, t_tr, 100.0, 10.0);
    let drift_e = (f.energy(&c) - e0).abs() / e0;
    Ok(Base {
        c,
        e_start,
        e_per_particle: e0 / n0,
        drift_e,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic_and_normal() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!(a.next_u64(), b.next_u64());
        let n = 200_000;
        let xs: Vec<f64> = (0..n).map(|_| a.normal()).collect();
        let m = xs.iter().sum::<f64>() / n as f64;
        let v = xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64;
        assert!(m.abs() < 0.01 && (v - 1.0).abs() < 0.02, "m={m} v={v}");
    }

    #[test]
    fn random_state_hits_norm_and_energy() {
        let f = ComplexField2D::new(32, 16.0, 1.0, 0.01);
        let mut rng = Rng::new(3);
        let c = random_state(&f, 1.0, 1.2, &mut rng, 60);
        assert!((f.norm(&c) / 256.0 - 1.0).abs() < 1e-12);
        assert!((f.energy(&c) / 256.0 - 1.2).abs() < 1e-6);
    }

    #[test]
    fn heat_hits_energy_and_conserves_norm() {
        let f = ComplexField2D::new(32, 16.0, 1.0, 0.01);
        let c0 = crate::vortex::uniform_condensate(&f);
        let mut rng = Rng::new(5);
        let n0 = f.norm(&c0);
        let c = heat(&f, &c0, 0.7 * n0, &mut rng, (0.2, 1.0), 80).unwrap();
        assert!((f.norm(&c) / n0 - 1.0).abs() < 1e-12);
        assert!((f.energy(&c) / n0 - 0.7).abs() < 1e-9);
        assert!(heat(&f, &c0, 0.1 * n0, &mut rng, (0.2, 1.0), 80).is_err());
    }

    #[test]
    fn resample_roundtrip_is_identity() {
        let f = ComplexField2D::new(32, 16.0, 1.0, 0.01);
        let mut rng = Rng::new(9);
        let c = random_state(&f, 1.0, 1.0, &mut rng, 60);
        let psi = f.psi(&c);
        let up = resample(&psi, 64);
        let back = resample(&up, 32);
        let err = psi
            .iter()
            .zip(&back)
            .map(|(a, b)| (a - b).norm())
            .fold(0.0, f64::max);
        assert!(err < 1e-12, "{err}");
    }
}
