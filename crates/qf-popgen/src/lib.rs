//! Absorption probabilities of finite Markov chains on `{0, ..., N}` by CVODE, with a fail-closed
//! certificate.
//!
//! # What is computed
//!
//! For a stochastic kernel `P` on `{0, ..., N}` with `0` and `N` absorbing, the *uniformised*
//! continuous-time chain has law `pi(t) = pi(0) exp(t (P - I))`, i.e. the linear ODE
//! `d pi / dt = pi (P - I)`. Since `exp(t (P - I)) = sum_n Poisson(t; n) P^n`, the absorption mass
//! `pi_N(t)` is a Poisson mixture of the n-step probabilities `P^n(i, N)`, and it converges to the
//! same limit. The n-step statement itself is what the Lean modules
//! `Programmes.PopGen.{Absorption, WrightFisherFixation, MoranModel, MoranFixation}` prove
//! (AutoevolveAI repository, `formal_cert/Programmes/PopGen/`): `P^n(i, N) -> h(i)` with
//! `h(i) = i/N` (neutral Wright-Fisher, and Moran with `r = 1`) and `h(i) = S_i / S_N`,
//! `S_n = sum_{k<n} r^{-k}`, for the Moran model with fitness `r`.
//!
//! The Poisson-mixture step and the identification of `P^n(i, N)` with a probability are *not*
//! formalised; this crate is therefore a floating-point (Tier X) cross-check and never supports a
//! claim. What it adds is an independent integrator (BDF with the exact linear Jacobian) and a
//! *fail-closed* certificate: a run is accepted only if the kernel is a valid absorbing kernel, the
//! total mass is conserved, *every component* of the state stays non-negative (up to a tolerance;
//! a signed sum alone can hide cancelling entries), the interior mass has decayed below a
//! tolerance, and the absorption mass matches the target within a tolerance. Anything else is an
//! error, never a silent pass.

use cvode::{Cvode, DenseMat, Method, Task};
use nvector::SerialVector;

/// A finite kernel on `{0, ..., n}`: `p[i][j]` is the probability of jumping from `i` to `j`.
#[derive(Debug, Clone)]
pub struct Chain {
    /// Largest state index `N` (the kernel is `(N + 1) x (N + 1)`).
    pub n: usize,
    /// Row-stochastic kernel.
    pub p: Vec<Vec<f64>>,
}

/// Reasons a certificate is refused.
#[derive(Debug, Clone, PartialEq)]
pub enum CertError {
    /// The kernel has a negative entry, a row not summing to 1, or a non-absorbing end state.
    InvalidKernel(String),
    /// CVODE set-up or stepping failed.
    Solver(String),
    /// A non-finite value appeared in the state.
    NonFinite,
    /// Total probability mass drifted by more than the tolerance.
    MassDrift {
        /// Largest `|sum pi - 1|` observed.
        drift: f64,
    },
    /// A component of the state fell below `-tol.negative` (probabilities cannot be negative).
    NegativeMass {
        /// State index of the most negative component.
        state: usize,
        /// Its value.
        value: f64,
    },
    /// The interior mass did not fall below the tolerance by `t_max`.
    NotAbsorbed {
        /// Interior mass at `t_max`.
        interior: f64,
        /// Time reached.
        t: f64,
    },
    /// The absorption mass at `N` differs from the target by more than the tolerance.
    ValueMismatch {
        /// CVODE absorption mass.
        got: f64,
        /// Target value.
        target: f64,
    },
}

impl std::fmt::Display for CertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Moran birth-death kernel with fitness `r` (the individual that dies is uniform over all `N`):
/// up `p_i = r i/(r i + N - i) (N - i)/N`, down `q_i = (N - i)/(r i + N - i) i/N`.
pub fn moran_kernel(n: usize, r: f64) -> Chain {
    let nf = n as f64;
    let mut p = vec![vec![0.0; n + 1]; n + 1];
    for i in 0..=n {
        let x = i as f64;
        let up = r * x / (r * x + nf - x) * (nf - x) / nf;
        let down = (nf - x) / (r * x + nf - x) * x / nf;
        if i < n {
            p[i][i + 1] = up;
        }
        if i > 0 {
            p[i][i - 1] = down;
        }
        p[i][i] = 1.0 - up - down;
    }
    Chain { n, p }
}

/// Neutral Wright-Fisher kernel: `P(i, j) = C(N, j) (i/N)^j (1 - i/N)^(N - j)`.
pub fn wright_fisher_kernel(n: usize) -> Chain {
    let nf = n as f64;
    let mut binom = vec![1.0_f64; n + 1];
    for j in 1..=n {
        binom[j] = binom[j - 1] * (n - j + 1) as f64 / j as f64;
    }
    let mut p = vec![vec![0.0; n + 1]; n + 1];
    for (i, row) in p.iter_mut().enumerate() {
        let x = i as f64 / nf;
        for (j, e) in row.iter_mut().enumerate() {
            *e = binom[j] * x.powi(j as i32) * (1.0 - x).powi((n - j) as i32);
        }
    }
    Chain { n, p }
}

/// The identity kernel: every state absorbing (a valid absorbing kernel with no absorption flow).
pub fn identity_kernel(n: usize) -> Chain {
    let mut p = vec![vec![0.0; n + 1]; n + 1];
    for (i, row) in p.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    Chain { n, p }
}

/// Moran fixation function `S_i / S_N`, `S_n = sum_{k<n} r^{-k}` (the Lean `moranFix`).
pub fn moran_fix(n: usize, r: f64, i: usize) -> f64 {
    let s = |m: usize| (0..m).map(|k| r.powi(-(k as i32))).sum::<f64>();
    s(i) / s(n)
}

/// Exponent-flipped (wrong) candidate `(1 - r^i)/(1 - r^N)`, used as a negative control.
pub fn flipped_fix(n: usize, r: f64, i: usize) -> f64 {
    (1.0 - r.powi(i as i32)) / (1.0 - r.powi(n as i32))
}

impl Chain {
    /// Check non-negativity, unit row sums (to `1e-12`) and absorbing end states.
    pub fn validate(&self) -> Result<(), CertError> {
        let m = self.n + 1;
        if self.p.len() != m || self.p.iter().any(|row| row.len() != m) {
            return Err(CertError::InvalidKernel("shape".into()));
        }
        for (i, row) in self.p.iter().enumerate() {
            if row.iter().any(|&x| !x.is_finite() || x < -1e-15) {
                return Err(CertError::InvalidKernel(format!(
                    "row {i}: negative or non-finite"
                )));
            }
            let s: f64 = row.iter().sum();
            if (s - 1.0).abs() > 1e-12 {
                return Err(CertError::InvalidKernel(format!("row {i} sums to {s}")));
            }
        }
        if (self.p[0][0] - 1.0).abs() > 1e-15 || (self.p[self.n][self.n] - 1.0).abs() > 1e-15 {
            return Err(CertError::InvalidKernel(
                "end states are not absorbing".into(),
            ));
        }
        Ok(())
    }
}

/// CVODE tolerances and limits.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    /// Relative tolerance.
    pub rtol: f64,
    /// Absolute tolerance.
    pub atol: f64,
    /// Step limit per output interval.
    pub max_steps: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rtol: 1e-11,
            atol: 1e-15,
            max_steps: 2_000_000,
        }
    }
}

/// Acceptance thresholds of a certificate.
#[derive(Debug, Clone, Copy)]
pub struct Tolerances {
    /// Largest accepted interior mass at the stopping time.
    pub interior: f64,
    /// Largest accepted `|pi_N(t) - target|`.
    pub value: f64,
    /// Largest accepted `|sum pi - 1|` along the run.
    pub mass: f64,
    /// Most negative component accepted: every `pi_k(t)` must be `>= -negative` at every output.
    pub negative: f64,
}

impl Default for Tolerances {
    fn default() -> Self {
        Self {
            interior: 1e-9,
            value: 1e-8,
            mass: 1e-9,
            negative: 1e-12,
        }
    }
}

/// One accepted run.
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Start state.
    pub start: usize,
    /// Time at which the interior mass first fell below the tolerance.
    pub t_stop: f64,
    /// `pi_N(t_stop)`.
    pub p_fix: f64,
    /// `pi_0(t_stop)`.
    pub p_loss: f64,
    /// Interior mass at `t_stop`.
    pub interior: f64,
    /// Largest `|sum pi - 1|` over the outputs.
    pub mass_drift: f64,
    /// Smallest component `min_k pi_k(t)` seen at any output (accepted only if `>= -tol.negative`).
    pub min_component: f64,
    /// `(t, pi_N(t), interior(t))` at the doubling output times.
    pub trace: Vec<(f64, f64, f64)>,
}

/// Componentwise non-negativity check: returns the smallest component of `y`, or
/// [`CertError::NegativeMass`] if it is below `-tol`.
///
/// The total-mass and interior-mass tests use *signed* sums, so a state such as
/// `(0.5, -0.25, 0.25, 0.5)` has total mass `1` and interior mass `0`; this check refuses it.
pub fn check_nonnegative(y: &[f64], tol: f64) -> Result<f64, CertError> {
    let mut state = 0usize;
    let mut min = f64::INFINITY;
    for (k, &v) in y.iter().enumerate() {
        if v < min {
            min = v;
            state = k;
        }
    }
    if min < -tol {
        return Err(CertError::NegativeMass { state, value: min });
    }
    Ok(min)
}

/// Integrate `d pi/dt = pi (P - I)` from the point mass at `start`, stopping when the interior mass
/// is below `tol.interior` (outputs at `t = 1, 2, 4, ...` up to `t_max`), and compare `pi_N` with
/// `target`. Fail-closed: every failure is an `Err`.
pub fn certify_absorption(
    chain: &Chain,
    start: usize,
    target: f64,
    t_max: f64,
    tol: Tolerances,
    set: Settings,
) -> Result<Certificate, CertError> {
    chain.validate()?;
    let m = chain.n + 1;
    if start >= m {
        return Err(CertError::InvalidKernel("start out of range".into()));
    }
    let p_rhs = chain.p.clone();
    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        for (j, yd) in ydot.iter_mut().enumerate() {
            let mut acc = -y[j];
            for (k, yk) in y.iter().enumerate() {
                acc += yk * p_rhs[k][j];
            }
            *yd = acc;
        }
        Ok(())
    };
    let p_jac = chain.p.clone();
    let jac = move |_t: f64, _y: &[f64], j: &mut DenseMat| -> Result<(), String> {
        for (k, col) in j.cols.iter_mut().enumerate() {
            for (row, e) in col.iter_mut().enumerate() {
                *e = p_jac[k][row] - if k == row { 1.0 } else { 0.0 };
            }
        }
        Ok(())
    };
    let mut y0 = vec![0.0; m];
    y0[start] = 1.0;
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(set.rtol)
        .atol(set.atol)
        .max_steps(set.max_steps)
        .jacobian(jac)
        .build(rhs, 0.0, SerialVector::from_slice(&y0))
        .map_err(|e| CertError::Solver(format!("{e}")))?;

    let mut trace = Vec::new();
    let mut drift: f64 = 0.0;
    let mut min_component = f64::INFINITY;
    let mut t_out = 1.0_f64;
    loop {
        let (t, y) = solver
            .solve(t_out, Task::Normal)
            .map_err(|e| CertError::Solver(format!("at t = {t_out}: {e}")))?;
        if !y.iter().all(|v| v.is_finite()) {
            return Err(CertError::NonFinite);
        }
        min_component = min_component.min(check_nonnegative(y, tol.negative)?);
        let total: f64 = y.iter().sum();
        drift = drift.max((total - 1.0).abs());
        let interior: f64 = y[1..m - 1].iter().sum();
        trace.push((t, y[m - 1], interior));
        if drift > tol.mass {
            return Err(CertError::MassDrift { drift });
        }
        if interior <= tol.interior {
            if (y[m - 1] - target).abs() > tol.value {
                return Err(CertError::ValueMismatch {
                    got: y[m - 1],
                    target,
                });
            }
            return Ok(Certificate {
                start,
                t_stop: t,
                p_fix: y[m - 1],
                p_loss: y[0],
                interior,
                mass_drift: drift,
                min_component,
                trace,
            });
        }
        if t_out >= t_max {
            return Err(CertError::NotAbsorbed { interior, t });
        }
        t_out *= 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(chain: &Chain, i: usize, target: f64, t_max: f64) -> Result<Certificate, CertError> {
        certify_absorption(
            chain,
            i,
            target,
            t_max,
            Tolerances::default(),
            Settings::default(),
        )
    }

    #[test]
    fn positive_moran_n3_r2_matches_the_lean_values() {
        let c = moran_kernel(3, 2.0);
        let a = run(&c, 1, 4.0 / 7.0, 1e5).expect("certified");
        let b = run(&c, 2, 6.0 / 7.0, 1e5).expect("certified");
        assert!((a.p_fix - 4.0 / 7.0).abs() < 1e-8);
        assert!((b.p_fix - 6.0 / 7.0).abs() < 1e-8);
        assert!((a.p_fix + a.p_loss + a.interior - 1.0).abs() < 1e-9);
    }

    #[test]
    fn positive_neutral_wright_fisher_gives_i_over_n() {
        let c = wright_fisher_kernel(6);
        for i in 1..6 {
            let cert = run(&c, i, i as f64 / 6.0, 1e6).expect("certified");
            assert!((cert.p_fix - i as f64 / 6.0).abs() < 1e-8);
        }
    }

    #[test]
    fn negative_flipped_and_neutral_targets_are_refused_under_selection() {
        let c = moran_kernel(10, 1.5);
        assert!(matches!(
            run(&c, 3, flipped_fix(10, 1.5, 3), 1e6),
            Err(CertError::ValueMismatch { .. })
        ));
        assert!(matches!(
            run(&c, 3, 0.3, 1e6),
            Err(CertError::ValueMismatch { .. })
        ));
        assert!(run(&c, 3, moran_fix(10, 1.5, 3), 1e6).is_ok());
    }

    #[test]
    fn negative_identity_kernel_never_absorbs() {
        // Valid absorbing kernel in the Lean sense (every state absorbing), but P^n(1, N) = 0 for
        // all n: the variance hypothesis of the Lean theorem fails and the certificate refuses.
        let c = identity_kernel(2);
        assert!(matches!(
            run(&c, 1, 0.5, 1e4),
            Err(CertError::NotAbsorbed { .. })
        ));
    }

    #[test]
    fn negative_corrupted_kernel_is_refused() {
        let mut c = moran_kernel(4, 1.3);
        c.p[2][2] += 1e-3;
        assert!(matches!(
            run(&c, 2, 0.5, 1e4),
            Err(CertError::InvalidKernel(_))
        ));
        let mut d = moran_kernel(4, 1.3);
        d.p[0][0] = 0.5;
        d.p[0][1] = 0.5;
        assert!(matches!(
            run(&d, 2, 0.5, 1e4),
            Err(CertError::InvalidKernel(_))
        ));
    }

    #[test]
    fn nonnegativity_check_refuses_cancelling_entries_that_the_signed_sums_accept() {
        // Negative control: total mass 1 and interior mass 0, so the signed tests pass, but a
        // component is negative.
        let bad = [0.5, -0.25, 0.25, 0.5];
        let total: f64 = bad.iter().sum();
        let interior: f64 = bad[1..3].iter().sum();
        assert!((total - 1.0).abs() < 1e-15 && interior.abs() < 1e-15);
        assert!(matches!(
            check_nonnegative(&bad, 1e-12),
            Err(CertError::NegativeMass { state: 1, .. })
        ));
        // Positive control: a probability vector and one with round-off-level negativity pass.
        assert_eq!(check_nonnegative(&[0.25, 0.25, 0.0, 0.5], 1e-12), Ok(0.0));
        assert!(check_nonnegative(&[0.5, -1e-14, 0.5], 1e-12).is_ok());
    }

    #[test]
    fn accepted_runs_record_a_nonnegative_minimum_component() {
        let c = moran_kernel(10, 1.5);
        let cert = run(&c, 3, moran_fix(10, 1.5, 3), 1e6).expect("certified");
        assert!(cert.min_component >= -Tolerances::default().negative);
    }

    #[test]
    fn negative_horizon_too_short_is_refused() {
        let c = moran_kernel(10, 1.5);
        assert!(matches!(
            run(&c, 3, moran_fix(10, 1.5, 3), 4.0),
            Err(CertError::NotAbsorbed { .. })
        ));
    }
}
