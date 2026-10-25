//! Rytova-Keldysh interaction of a 2D dielectric sheet and the Madelung energy of a Keldysh Wigner crystal on a
//! uniform neutralising background (Keldysh jellium), by Ewald summation over a Bravais lattice.
//!
//! Units: charge^2 / (4 pi eps0 epsbar) = 1, lengths in any unit, screening length `r0 >= 0`.
//!
//! * Real space (DLMF 11.5.2): `V(r) = (pi/(2 r0)) [H0(r/r0) - Y0(r/r0)] = int_0^inf exp(-r v) / sqrt(1 + r0^2 v^2) dv`.
//! * Reciprocal space: `Vhat(k) = int V(|x|) exp(-i k.x) d^2x = 2 pi / (k (1 + r0 k))`; `r0 = 0` is Coulomb `2 pi / k`.
//! * Gaussian mixture (the definition used in the Lean module `Autoresearch.KeldyshCM`):
//!   `V(r) = int_0^inf w(a) exp(-r^2/(4a)) / (2a) da`, `w(a) = (1/r0) erfcx(sqrt(a)/r0)`, `w -> 1/sqrt(pi a)` as `r0 -> 0`.
//!   Each Gaussian `exp(-r^2/(4a))/(2a)` has transform `2 pi exp(-a k^2)`, so `Vhat(k) = 2 pi int w(a) exp(-a k^2) da`.
//!
//! Ewald split at `a* = 1/(4 alpha^2)`: Gaussians with `a < a*` go to real space, `a > a*` to reciprocal space. For
//! `r0 = 0` this is exactly the `erfc(alpha r)/r` / `2 pi erfc(k/(2 alpha))/k` split of `crate::jellium`. Per particle,
//! at density `rho = 1/A` (one particle per cell of area `A`):
//!
//! `e = 1/2 sum_{v != 0} V_sr(|v|) + (rho/2) sum_{G != 0} Vhat_lr(G) - 1/2 V_lr(0) - (rho/2) Vhat_sr(0)`,
//!
//! with `V_sr(r) = int_0^a* w e^{-r^2/4a}/(2a) da`, `Vhat_lr(G) = 2 pi int_a*^inf w e^{-a G^2} da`,
//! `V_lr(0) = int_a*^inf w/(2a) da`, `Vhat_sr(0) = 2 pi int_0^a* w da = 2 pi [r0 (erfcx(X) - 1) + 2 sqrt(a*/pi)]`,
//! `X = sqrt(a*)/r0` (from `erfcx' = 2x erfcx - 2/sqrt(pi)`). The last term is what survives of the particle-background
//! and background-background energies after the divergent `k = 0` parts cancel (Tier X numerics; not covered by the
//! Lean theorem, which is about the raw, non-neutralised sum).

use std::f64::consts::PI;

const XGK: [f64; 8] = [
    0.991_455_371_120_812_6,
    0.949_107_912_342_758_5,
    0.864_864_423_359_769_1,
    0.741_531_185_599_394_4,
    0.586_087_235_467_691_1,
    0.405_845_151_377_397_2,
    0.207_784_955_007_898_5,
    0.0,
];
const WGK: [f64; 8] = [
    0.022_935_322_010_529_225,
    0.063_092_092_629_978_55,
    0.104_790_010_322_250_18,
    0.140_653_259_715_525_92,
    0.169_004_726_639_267_9,
    0.190_350_578_064_785_4,
    0.204_432_940_075_298_9,
    0.209_482_141_084_727_83,
];
const WG: [f64; 4] = [
    0.129_484_966_168_869_7,
    0.279_705_391_489_276_7,
    0.381_830_050_505_118_9,
    0.417_959_183_673_469_4,
];

/// One Gauss-Kronrod 7-15 panel: (Kronrod estimate, |Kronrod - Gauss|).
fn gk15<F: Fn(f64) -> f64>(f: &F, a: f64, b: f64) -> (f64, f64) {
    let c = 0.5 * (a + b);
    let h = 0.5 * (b - a);
    let fc = f(c);
    let mut k = WGK[7] * fc;
    let mut g = WG[3] * fc;
    for i in 0..7 {
        let d = h * XGK[i];
        let s = f(c - d) + f(c + d);
        k += WGK[i] * s;
        if i % 2 == 1 {
            g += WG[i / 2] * s;
        }
    }
    (k * h, ((k - g) * h).abs())
}

fn adapt<F: Fn(f64) -> f64>(f: &F, a: f64, b: f64, whole: (f64, f64), tol: f64, depth: u32) -> f64 {
    let (est, err) = whole;
    // Absolute tolerance, or a relative floor of 1e-14 (|K - G| cannot fall far below rounding).
    if err <= tol.max(1e-14 * est.abs()) || depth == 0 {
        return est;
    }
    let m = 0.5 * (a + b);
    let l = gk15(f, a, m);
    let r = gk15(f, m, b);
    adapt(f, a, m, l, 0.5 * tol, depth - 1) + adapt(f, m, b, r, 0.5 * tol, depth - 1)
}

/// Adaptive Gauss-Kronrod integral of `f` over `[a, b]` to absolute tolerance `tol` (depth-limited bisection).
pub fn integrate<F: Fn(f64) -> f64>(f: F, a: f64, b: f64, tol: f64) -> f64 {
    if b <= a {
        return 0.0;
    }
    let whole = gk15(&f, a, b);
    adapt(&f, a, b, whole, tol, 30)
}

/// `erfcx(x) = exp(x^2) erfc(x)` for `x >= 0`: direct below 26, asymptotic series above (relative error < 1e-16 there).
pub fn erfcx(x: f64) -> f64 {
    assert!(x >= 0.0, "erfcx is only needed for x >= 0");
    if x < 26.0 {
        return (x * x).exp() * libm::erfc(x);
    }
    let z = 1.0 / (2.0 * x * x);
    let (mut term, mut sum) = (1.0, 1.0);
    for k in 1..12 {
        term *= -((2 * k - 1) as f64) * z;
        sum += term;
    }
    sum / (x * PI.sqrt())
}

/// The Keldysh interaction with screening length `r0 >= 0` (`r0 = 0` is Coulomb `1/r`).
#[derive(Clone, Copy, Debug)]
pub struct Keldysh {
    pub r0: f64,
}

impl Keldysh {
    /// Mixing weight `w(a) = (1/r0) erfcx(sqrt(a)/r0)`; `1/sqrt(pi a)` at `r0 = 0`.
    pub fn weight(&self, a: f64) -> f64 {
        if self.r0 == 0.0 {
            1.0 / (PI * a).sqrt()
        } else {
            erfcx(a.sqrt() / self.r0) / self.r0
        }
    }

    /// `V(r)` from DLMF 11.5.2, `int_0^inf exp(-s) / sqrt(1 + (r0 s / r)^2) ds / r`, by quadrature.
    pub fn potential_dlmf(&self, r: f64) -> f64 {
        let q = self.r0 / r;
        let f = |s: f64| (-s).exp() / (1.0 + q * q * s * s).sqrt();
        let pts = [0.0, 0.5, 2.0, 6.0, 15.0, 40.0, 80.0];
        let mut acc = 0.0;
        for w in pts.windows(2) {
            acc += integrate(f, w[0], w[1], 1e-17);
        }
        acc / r
    }

    /// `V(r)` from the Gaussian mixture, in the log variable `a = exp(y)`.
    pub fn potential_mixture(&self, r: f64) -> f64 {
        self.mixture_window(r, f64::NEG_INFINITY, f64::INFINITY)
    }

    /// `int_{a_lo}^{a_hi} w(a) exp(-r^2/(4a)) / (2a) da` (a window of the mixture), in `y = ln a`.
    fn mixture_window(&self, r: f64, a_lo: f64, a_hi: f64) -> f64 {
        let r2 = r * r;
        // Below a = r^2/2800 the Gaussian factor is < e^-700; above a_top the tail is < 1e-18 relative.
        let y_lo = (r2 / 2800.0).ln().max(if a_lo > 0.0 {
            a_lo.ln()
        } else {
            f64::NEG_INFINITY
        });
        let top = (r2 / 4.0).max(self.r0 * self.r0).max(1e-300).ln() + 90.0;
        let y_hi = top.min(if a_hi.is_finite() {
            a_hi.ln()
        } else {
            f64::INFINITY
        });
        if y_hi <= y_lo {
            return 0.0;
        }
        let f = |y: f64| {
            let a = y.exp();
            0.5 * self.weight(a) * (-r2 / (4.0 * a)).exp()
        };
        let n = ((y_hi - y_lo) / 2.0).ceil().max(1.0) as usize;
        let h = (y_hi - y_lo) / n as f64;
        (0..n)
            .map(|i| integrate(f, y_lo + i as f64 * h, y_lo + (i + 1) as f64 * h, 1e-19))
            .sum()
    }

    /// The exact reciprocal kernel `2 pi / (k (1 + r0 k))`.
    pub fn kernel(&self, k: f64) -> f64 {
        2.0 * PI / (k * (1.0 + self.r0 * k))
    }

    /// Short-range (real-space) part `V_sr(r) = int_0^a* w e^{-r^2/4a}/(2a) da`.
    pub fn v_short(&self, r: f64, astar: f64) -> f64 {
        self.mixture_window(r, 0.0, astar)
    }

    /// Long-range part in reciprocal space, `2 pi int_a*^inf w(a) exp(-a k^2) da`, with `a = a* + t/k^2`.
    pub fn vhat_long(&self, k: f64, astar: f64) -> f64 {
        let k2 = k * k;
        let f = |t: f64| self.weight(astar + t / k2) * (-t).exp();
        let pts = [0.0, 0.5, 2.0, 6.0, 15.0, 40.0];
        let s: f64 = pts
            .windows(2)
            .map(|w| integrate(f, w[0], w[1], 1e-19))
            .sum();
        2.0 * PI * (-astar * k2).exp() * s / k2
    }

    /// Short-range part in reciprocal space, `2 pi int_0^a* w(a) exp(-a k^2) da` (used only for checks).
    pub fn vhat_short(&self, k: f64, astar: f64) -> f64 {
        let k2 = k * k;
        // a = a* u^2 removes the a^{-1/2} endpoint singularity of w at r0 = 0.
        let f = |u: f64| {
            let a = astar * u * u;
            self.weight(a) * (-a * k2).exp() * 2.0 * astar * u
        };
        2.0 * PI * integrate(f, 0.0, 1.0, 1e-17)
    }

    /// `V_lr(0) = int_a*^inf w/(2a) da`. For `r0 > 0`: `(1/r0) int_0^{1/X} erfcx(1/u)/u du`, `X = sqrt(a*)/r0`.
    pub fn v_long_at_zero(&self, astar: f64) -> f64 {
        if self.r0 == 0.0 {
            return 1.0 / (PI * astar).sqrt();
        }
        let umax = self.r0 / astar.sqrt();
        let f = |u: f64| erfcx(1.0 / u) / u;
        let mut s = integrate(f, 0.0, umax.min(1.0), 1e-18);
        if umax > 1.0 {
            // u = e^y on [1, umax]: the integrand erfcx(e^-y) is smooth and tends to 1.
            let ymax = umax.ln();
            let n = (ymax / 2.0).ceil().max(1.0) as usize;
            let h = ymax / n as f64;
            let g = |y: f64| erfcx((-y).exp());
            s += (0..n)
                .map(|i| integrate(g, i as f64 * h, (i + 1) as f64 * h, 1e-18))
                .sum::<f64>();
        }
        s / self.r0
    }

    /// `Vhat_sr(0) = 2 pi int_0^a* w da = 2 pi [r0 (erfcx(X) - 1) + 2 sqrt(a*/pi)]`, `X = sqrt(a*)/r0`.
    /// For `X < 0.1` the two leading terms cancel; there we use `erfcx(X) = sum_n (-X)^n / Gamma(1 + n/2)` from `n = 2`.
    pub fn vhat_short_at_zero(&self, astar: f64) -> f64 {
        if self.r0 == 0.0 {
            return 2.0 * PI * 2.0 * (astar / PI).sqrt();
        }
        let x = astar.sqrt() / self.r0;
        if x >= 0.1 {
            return 2.0 * PI * (self.r0 * (erfcx(x) - 1.0) + 2.0 * (astar / PI).sqrt());
        }
        // Gamma(1 + n/2) by Gamma(1 + n/2) = (n/2) Gamma(n/2), from Gamma(1) = 1, Gamma(3/2) = sqrt(pi)/2.
        let (mut g_even, mut g_odd) = (1.0, 0.5 * PI.sqrt());
        let mut s = 0.0;
        for n in 2..30 {
            let g = if n % 2 == 0 {
                g_even *= n as f64 / 2.0;
                g_even
            } else {
                g_odd *= n as f64 / 2.0;
                g_odd
            };
            s += (-x).powi(n) / g;
        }
        2.0 * PI * self.r0 * s
    }
}

/// The four Ewald terms of the per-particle Madelung energy (already multiplied by their prefactors).
#[derive(Clone, Copy, Debug)]
pub struct MadelungParts {
    pub real: f64,
    pub recip: f64,
    pub self_term: f64,
    pub background: f64,
}

impl MadelungParts {
    pub fn total(&self) -> f64 {
        self.real + self.recip + self.self_term + self.background
    }
}

/// Per-particle Keldysh-jellium Madelung energy of the Bravais lattice spanned by `b1, b2`, with Ewald parameter
/// `alpha` (`a* = 1/(4 alpha^2)`); the sums are cut where the neglected terms are below `exp(-tail)`.
pub fn madelung_parts(
    kel: &Keldysh,
    b1: [f64; 2],
    b2: [f64; 2],
    alpha: f64,
    tail: f64,
) -> MadelungParts {
    let astar = 1.0 / (4.0 * alpha * alpha);
    let det = b1[0] * b2[1] - b1[1] * b2[0];
    let area = det.abs();
    let rc = tail.sqrt() / alpha;
    let gc = 2.0 * alpha * tail.sqrt();
    let g1 = [2.0 * PI * b2[1] / det, -2.0 * PI * b2[0] / det];
    let g2 = [-2.0 * PI * b1[1] / det, 2.0 * PI * b1[0] / det];
    let lattice_sum = |u: [f64; 2], w: [f64; 2], cut: f64, f: &dyn Fn(f64) -> f64| -> f64 {
        let lmax = (u[0].hypot(u[1])).max(w[0].hypot(w[1]));
        let ucell = (u[0] * w[1] - u[1] * w[0]).abs();
        let m = (cut / (ucell / lmax)).ceil() as i64 + 1;
        let mut s = 0.0;
        for i in -m..=m {
            for j in -m..=m {
                if i == 0 && j == 0 {
                    continue;
                }
                let vx = i as f64 * u[0] + j as f64 * w[0];
                let vy = i as f64 * u[1] + j as f64 * w[1];
                let r = vx.hypot(vy);
                if r < cut {
                    s += f(r);
                }
            }
        }
        s
    };
    let real = 0.5 * lattice_sum(b1, b2, rc, &|r| kel.v_short(r, astar));
    let recip = 0.5 / area * lattice_sum(g1, g2, gc, &|k| kel.vhat_long(k, astar));
    MadelungParts {
        real,
        recip,
        self_term: -0.5 * kel.v_long_at_zero(astar),
        background: -0.5 / area * kel.vhat_short_at_zero(astar),
    }
}

/// Per-particle Madelung energy (sum of the four Ewald terms).
pub fn madelung(kel: &Keldysh, b1: [f64; 2], b2: [f64; 2], alpha: f64) -> f64 {
    madelung_parts(kel, b1, b2, alpha, 46.0).total()
}

/// Density-one triangular basis.
pub fn triangular_basis() -> ([f64; 2], [f64; 2]) {
    let a = crate::spacing_density_one();
    ([a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a])
}

/// Density-one square basis.
pub fn square_basis() -> ([f64; 2], [f64; 2]) {
    ([1.0, 0.0], [0.0, 1.0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jellium::{ETA_SQUARE, ETA_TRIANGULAR, Jellium};
    use crate::{spacing_density_one, triangular};

    #[test]
    fn gauss_kronrod_is_exact_on_polynomials_and_accurate_on_exp() {
        let p = integrate(|x: f64| x.powi(20), 0.0, 1.0, 1e-16);
        assert!((p - 1.0 / 21.0).abs() < 1e-15, "{p}");
        let e = integrate(|x: f64| x.exp(), 0.0, 3.0, 1e-15);
        assert!((e - (3.0_f64.exp() - 1.0)).abs() < 1e-13, "{e}");
    }

    #[test]
    fn erfcx_matches_mpmath() {
        // Values from scripts/keldysh/mpmath_refs.py (mpmath, 40 digits).
        for (x, v) in [
            (0.5, 0.6156903441929259),
            (3.0, 0.17900115118138996),
            (25.5, 0.022108108052519827),
            (26.5, 0.021275046685371106),
            (1000.0, 0.0005641893014533876),
            (1e8, 5.641895835477562e-09),
        ] {
            assert!(
                ((erfcx(x) - v) / v).abs() < 2e-14,
                "erfcx({x}) = {} vs {v}",
                erfcx(x)
            );
        }
    }

    #[test]
    fn dlmf_and_mixture_agree_with_struve_closed_form() {
        // (r0, r, mpmath (pi/(2 r0)) [H0 - Y0](r/r0)).
        for (r0, r, v) in [
            (0.05, 0.1, 8.811638878737215),
            (0.3, 2.0, 0.49033437042333833),
            (1.0, 1.0, 0.7546100257709721),
            (4.0, 0.5, 0.5769220857738825),
            (4.0, 12.0, 0.07754084438316729),
        ] {
            let k = Keldysh { r0 };
            let (d, m) = (k.potential_dlmf(r), k.potential_mixture(r));
            assert!(
                ((d - v) / v).abs() < 1e-12,
                "DLMF r0={r0} r={r}: {d} vs {v}"
            );
            assert!(
                ((m - v) / v).abs() < 1e-11,
                "mixture r0={r0} r={r}: {m} vs {v}"
            );
        }
    }

    #[test]
    fn split_kernels_add_up_to_exact_kernel() {
        for r0 in [0.0, 0.2, 3.0] {
            let k = Keldysh { r0 };
            for q in [0.3, 2.0, 7.0] {
                let s = k.vhat_short(q, 0.11) + k.vhat_long(q, 0.11);
                let ex = k.kernel(q);
                assert!(((s - ex) / ex).abs() < 1e-11, "r0={r0} k={q}: {s} vs {ex}");
            }
        }
        // Closed-form background integral vs quadrature.
        let k = Keldysh { r0: 0.7 };
        let quad = 2.0 * PI * integrate(|u: f64| k.weight(0.2 * u * u) * 0.4 * u, 0.0, 1.0, 1e-17);
        assert!((quad - k.vhat_short_at_zero(0.2)).abs() < 1e-13);
    }

    #[test]
    fn coulomb_limit_reproduces_jellium_rs_and_literature() {
        let (b1, b2) = triangular_basis();
        let ec = madelung(&Keldysh { r0: 0.0 }, b1, b2, 1.3);
        let a = spacing_density_one();
        let j = Jellium::new(16.0 * a, 9.0 * 3.0_f64.sqrt() * a, 0.66, 8.0, 7.0);
        let ebox = j.energy(&triangular(16, 18, a)) / 288.0;
        assert!(
            (ec - ebox).abs() < 1e-10,
            "lattice {ec} vs jellium.rs box {ebox}"
        );
        assert!((ec - ETA_TRIANGULAR * PI.sqrt()).abs() < 1e-8);
        let (s1, s2) = square_basis();
        let esq = madelung(&Keldysh { r0: 0.0 }, s1, s2, 1.3);
        assert!((esq - ETA_SQUARE * PI.sqrt()).abs() < 1e-8);
        // r0 -> 0 through the r0 > 0 code path (erfcx branch): within 1e-8 of Coulomb at r0 = 1e-9.
        let e9 = madelung(&Keldysh { r0: 1e-9 }, b1, b2, 1.3);
        assert!((e9 - ec).abs() < 1e-8, "{e9} vs {ec}");
    }

    #[test]
    fn energy_independent_of_ewald_parameter() {
        for r0 in [0.1, 1.0, 5.0] {
            let (b1, b2) = triangular_basis();
            let e: Vec<f64> = [0.8, 1.6, 3.2]
                .iter()
                .map(|&al| madelung(&Keldysh { r0 }, b1, b2, al))
                .collect();
            for v in &e[1..] {
                assert!(((v - e[0]) / e[0]).abs() < 1e-10, "r0={r0}: {:?}", e);
            }
        }
    }

    #[test]
    fn negative_control_wrong_sign_reciprocal_kernel_breaks_alpha_independence() {
        let (b1, b2) = triangular_basis();
        let kel = Keldysh { r0: 1.0 };
        let wrong = |al: f64| {
            let p = madelung_parts(&kel, b1, b2, al, 46.0);
            p.real - p.recip + p.self_term + p.background
        };
        assert!(
            (wrong(1.6) - wrong(3.2)).abs() > 1e-2,
            "{} {}",
            wrong(1.6),
            wrong(3.2)
        );
    }

    #[test]
    fn small_x_series_matches_direct_background_formula() {
        // X = sqrt(a*)/r0 = 0.0999: series branch vs the direct formula (both accurate there).
        let astar = 0.04;
        let k = Keldysh { r0: 0.2 / 0.0999 };
        let direct = 2.0 * PI * (k.r0 * (erfcx(0.0999) - 1.0) + 2.0 * (astar / PI).sqrt());
        let series = k.vhat_short_at_zero(astar);
        assert!(
            ((series - direct) / direct).abs() < 1e-12,
            "{series} vs {direct}"
        );
        // Large r0: alpha independence still holds (self and background terms in their large-r0 forms).
        let (b1, b2) = triangular_basis();
        let kk = Keldysh { r0: 1e4 };
        let (e1, e2) = (madelung(&kk, b1, b2, 0.8), madelung(&kk, b1, b2, 3.2));
        assert!(((e1 - e2) / e1).abs() < 1e-9, "{e1} vs {e2}");
    }

    #[test]
    fn triangular_below_square() {
        let (t1, t2) = triangular_basis();
        let (s1, s2) = square_basis();
        for r0 in [0.0, 0.5, 4.0] {
            let k = Keldysh { r0 };
            assert!(
                madelung(&k, t1, t2, 1.3) < madelung(&k, s1, s2, 1.3),
                "r0={r0}"
            );
        }
    }
}
