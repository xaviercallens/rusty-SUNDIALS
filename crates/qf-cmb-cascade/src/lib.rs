//! Independent reproduction, in pure Rust, of a published CMB bound on a late-time first-order
//! dark-energy phase transition — Koren, Tsai, Wang, "Boiling After the Dust Settles" (hep-ph,
//! arXiv:2509.07076), checked against the real Planck 2018 TT power spectrum, using the EXACT
//! bubble-completion-time power spectrum from Elor, Jinno, Kumar, McGehee, Tsai, "Finite Bubble
//! Statistics Constrain Late Cosmological Phase Transitions" (Phys. Rev. Lett. 133, 211003 (2024),
//! arXiv:2311.16222), Supplementary Material Eqs. (S1)-(S16).
//!
//! This is a line-for-line port of `SocrateAI-Scientific-QuantumFluids`'s
//! `exploration/cmb/{pdt_exact,cmb_cascade,cmb_bound,fisher_forecast}.py` (see that repository's
//! `docs/designs/CMB_CASCADE_REPRODUCTION.md` and `docs/designs/CMB_FISHER_FORECAST.md` for the
//! full physics derivation, every approximation made, and an honest account of what does and does
//! not match the original papers). The port exists because the Python implementation's own design
//! doc states plainly that a full Fisher-forecast grid did not finish in one session: "Pdt_hatk_exact
//! is called on the order of 10^5-10^6 times per r_bound_2sigma evaluation... minutes per benchmark
//! point rather than seconds... a future session should first vectorize [it]" — this crate is that
//! vectorization, done by moving the hot loop to a compiled language rather than by restructuring
//! the NumPy code.
//!
//! Only the EXACT bubble-time spectrum is ported (not the Python's original, cruder, interpolated
//! two-asymptote approximation, which the exact spectrum superseded even in the Python source): the
//! interpolation was explicitly a stepping stone toward the exact calculation, not a result worth
//! preserving in its own right, so porting only the final, best version is a deliberate scope
//! reduction, not an oversight.
//!
//! What this crate does NOT establish (same as the Python source): no claim that the discrete-
//! cascade dark-energy model is preferred, disfavoured, or newly constrained; no mapping from any
//! quantum-fluid simulation to `(r, beta/H*)`; no claim of exact bit-for-bit agreement with the
//! original papers' own Fig. 2/Fig. 3 (agreement is to a few tens of percent at best-resolved
//! transition rates, worse at the lowest ones, for reasons stated in the design doc).

use std::sync::OnceLock;

// ============================================================================================
// Gauss-Legendre quadrature (Newton's method on Legendre-polynomial roots — the standard
// "gauleg" construction; no external crate needed for smooth, non-singular integrands like these).
// ============================================================================================

/// Nodes and weights for `n`-point Gauss-Legendre quadrature on `[-1, 1]`.
pub fn gauss_legendre_unit(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut x = vec![0.0_f64; n];
    let mut w = vec![0.0_f64; n];
    let m = n.div_ceil(2);
    let nf = n as f64;
    for i in 0..m {
        let mut z = (std::f64::consts::PI * (i as f64 + 0.75) / (nf + 0.5)).cos();
        let mut pp;
        loop {
            let mut p1 = 1.0_f64;
            let mut p2 = 0.0_f64;
            for j in 0..n {
                let p3 = p2;
                p2 = p1;
                let jf = j as f64;
                p1 = ((2.0 * jf + 1.0) * z * p2 - jf * p3) / (jf + 1.0);
            }
            pp = nf * (z * p1 - p2) / (z * z - 1.0);
            let z1 = z;
            z -= p1 / pp;
            if (z - z1).abs() <= 1e-14 {
                break;
            }
        }
        x[i] = -z;
        x[n - 1 - i] = z;
        w[i] = 2.0 / ((1.0 - z * z) * pp * pp);
        w[n - 1 - i] = w[i];
    }
    (x, w)
}

/// `n`-point Gauss-Legendre quadrature of `f` on `[a, b]`.
pub fn quad_gl(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> f64 {
    let (nodes, weights) = gauss_legendre_cached(n);
    let half = 0.5 * (b - a);
    let mid = 0.5 * (b + a);
    let mut sum = 0.0;
    for (&xi, &wi) in nodes.iter().zip(weights.iter()) {
        sum += wi * f(mid + half * xi);
    }
    half * sum
}

fn gauss_legendre_cached(n: usize) -> &'static (Vec<f64>, Vec<f64>) {
    static TABLE_100: OnceLock<(Vec<f64>, Vec<f64>)> = OnceLock::new();
    static TABLE_16: OnceLock<(Vec<f64>, Vec<f64>)> = OnceLock::new();
    static TABLE_64: OnceLock<(Vec<f64>, Vec<f64>)> = OnceLock::new();
    match n {
        16 => TABLE_16.get_or_init(|| gauss_legendre_unit(16)),
        64 => TABLE_64.get_or_init(|| gauss_legendre_unit(64)),
        100 => TABLE_100.get_or_init(|| gauss_legendre_unit(100)),
        _ => panic!("gauss_legendre_cached: unsupported order {n} (add a OnceLock arm if needed)"),
    }
}

fn trapz(x: &[f64], y: &[f64]) -> f64 {
    let mut s = 0.0;
    for i in 0..x.len() - 1 {
        s += 0.5 * (y[i] + y[i + 1]) * (x[i + 1] - x[i]);
    }
    s
}

// ============================================================================================
// The exact bubble-time correlator and power spectrum (Elor et al. arXiv:2311.16222, Eqs. S1-S16).
// ============================================================================================

fn i_func(t_xy: f64, r: f64) -> f64 {
    8.0 * std::f64::consts::PI
        * ((t_xy / 2.0).exp()
            + (-t_xy / 2.0).exp()
            + (t_xy * t_xy - (r * r + 4.0 * r)) / (4.0 * r) * (-r / 2.0).exp())
}

/// Eq. (S11)'s integrand: the single-bubble contribution to `beta^2 <delta_tc delta_tc>(r)`.
fn single_integrand(t_xy: f64, r: f64) -> f64 {
    let ival = i_func(t_xy, r);
    let term1 = 2.0 * std::f64::consts::PI * (-r / 2.0).exp() / (r * ival);
    let term2 = r * r / 4.0 + r + 2.0 - t_xy * t_xy / 4.0;
    let term3 = (ival / (8.0 * std::f64::consts::PI)).ln().powi(2) - t_xy * t_xy / 4.0
        + std::f64::consts::PI.powi(2) / 6.0;
    term1 * term2 * term3
}

/// Eq. (S16)'s integrand: the double-bubble contribution.
fn double_integrand(t_xy: f64, r: f64) -> f64 {
    let ival = i_func(t_xy, r);
    let term_b = ((-t_xy / 2.0 - r / 2.0).exp() / (2.0 * r)) * (r + t_xy + 4.0) * (r - t_xy);
    let term_c = ((t_xy / 2.0 - r / 2.0).exp() / (2.0 * r)) * (r - t_xy + 4.0) * (r + t_xy);
    let term_d =
        ((-r).exp() / (16.0 * r * r)) * ((r + 4.0).powi(2) - t_xy * t_xy) * (r * r - t_xy * t_xy);
    let bracket1 = 4.0 - term_b - term_c + term_d;
    let bracket2 = ((ival / (8.0 * std::f64::consts::PI)).ln() - 1.0).powi(2) - t_xy * t_xy / 4.0
        + std::f64::consts::PI.powi(2) / 6.0
        - 1.0;
    (16.0 * std::f64::consts::PI.powi(2) / (ival * ival)) * bracket1 * bracket2
}

/// `beta^2 <delta_tc(x) delta_tc(y)>(r)`, Eq. (S3) = Eq. (S11) + Eq. (S16).
pub fn correlator(r: f64) -> f64 {
    let r = r.max(1e-4);
    let s = quad_gl(|t| single_integrand(t, r), -r, r, 100);
    let d = quad_gl(|t| double_integrand(t, r), -r, r, 100);
    s + d
}

const R_MAX: f64 = 45.0;
const N_R: usize = 250;

fn corr_table() -> &'static (Vec<f64>, Vec<f64>) {
    static TABLE: OnceLock<(Vec<f64>, Vec<f64>)> = OnceLock::new();
    TABLE.get_or_init(|| {
        let r_grid: Vec<f64> = (0..N_R)
            .map(|i| 1e-4 + (R_MAX - 1e-4) * i as f64 / (N_R as f64 - 1.0))
            .collect();
        let corr_grid: Vec<f64> = r_grid.iter().map(|&r| correlator(r)).collect();
        (r_grid, corr_grid)
    })
}

/// Eq. (S2): the Fourier transform of the correlator, via a Fourier sine transform over the
/// precomputed correlator table (fast: no nested quadrature at call time).
pub fn p_beta_dtc_exact(k_elor: f64) -> f64 {
    let k_elor = k_elor.max(1e-8);
    let (r_grid, corr_grid) = corr_table();
    let integrand: Vec<f64> = r_grid
        .iter()
        .zip(corr_grid.iter())
        .map(|(&r, &c)| {
            let kr = k_elor * r;
            let sinc = if kr < 1e-8 { 1.0 } else { kr.sin() / kr };
            4.0 * std::f64::consts::PI * r * r * sinc * c
        })
        .collect();
    trapz(r_grid, &integrand)
}

/// Eq. (S1): the dimensionless power spectrum `P_delta-t`, in Elor et al.'s own `k/beta` units.
///
/// Known limitation, same as the Python source: for `k_elor` beyond about 5-10 the fixed-grid
/// Fourier sine transform under-resolves the oscillatory integrand and can return small negative
/// numerical noise on the true spectrum's tiny-but-positive `k^-3` tail; clipped to zero. This
/// region is never reached by the CMB bound calculation below (whose scan stays within a factor
/// of ten of the peak at `k_elor ~ 0.5`).
pub fn p_exact_k_elor(k_elor: f64) -> f64 {
    (k_elor.powi(3) / (2.0 * std::f64::consts::PI.powi(2)) * p_beta_dtc_exact(k_elor)).max(0.0)
}

/// The exact spectrum's peak, `k_elor = 0.493` (see the crate's own tests for the independent
/// check of this value), converted to Koren-Tsai-Wang's own rescaled `xi` variable.
pub fn xi_peak_exact() -> f64 {
    0.493 * (8.0 * std::f64::consts::PI).powf(1.0 / 3.0)
}

// ============================================================================================
// Cosmology background (Planck 2018 best-fit, as adopted by Koren-Tsai-Wang Sec. III).
// ============================================================================================

pub const H0: f64 = 68.0; // km/s/Mpc
pub const OM: f64 = 0.31;
pub const OL: f64 = 0.69;
pub const C_LIGHT: f64 = 299_792.458; // km/s
pub const T_CMB_UK: f64 = 2.7255e6; // muK

pub fn h_over_h0(z: f64) -> f64 {
    (OL + OM * (1.0 + z).powi(3)).sqrt()
}

pub fn hstar_invmpc(zpt: f64) -> f64 {
    (H0 / C_LIGHT) * h_over_h0(zpt)
}

/// Comoving distance (no-PT LCDM), Mpc, from `zf` to `zi` (`zi > zf`).
pub fn chi0(zi: f64, zf: f64) -> f64 {
    let h0_invmpc = H0 / C_LIGHT;
    quad_gl(|z| 1.0 / ((1.0 + z) * h0_invmpc * h_over_h0(z)), zf, zi, 64)
}

// ============================================================================================
// The bubble-time spectrum in `hat_k = k/H*` units, and the redshift-perturbation power spectrum.
// ============================================================================================

pub fn xi_of_hatk(hat_k: f64, zpt: f64, beta_over_h: f64, vw: f64) -> f64 {
    (8.0 * std::f64::consts::PI).powf(1.0 / 3.0) * vw * (1.0 + zpt) * hat_k / beta_over_h
}

pub fn hatk_of_xi(xi: f64, zpt: f64, beta_over_h: f64, vw: f64) -> f64 {
    xi * beta_over_h / ((8.0 * std::f64::consts::PI).powf(1.0 / 3.0) * vw * (1.0 + zpt))
}

/// The exact bubble-time spectrum in `hat_k` units (the sole spectrum this crate ports — see the
/// module docstring for why the Python source's original interpolated approximation is not).
pub fn pdt_hatk_exact(hat_k: f64, zpt: f64, beta_over_h: f64, vw: f64) -> f64 {
    let k_elor = (1.0 + zpt) * hat_k / beta_over_h / vw;
    p_exact_k_elor(k_elor) / (beta_over_h * beta_over_h)
}

/// Eq. (12), worked entirely in `hat_k = k/H*` units.
pub fn idt(hat_k: f64, zpt: f64, beta_over_h: f64, vw: f64, n_r: usize) -> f64 {
    let xi_peak = xi_peak_exact();
    let hatk_peak = hatk_of_xi(xi_peak, zpt, beta_over_h, vw);
    let (kmin, kmax) = (hatk_peak / 10.0, hatk_peak * 10.0);
    let log_r: Vec<f64> = (0..n_r)
        .map(|i| kmin.ln() + (kmax.ln() - kmin.ln()) * i as f64 / (n_r as f64 - 1.0))
        .collect();
    let r_grid: Vec<f64> = log_r.iter().map(|&lr| lr.exp()).collect();
    let (mu, mu_w) = gauss_legendre_cached(16);
    let vals: Vec<f64> = r_grid
        .iter()
        .map(|&r| {
            let inner: f64 = mu
                .iter()
                .zip(mu_w.iter())
                .map(|(&m, &w)| {
                    let s = (hat_k * hat_k + r * r - 2.0 * hat_k * r * m).max(1e-12).sqrt();
                    w * pdt_hatk_exact(s, zpt, beta_over_h, vw) / s.powi(3)
                })
                .sum();
            pdt_hatk_exact(r, zpt, beta_over_h, vw) * inner
        })
        .collect();
    let outer = trapz(&log_r, &vals);
    hat_k.powi(3) * outer
}

pub fn pdz0_hatk(hat_k: f64, zpt: f64, r_pt: f64, beta_over_h: f64, vw: f64) -> f64 {
    let pref = r_pt * r_pt * OL * OL * (OL + OM * (1.0 + zpt).powi(3)).powi(-3);
    pref * idt(hat_k, zpt, beta_over_h, vw, 40)
}

// ============================================================================================
// Spherical Bessel functions j_ell(x), via Miller's downward-recursion algorithm (stable for any
// x, n — the standard robust construction; here x stays in the tens-to-few-hundreds range and
// n in 2..=50 for the parameter ranges this crate actually scans, so this is cheap in practice).
// ============================================================================================

pub fn spherical_jn(n: usize, x: f64) -> f64 {
    if x.abs() < 1e-8 {
        if n == 0 {
            return 1.0;
        }
        // small-x series: j_n(x) ~ x^n / (2n+1)!!
        let mut double_fact = 1.0_f64;
        let mut k = 2 * n as i64 - 1;
        while k > 0 {
            double_fact *= k as f64;
            k -= 2;
        }
        return x.powi(n as i32) / double_fact;
    }
    let nstart = n + 20 + (x.abs() as usize);
    let mut j_np1 = 0.0_f64;
    let mut j_n = 1e-300_f64;
    let mut j_at_target = if n == nstart { j_n } else { 0.0 };
    for k in (1..=nstart).rev() {
        let j_nm1 = ((2 * k + 1) as f64 / x) * j_n - j_np1;
        j_np1 = j_n;
        j_n = j_nm1;
        if k - 1 == n {
            j_at_target = j_n;
        }
        if j_n.abs() > 1e250 {
            j_n *= 1e-250;
            j_np1 *= 1e-250;
            j_at_target *= 1e-250;
        }
    }
    let true_j0 = x.sin() / x;
    let scale = true_j0 / j_n;
    j_at_target * scale
}

/// Eq. (13), restoring `T_CMB^2` to express the result in `muK^2` (the design doc: this factor is
/// not spelled out explicitly in Eq. 13 but is required to match the paper's `muK^2` axis).
pub fn d_ell_pt(ell: usize, zpt: f64, r_pt: f64, beta_over_h: f64, vw: f64, n_k: usize) -> f64 {
    let xi_peak = xi_peak_exact();
    let hstar = hstar_invmpc(zpt);
    let delta_tau = chi0(zpt, 0.0);
    let hatk_peak = hatk_of_xi(xi_peak, zpt, beta_over_h, vw);
    let (hatk_min, hatk_max) = (hatk_peak / 10.0, hatk_peak * 10.0);
    let log_k: Vec<f64> = (0..n_k)
        .map(|i| hatk_min.ln() + (hatk_max.ln() - hatk_min.ln()) * i as f64 / (n_k as f64 - 1.0))
        .collect();
    let integrand: Vec<f64> = log_k
        .iter()
        .map(|&lk| {
            let hk = lk.exp();
            let k_phys = hk * hstar;
            let jl = spherical_jn(ell, k_phys * delta_tau);
            let pz0 = pdz0_hatk(hk, zpt, r_pt, beta_over_h, vw);
            pz0 * jl * jl
        })
        .collect();
    let val = trapz(&log_k, &integrand);
    2.0 * ell as f64 * (ell as f64 + 1.0) * val * T_CMB_UK * T_CMB_UK
}

// ============================================================================================
// Real Planck 2018 TT power spectrum, and the chi-squared 2-sigma bound (Eq. 14).
// ============================================================================================

const PLANCK_TT_TXT: &str = include_str!("../data/COM_PowerSpect_CMB-TT-full_R3.01.txt");

/// `(ell, D_ell [muK^2], symmetrized 1-sigma error [muK^2])`, parsed once from the real Planck
/// 2018 TT power-spectrum table (Planck Legacy Archive; cross-checked in the Python source against
/// Aghanim et al. 2020, arXiv:1807.06209: the low-`ell` plateau and first-peak amplitude match).
fn planck_table() -> &'static (Vec<f64>, Vec<f64>, Vec<f64>) {
    static TABLE: OnceLock<(Vec<f64>, Vec<f64>, Vec<f64>)> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut ell = Vec::new();
        let mut dl = Vec::new();
        let mut sigma = Vec::new();
        for line in PLANCK_TT_TXT.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<f64> = line.split_whitespace().filter_map(|s| s.parse().ok()).collect();
            if cols.len() < 4 {
                continue;
            }
            ell.push(cols[0]);
            dl.push(cols[1]);
            sigma.push(0.5 * (cols[2] + cols[3])); // symmetrized 1-sigma bar
        }
        (ell, dl, sigma)
    })
}

fn nearest_idx(ell_table: &[f64], ell: f64) -> usize {
    ell_table
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (**a - ell).abs().partial_cmp(&(**b - ell).abs()).unwrap())
        .map(|(i, _)| i)
        .unwrap()
}

pub fn sigma_ell(ell: f64) -> f64 {
    let (ell_t, _, sigma_t) = planck_table();
    sigma_t[nearest_idx(ell_t, ell)]
}

/// Planck's own measured `D_ell` (muK^2) — used by the Fisher-forecast cosmic-variance floor.
pub fn dl_ell(ell: f64) -> f64 {
    let (ell_t, dl_t, _) = planck_table();
    dl_t[nearest_idx(ell_t, ell)]
}

const ELL_GRID: [usize; 10] = [3, 5, 7, 9, 12, 15, 20, 25, 30, 40];

/// Coarse peak-finder for `D_ell,pt`, on the same sparse `ell` grid as the Python source (kept
/// sparse to bound cost; this crate is fast enough to widen it, but the sparse grid is what the
/// design doc's own published numbers used, so it is reproduced here rather than "improved" out
/// from under the comparison).
pub fn peak_ell(zpt: f64, beta_over_h: f64, r_pt: f64) -> usize {
    ELL_GRID
        .iter()
        .map(|&l| (l, d_ell_pt(l, zpt, r_pt, beta_over_h, 1.0, 60)))
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(l, _)| l)
        .unwrap()
}

/// Eq. (14), three `ell`-bins centered on `ell_peak`. `sigma_func` selects the noise curve
/// (Planck's own measured bars by default; the Fisher forecast passes a cosmic-variance-floor
/// version instead).
pub fn chi2_for_r(
    r_pt: f64,
    zpt: f64,
    beta_over_h: f64,
    ell_peak: usize,
    sigma_func: impl Fn(f64) -> f64,
) -> f64 {
    let ells: Vec<usize> = [ell_peak.saturating_sub(1), ell_peak, ell_peak + 1]
        .into_iter()
        .filter(|&l| l >= 2)
        .collect();
    ells.iter()
        .map(|&l| {
            let dl = d_ell_pt(l, zpt, r_pt, beta_over_h, 1.0, 60);
            (dl / sigma_func(l as f64)).powi(2)
        })
        .sum()
}

/// `D_ell,pt(r) = r^2 * D_ell,pt(1)` exactly (Eq. 11: `Pdz0` is proportional to `r^2`), so
/// `chi2(r) = (r/r_probe)^4 * chi2(r_probe)`: solved directly, no root-finder needed.
pub fn r_bound_2sigma(
    zpt: f64,
    beta_over_h: f64,
    chi2_target: f64,
    r_probe: f64,
    sigma_func: impl Fn(f64) -> f64,
) -> (f64, usize) {
    let ell_p = peak_ell(zpt, beta_over_h, r_probe);
    let chi2_probe = chi2_for_r(r_probe, zpt, beta_over_h, ell_p, sigma_func);
    if chi2_probe <= 0.0 {
        return (f64::NAN, ell_p);
    }
    (r_probe * (chi2_target / chi2_probe).powf(0.25), ell_p)
}

// ============================================================================================
// Fisher-style forecast: how much could a next-generation CMB temperature dataset improve the
// bound? (see docs/designs/CMB_FISHER_FORECAST.md in the companion repo for the full write-up).
// ============================================================================================

pub const F_SKY: f64 = 0.7;

/// The cosmic-variance-only noise floor at this `ell`, from Planck's own measured `D_ell`.
pub fn sigma_cv(ell: f64) -> f64 {
    let dl = dl_ell(ell);
    dl * (2.0 / ((2.0 * ell + 1.0) * F_SKY)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// K1: the single-bubble integral reduces to `pi^2/6` exactly at `r->0` (the double-bubble
    /// contribution vanishes there) — a closed-form landmark, not merely a number the Python port
    /// happened to produce.
    #[test]
    fn correlator_at_zero_is_pi_squared_over_six() {
        let c = correlator(1e-4);
        let expected = std::f64::consts::PI.powi(2) / 6.0;
        assert!(
            (c - expected).abs() < 1e-6,
            "correlator(0) = {c}, expected {expected}"
        );
    }

    /// K2: the exact spectrum peaks near k_elor=0.493 with value near 1.077, matching Elor et
    /// al.'s own Fig. S4 (independently checked against the Python port's own `pdt_exact.py`
    /// `__main__` sweep, not merely trusted from its docstring).
    #[test]
    fn exact_spectrum_peaks_near_elor_fig_s4() {
        let mut best_k = 0.0;
        let mut best_v = -1.0;
        let mut k = 0.1;
        while k <= 2.0 {
            let v = p_exact_k_elor(k);
            if v > best_v {
                best_v = v;
                best_k = k;
            }
            k += 0.01;
        }
        assert!((best_k - 0.493).abs() < 0.03, "peak at k={best_k}, expected ~0.493");
        assert!((best_v - 1.077).abs() < 0.05, "peak value={best_v}, expected ~1.077");
    }

    /// K3: reproduces two landmark points of `CMB_CASCADE_REPRODUCTION.md`'s exact-spectrum
    /// results table (beta/H*=100 and beta/H*=500 at zpt=0.1) to within a generous tolerance —
    /// confirming the port is faithful, not exact bit-for-bit reproduction across languages.
    #[test]
    fn r_bound_matches_python_design_doc_landmarks() {
        let (r100, _) = r_bound_2sigma(0.1, 100.0, 5.99, 0.1, sigma_ell);
        let (r500, _) = r_bound_2sigma(0.1, 500.0, 5.99, 0.1, sigma_ell);
        assert!(
            (r100 - 0.084).abs() / 0.084 < 0.30,
            "r_bound(beta/H*=100) = {r100}, expected ~0.084 (design doc: 0.08405)"
        );
        assert!(
            (r500 - 1.425).abs() / 1.425 < 0.30,
            "r_bound(beta/H*=500) = {r500}, expected ~1.425 (design doc: 1.425)"
        );
    }

    /// K4: Planck's own TT bars are close to (within a factor of ~2, at worst, at the lowest
    /// multipoles) the pure cosmic-variance floor at ell=5-50 — the Fisher-forecast's headline
    /// finding, checked here as a smoke test that the cosmic-variance code path runs and returns
    /// a sane, positive, same-order-of-magnitude number.
    #[test]
    fn cosmic_variance_floor_is_same_order_as_planck_bars() {
        for &ell in &[5.0, 10.0, 20.0, 30.0, 50.0] {
            let sp = sigma_ell(ell);
            let scv = sigma_cv(ell);
            assert!(scv > 0.0 && sp > 0.0, "non-positive sigma at ell={ell}");
            let ratio = sp / scv;
            assert!(
                (0.5..2.5).contains(&ratio),
                "Planck/CV ratio at ell={ell} is {ratio}, expected order-1"
            );
        }
    }

    #[test]
    fn spherical_bessel_matches_closed_forms() {
        // j_0(x) = sin(x)/x, j_1(x) = sin(x)/x^2 - cos(x)/x
        let x = 3.7_f64;
        let j0 = spherical_jn(0, x);
        let j1 = spherical_jn(1, x);
        assert!((j0 - x.sin() / x).abs() < 1e-9);
        assert!((j1 - (x.sin() / (x * x) - x.cos() / x)).abs() < 1e-9);
    }
}
