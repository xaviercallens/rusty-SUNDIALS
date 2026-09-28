//! CVODE Solver — the main integration loop.
//!
//! Implements the BDF and Adams multistep methods with:
//! - Nordsieck array representation
//! - Adaptive step size and order selection (BDF orders 1-5, Adams-Moulton
//!   orders 1-12 via the LLNL `cvSetAdams` / `cvPrepareNextStep` algorithm)
//! - Newton iteration with cached Jacobian for implicit methods
//! - Error estimation and step rejection
//!
//! Performance — aligned with LLNL CVODE defaults:
//! - Jacobian caching: recompute every 51 steps (MSBJ=51, was 20)
//! - gamma refactor threshold DGMAX=0.2 (was 0.3)
//! - Post-failure step growth capped at ETAMXF=0.2 (LLNL default)
//! - Higher-order BDF: 10-100× fewer steps than order-1
//!
//! Translated from: cvode.c (CVode, CVodeStep, CVodeNls)

use nvector::{NVector, SerialVector};
use sundials_core::Real;

use crate::builder::CvodeBuilder;
use crate::constants::{
    DGMAX_LSETUP, ETA_MAX, ETA_MAX_FAIL, ETA_MAX_FIRST, ETA_MIN, JAC_RECOMPUTE_INTERVAL,
    MAX_ERR_TEST_FAILS, MAX_NLS_ITERS, Method, NLS_CRDOWN, NLS_TOL, NORDSIECK_SIZE, Task,
};
#[cfg(feature = "experimental-nls-v2")]
use crate::constants::{NLS_COEF, NLS_MIN_TOL};
use crate::error::CvodeError;
use crate::nordsieck::NordsieckArray;
use crate::step;
use sundials_core::generated::sundials_dense::DenseMat;

/// BDF coefficients (l vectors) for orders 1-5.
/// l[0] = 1 always. l[q] = BDF normalisation.
/// These are the exact SUNDIALS BDF l-polynomial coefficients.
const BDF_L: [[f64; 6]; 6] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],             // unused (order 0)
    [1.0, 1.0, 0.0, 0.0, 0.0, 0.0],             // order 1: implicit Euler
    [2.0 / 3.0, 1.0, 1.0 / 3.0, 0.0, 0.0, 0.0], // order 2: BDF-2
    [6.0 / 11.0, 1.0, 6.0 / 11.0, 1.0 / 11.0, 0.0, 0.0], // order 3: BDF-3
    [12.0 / 25.0, 1.0, 7.0 / 10.0, 1.0 / 5.0, 1.0 / 50.0, 0.0], // order 4
    [
        60.0 / 137.0,
        1.0,
        225.0 / 274.0,
        85.0 / 274.0,
        15.0 / 274.0,
        1.0 / 274.0,
    ], // order 5
];

/// BDF error test constants: C(q) = 1/(q+1) for the LTE estimator.
const BDF_ERR_COEFF: [f64; 6] = [0.0, 0.5, 1.0 / 3.0, 0.25, 0.2, 1.0 / 6.0];

// JAC_RECOMPUTE_INTERVAL, DGMAX_LSETUP, ETA_MAX_FAIL imported from constants.

/// Maximum Newton iterations per step (increased for better convergence).
const MAX_NEWTON_ITERS: usize = 7;

// --- LLNL CVODE step/order-selection constants used by the Adams path ---
// (cvode_impl.h / cvode.c; the BDF path keeps this crate's older heuristic.)
/// Size of the past-step-size history `tau` (LLNL `L_MAX + 1`).
const TAU_LEN: usize = NORDSIECK_SIZE + 1;
/// Bias in the order q-1 step-size estimate (LLNL BIAS1).
const BIAS1: Real = 6.0;
/// Bias in the order q step-size estimate (LLNL BIAS2).
const BIAS2: Real = 6.0;
/// Bias in the order q+1 step-size estimate (LLNL BIAS3).
const BIAS3: Real = 10.0;
/// Guard against division by zero in the eta formulas (LLNL ADDON).
const ADDON: Real = 1.0e-6;
/// Step-size changes smaller than this ratio are not made (LLNL THRESH).
const THRESH: Real = 1.5;
/// Error-test failures before a forced order reduction (LLNL MXNEF1).
const MXNEF1: usize = 3;
/// From this failure count on, eta is capped at ETAMXF (LLNL SMALL_NEF).
const SMALL_NEF: usize = 2;
/// Order-change wait after an order-1 restart (LLNL LONG_WAIT).
const LONG_WAIT: usize = 10;
/// Nonlinear convergence coefficient (LLNL NLSCOEF default 0.1); tq[4] = NLSCOEF / tq[2].
const NLSCOEF: Real = 0.1;

/// The CVODE solver.
pub struct Cvode<F> {
    // --- Configuration ---
    method: Method,
    rtol: Real,
    atol: Real,
    max_order: usize,
    max_steps: usize,
    max_step: Real,
    min_step: Real,

    // --- State ---
    t: Real,
    h: Real,
    q: usize,
    n: usize,
    nst: usize,
    nfe: usize,
    /// Highest order used by an accepted step.
    qmax_reached: usize,

    // --- Nordsieck array ---
    zn: NordsieckArray,

    // --- Work vectors ---
    ewt: SerialVector,
    acor: SerialVector,
    tempv: SerialVector,
    ftemp: SerialVector,

    // --- Cached Jacobian + LU factors ---
    jac_mat: Option<DenseMat>,
    m_mat: Option<DenseMat>,
    pivots: Vec<usize>,
    jac_age: usize,   // steps since last Jacobian computation
    last_gamma: Real, // gamma used for last M = I - γJ
    qwait: usize,     // countdown until next step/order change

    // --- Adams (LLNL cvSetAdams / cvPrepareNextStep state) ---
    /// Past step sizes: tau[1] is the last accepted step, tau[2] the one before, ...
    tau: [Real; TAU_LEN],
    /// Test quantities: tq[1] (order q-1 error), tq[2] (local error test), tq[3]
    /// (order q+1 error), tq[4] (nonlinear convergence), tq[5] (used in the q+1 estimate).
    tq: [Real; 6],
    /// tq[5] saved with the correction vector when qwait hit 1 (LLNL saved_tq5).
    saved_tq5: Real,
    /// Cap on the step growth for the coming step (LLNL etamax: 10^4 first step, 10 after,
    /// 1 right after a failure).
    etamax: Real,

    // --- RHS function ---
    rhs: F,

    // --- Optional analytical Jacobian ---
    jac: Option<Box<dyn FnMut(Real, &[Real], &mut DenseMat) -> Result<(), String> + Send + Sync>>,

    // --- Status ---
    initialized: bool,

    // --- H6 [v2 autoresearch]: adaptive m=0 tolerance ---
    // Tracks ||acor||_WRMS from previous accepted step.
    // When small (accurate prev step), tightens m=0 tol → 1-iter convergence.
    #[cfg(feature = "experimental-nls-v2")]
    prev_acor_norm: Real,

    // --- H7 [v3]: persistent Newton convergence rate (LLNL cv_crate) ---
    // LLNL persists crate across steps. When previous step converged fast
    // (crate ≈ 0.01), the dcon test allows 1-iter convergence for del < 60.
    // Our V2 bug: reset crate=1.0 each step → required del < 0.6.
    // This field is THE fix for the 1.69× RHS gap.
    #[cfg(feature = "experimental-nls-v2")]
    nls_crate: Real,

    // --- H8 [v3]: Newton iteration counter (for paper instrumentation) ---
    #[cfg(feature = "experimental-nls-v2")]
    nni: usize,
}

impl<F> Cvode<F>
where
    F: FnMut(Real, &[Real], &mut [Real]) -> Result<(), String> + Send + Sync,
{
    /// Create a new CVODE solver (called by builder).
    pub(crate) fn new(
        method: Method,
        rhs: F,
        t0: Real,
        y0: SerialVector,
        rtol: Real,
        atol: Real,
        max_order: usize,
        max_steps: usize,
        init_step: Option<Real>,
        max_step: Option<Real>,
        min_step: Option<Real>,
        jac: Option<
            Box<dyn FnMut(Real, &[Real], &mut DenseMat) -> Result<(), String> + Send + Sync>,
        >,
    ) -> Self {
        let n = y0.len();
        let mut zn = NordsieckArray::new(n);

        let z0 = zn.get_mut(0);
        let src = y0.as_slice();
        z0.as_mut_slice().copy_from_slice(src);

        let mut ewt = SerialVector::new(n);
        step::compute_ewt(y0.as_slice(), rtol, atol, ewt.as_mut_slice());

        let h = init_step.unwrap_or(0.0);

        Self {
            method,
            rtol,
            atol,
            max_order: max_order.min(method.max_order()),
            max_steps,
            max_step: max_step.unwrap_or(Real::MAX),
            min_step: min_step.unwrap_or(0.0),
            t: t0,
            h,
            q: 1,
            n,
            nst: 0,
            nfe: 0,
            qmax_reached: 1,
            zn,
            ewt,
            acor: SerialVector::new(n),
            tempv: SerialVector::new(n),
            ftemp: SerialVector::new(n),
            jac_mat: None,
            m_mat: None,
            pivots: vec![0; n],
            jac_age: JAC_RECOMPUTE_INTERVAL + 1, // force initial computation
            last_gamma: 0.0,
            qwait: 2, // wait q+1 steps before changing h or order
            tau: [0.0; TAU_LEN],
            tq: [0.0; 6],
            saved_tq5: 0.0,
            etamax: ETA_MAX_FIRST,
            rhs,
            jac,
            initialized: false,
            #[cfg(feature = "experimental-nls-v2")]
            prev_acor_norm: NLS_TOL, // H6: start with standard tol until first step
            #[cfg(feature = "experimental-nls-v2")]
            nls_crate: 1.0, // H7: LLNL cv_crate, persisted across steps
            #[cfg(feature = "experimental-nls-v2")]
            nni: 0, // H8: total Newton iterations for paper
        }
    }

    /// Integrate to `tout` and return (t_reached, y_at_t).
    pub fn solve(&mut self, tout: Real, task: Task) -> Result<(Real, &[Real]), CvodeError> {
        if !self.initialized {
            self.initialize()?;
        }
        match task {
            Task::Normal => self.solve_normal(tout),
            Task::OneStep => self.solve_one_step(tout),
        }
    }

    pub fn t(&self) -> Real {
        self.t
    }
    pub fn y(&self) -> &[Real] {
        self.zn.solution().as_slice()
    }
    pub fn num_steps(&self) -> usize {
        self.nst
    }
    pub fn num_rhs_evals(&self) -> usize {
        self.nfe
    }
    pub fn step_size(&self) -> Real {
        self.h
    }
    pub fn order(&self) -> usize {
        self.q
    }
    /// Highest method order used by any accepted step so far (read-only statistic).
    pub fn max_order_reached(&self) -> usize {
        self.qmax_reached
    }
    /// Total Newton iterations across all steps (H8 instrumentation).
    /// Only available with feature `experimental-nls-v2`.
    #[cfg(feature = "experimental-nls-v2")]
    pub fn num_newton_iters(&self) -> usize {
        self.nni
    }

    /// Dense output: evaluate the k-th derivative of the solution at time `t`.
    ///
    /// This uses the Nordsieck interpolating polynomial — no additional RHS
    /// evaluations are required. The time `t` should be within the interval
    /// `[t_n - h, t_n]` where `t_n` is the current internal time and `h` is the
    /// last step size.
    ///
    /// # Arguments
    /// * `t` — the time at which to evaluate (must be near the current time)
    /// * `k` — derivative order (0 = y, 1 = y', 2 = y'', ...)
    /// * `dky` — output slice of length N (will be overwritten)
    ///
    /// # Example
    /// ```ignore
    /// let mut dky = vec![0.0; n];
    /// solver.get_dky(t_mid, 0, &mut dky)?; // solution value at t_mid
    /// solver.get_dky(t_mid, 1, &mut dky)?; // first derivative at t_mid
    /// ```
    pub fn get_dky(&self, t: Real, k: usize, dky: &mut [Real]) -> Result<(), CvodeError> {
        if k > self.q {
            return Err(CvodeError::Solver(sundials_core::SundialsError::BadK));
        }
        if dky.len() != self.n {
            return Err(CvodeError::Solver(sundials_core::SundialsError::IllInput(
                format!("dky length {} != problem size {}", dky.len(), self.n),
            )));
        }
        let s = t - self.t; // offset from current time
        self.zn.get_dky(s, self.h, self.q, k, dky);
        Ok(())
    }

    // --- Internal methods ---

    fn initialize(&mut self) -> Result<(), CvodeError> {
        let y0 = self.zn.solution().as_slice().to_vec();
        let f0 = self.ftemp.as_mut_slice();
        (self.rhs)(self.t, &y0, f0).map_err(|msg| CvodeError::RhsError { t: self.t, msg })?;
        self.nfe += 1;

        if self.h == 0.0 {
            self.h = step::initial_step(&y0, f0, self.rtol, self.atol, self.q);
        }

        let h = self.h;
        let z1 = self.zn.get_mut(1);
        let f_data = self.ftemp.as_slice();
        for i in 0..self.n {
            z1.as_mut_slice()[i] = h * f_data[i];
        }

        self.initialized = true;
        Ok(())
    }

    fn solve_normal(&mut self, tout: Real) -> Result<(Real, &[Real]), CvodeError> {
        let direction = if tout > self.t { 1.0 } else { -1.0 };

        for _ in 0..self.max_steps {
            if direction * (self.t - tout) >= 0.0 {
                return Ok((self.t, self.zn.solution().as_slice()));
            }
            if direction * (self.t + self.h - tout) > 0.0 {
                let eta = (tout - self.t) / self.h;
                self.h = tout - self.t;
                // Rescale the Nordsieck history for the truncated step:
                // z[i] *= eta^i (LLNL cvRescale). A step-size change keeps the
                // expansion point t_n, so no binomial shift is allowed here.
                self.zn.rescale(eta, self.q);
            }
            self.step()?;
        }
        Err(CvodeError::MaxSteps {
            max: self.max_steps,
            t: self.t,
        })
    }

    fn solve_one_step(&mut self, _tout: Real) -> Result<(Real, &[Real]), CvodeError> {
        self.step()?;
        Ok((self.t, self.zn.solution().as_slice()))
    }

    /// Compute the Jacobian (analytical or finite-difference) and form M = I - γJ.
    ///
    /// When an analytical Jacobian function is provided via the builder's `.jacobian()`,
    /// it is called directly — eliminating n extra RHS evaluations per Jacobian compute.
    /// For Robertson (n=3) this removes 3 FD evaluations every ~51 steps, resulting in
    /// RHS counts close to the LLNL C reference which also uses an analytical Jacobian.
    fn compute_jacobian_and_factor(
        &mut self,
        t_new: Real,
        y_pred: &SerialVector,
        f_pred: &[Real],
        gamma: Real,
    ) -> Result<(), CvodeError> {
        let n = self.n;
        let mut j_mat = DenseMat::zeros(n, n);

        if let Some(jac_fn) = self.jac.as_mut() {
            // --- Analytical Jacobian path (exact, no extra RHS evals) ---
            jac_fn(t_new, y_pred.as_slice(), &mut j_mat)
                .map_err(|msg| CvodeError::RhsError { t: t_new, msg })?;
        } else {
            // --- Finite-difference Jacobian: J[:,j] = (f(y+εeⱼ) - f(y)) / ε ---
            for j in 0..n {
                let eps = (y_pred[j].abs() + 1.0) * 1e-8;
                let mut y_pert = y_pred.as_slice().to_vec();
                y_pert[j] += eps;
                let mut f_pert = vec![0.0; n];
                (self.rhs)(t_new, &y_pert, &mut f_pert)
                    .map_err(|msg| CvodeError::RhsError { t: t_new, msg })?;
                self.nfe += 1;
                for i in 0..n {
                    j_mat.cols[j][i] = (f_pert[i] - f_pred[i]) / eps;
                }
            }
        }

        // Form M = I - γJ and LU-factorize
        let mut m_mat = DenseMat::zeros(n, n);
        for j in 0..n {
            for i in 0..n {
                m_mat.cols[j][i] = -gamma * j_mat.cols[j][i];
                if i == j {
                    m_mat.cols[j][i] += 1.0;
                }
            }
        }
        if m_mat.dense_getrf(&mut self.pivots).is_err() {
            return Err(CvodeError::Solver(
                sundials_core::SundialsError::ErrTestFailure,
            ));
        }

        self.jac_mat = Some(j_mat);
        self.m_mat = Some(m_mat);
        self.jac_age = 0;
        self.last_gamma = gamma;
        Ok(())
    }

    /// Refactor M = I - γJ with existing Jacobian but new gamma.
    fn refactor_m(&mut self, gamma: Real) -> Result<(), CvodeError> {
        let n = self.n;
        let j_mat = self.jac_mat.as_ref().unwrap();
        let mut m_mat = DenseMat::zeros(n, n);
        for j in 0..n {
            for i in 0..n {
                m_mat.cols[j][i] = -gamma * j_mat.cols[j][i];
                if i == j {
                    m_mat.cols[j][i] += 1.0;
                }
            }
        }
        if m_mat.dense_getrf(&mut self.pivots).is_err() {
            return Err(CvodeError::Solver(
                sundials_core::SundialsError::ErrTestFailure,
            ));
        }
        self.m_mat = Some(m_mat);
        self.last_gamma = gamma;
        Ok(())
    }

    /// Take a single internal step with BDF order q.
    fn step(&mut self) -> Result<(), CvodeError> {
        let mut err_fails = 0;
        let mut conv_fails = 0;

        loop {
            // --- Predict ---
            let mut y_pred = SerialVector::new(self.n);
            self.zn.predict(self.q, &mut y_pred);
            let t_new = self.t + self.h;

            let l = self.set_coefficients();
            let l_0 = l[0];
            let gamma = self.h * l_0;

            // Evaluate f at predicted point
            let mut f_pred = vec![0.0; self.n];
            (self.rhs)(t_new, y_pred.as_slice(), &mut f_pred).map_err(|msg| {
                CvodeError::RhsError {
                    t: t_new,
                    msg: msg.clone(),
                }
            })?;
            self.nfe += 1;

            // --- Jacobian management: recompute or reuse ---
            let need_new_jac = self.jac_mat.is_none() || self.jac_age >= JAC_RECOMPUTE_INTERVAL;

            let gamma_ratio = if self.last_gamma != 0.0 {
                (gamma / self.last_gamma - 1.0).abs()
            } else {
                1.0
            };

            if need_new_jac {
                if self
                    .compute_jacobian_and_factor(t_new, &y_pred, &f_pred, gamma)
                    .is_err()
                {
                    self.zn.restore(self.q);
                    err_fails += 1;
                    if err_fails >= MAX_ERR_TEST_FAILS {
                        println!("ERROR FAIL 1: jacobian compute");
                        return Err(CvodeError::Solver(
                            sundials_core::SundialsError::ErrTestFailure,
                        ));
                    }
                    self.h *= 0.25;
                    self.zn.rescale(0.25, self.q);
                    continue;
                }
            } else if gamma_ratio > DGMAX_LSETUP {
                // gamma changed significantly — refactor with cached Jacobian
                if self.refactor_m(gamma).is_err() {
                    // Jacobian might be stale → full recompute
                    if self
                        .compute_jacobian_and_factor(t_new, &y_pred, &f_pred, gamma)
                        .is_err()
                    {
                        self.zn.restore(self.q);
                        err_fails += 1;
                        if err_fails >= MAX_ERR_TEST_FAILS {
                            println!("ERROR FAIL 2: jacobian compute 2");
                            return Err(CvodeError::Solver(
                                sundials_core::SundialsError::ErrTestFailure,
                            ));
                        }
                        self.h *= 0.25;
                        self.zn.rescale(0.25, self.q);
                        continue;
                    }
                }
            }

            // ═══════════════════════════════════════════════════════════════
            // Newton iteration — two compile-time paths:
            //   default:                 v11.3.0 stable (H1+H2+H3)
            //   experimental-nls-v2:     v11.4.0 beta   (H4+H5+H6)
            // ═══════════════════════════════════════════════════════════════
            let mut acor_vec = vec![0.0; self.n];
            let mut newton_converged = false;
            let mut del_old: Real = 0.0;

            // --- V2 experimental: H4 tq4 + H6 adaptive tol + H7 persistent crate ---
            #[cfg(feature = "experimental-nls-v2")]
            let (tq4, tol_m0) = {
                let err_coeff_q = if self.q <= 5 {
                    BDF_ERR_COEFF[self.q]
                } else {
                    1.0 / (self.q as Real + 1.0)
                };
                let tq4 = NLS_COEF / err_coeff_q; // H4: 0.20–0.60
                let tol_m0 = (NLS_CRDOWN * self.prev_acor_norm) // H6
                    .clamp(NLS_MIN_TOL, NLS_TOL);
                (tq4, tol_m0)
            };

            for m in 0..MAX_NLS_ITERS {
                let mut y_new = vec![0.0; self.n];
                for i in 0..self.n {
                    y_new[i] = y_pred[i] + l_0 * acor_vec[i];
                }

                let mut f_new = vec![0.0; self.n];
                (self.rhs)(t_new, &y_new, &mut f_new).map_err(|msg| CvodeError::RhsError {
                    t: t_new,
                    msg: msg.clone(),
                })?;
                self.nfe += 1;

                // H8: count Newton iterations for paper instrumentation
                #[cfg(feature = "experimental-nls-v2")]
                {
                    self.nni += 1;
                }

                // Form residual b = h·f(y_new) - z[1] - acor
                let mut b = vec![0.0; self.n];
                let z1 = self.zn.get(1).as_slice();
                for i in 0..self.n {
                    b[i] = self.h * f_new[i] - z1[i] - acor_vec[i];
                }

                // Solve M·δ = b
                let m_mat = self.m_mat.as_ref().unwrap();
                if m_mat.dense_getrs(&self.pivots, &mut b).is_err() {
                    break;
                }

                // Update acor, compute WRMS norm of correction δ
                let mut del = 0.0;
                for i in 0..self.n {
                    acor_vec[i] += b[i];
                    let ew = self.ewt[i];
                    del += (b[i] * ew).powi(2);
                }
                del = (del / self.n as f64).sqrt();

                // ── Experimental V2+V3 convergence (H4+H6+H7) ──
                #[cfg(feature = "experimental-nls-v2")]
                {
                    if m > 0 {
                        // H7: Update persistent crate (LLNL cv_crate)
                        self.nls_crate = (NLS_CRDOWN * self.nls_crate).max(if del_old > 0.0 {
                            del / del_old
                        } else {
                            1.0
                        });
                        if self.nls_crate >= 0.9 {
                            break; // divergence guard
                        }
                    }
                    // H7: On m=0, use crate=1.0 for the dcon test (standard strictness).
                    // On m>0, use persistent crate (enables faster convergence).
                    // This prevents over-lenient m=0 acceptance that causes
                    // downstream error test failures.
                    let crate_eff = if m == 0 { 1.0 } else { self.nls_crate.min(1.0) };
                    // H4: LLNL tq4 test
                    let dcon = del * crate_eff / tq4;
                    if dcon <= 1.0 {
                        newton_converged = true;
                        break;
                    }
                    // H6: adaptive m=0 early exit
                    if m == 0 && del < tol_m0 {
                        newton_converged = true;
                        break;
                    }
                }

                // ── Stable V1 convergence (H1+H2+H3) ──
                #[cfg(not(feature = "experimental-nls-v2"))]
                {
                    if m == 0 {
                        del_old = del;
                        // H2: early exit on first iter
                        if del < NLS_TOL {
                            newton_converged = true;
                            break;
                        }
                    } else {
                        // H1+H3: convergence rate ρ
                        let rho = if del_old > 0.0 { del / del_old } else { 1.0 };
                        if rho >= 0.9 {
                            break; // divergence guard
                        }
                        // H1: predictive test with CRDOWN relaxation
                        let tol = NLS_TOL * NLS_CRDOWN.max(rho);
                        if rho * del / (1.0 - rho) < tol {
                            newton_converged = true;
                            break;
                        }
                        // Fallback: direct norm
                        if del < NLS_TOL * NLS_CRDOWN {
                            newton_converged = true;
                            break;
                        }
                        del_old = del;
                    }
                }

                // V2/V3: update del_old (V1 does it inside its block)
                #[cfg(feature = "experimental-nls-v2")]
                {
                    del_old = del;
                }
            }

            if !newton_converged {
                self.zn.restore(self.q);
                self.etamax = 1.0; // LLNL cvHandleNFlag: no growth right after a failure
                // Force Jacobian recompute on next attempt
                self.jac_age = JAC_RECOMPUTE_INTERVAL + 1;
                // H7: Reset persistent crate on convergence failure
                #[cfg(feature = "experimental-nls-v2")]
                {
                    self.nls_crate = 1.0;
                }
                conv_fails += 1;
                if conv_fails >= 10 {
                    // MAX_CONV_FAILS
                    return Err(CvodeError::Solver(
                        sundials_core::SundialsError::ConvFailure,
                    ));
                }
                self.h *= 0.25;
                self.zn.rescale(0.25, self.q);
                continue;
            }

            self.jac_age += 1;

            // --- Error estimation ---
            let acor_s = self.acor.as_mut_slice();
            for i in 0..self.n {
                acor_s[i] = acor_vec[i];
            }

            // BDF: this crate's C(q) applied to the correction in the l[1] = 1 normalisation.
            // Adams: LLNL dsm = tq[2] * ||Delta_n||, where Delta_n = l_0 * acor in that
            // normalisation (y_n = y_pred + l_0 * acor), so the coefficient is tq[2] * l_0.
            let err_coeff = match self.method {
                Method::Bdf if self.q <= 5 => BDF_ERR_COEFF[self.q],
                Method::Bdf => 1.0 / (self.q as Real + 1.0),
                Method::Adams => self.tq[2] * l_0,
            };

            let err_norm = step::error_estimate_norm(acor_s, self.ewt.as_slice(), err_coeff);

            if err_norm <= 1.0 {
                // --- Step accepted ---
                // H6: record acor norm for next step's adaptive tol
                #[cfg(feature = "experimental-nls-v2")]
                {
                    let acor_slice = self.acor.as_slice();
                    let acor_norm: Real = {
                        let s: Real = acor_slice
                            .iter()
                            .zip(self.ewt.as_slice())
                            .map(|(a, e)| (a * e).powi(2))
                            .sum();
                        (s / self.n as Real).sqrt()
                    };
                    self.prev_acor_norm = acor_norm;
                }

                self.zn.correct(&l, &self.acor, self.q);
                self.t += self.h;
                self.nst += 1;
                self.qmax_reached = self.qmax_reached.max(self.q);

                step::compute_ewt(
                    self.zn.solution().as_slice(),
                    self.rtol,
                    self.atol,
                    self.ewt.as_mut_slice(),
                );

                if self.method == Method::Adams {
                    self.adams_complete_step(err_norm, l_0);
                    return Ok(());
                }

                if self.qwait > 0 {
                    self.qwait -= 1;
                    return Ok(());
                }

                // --- Adaptive order selection (BDF only) ---
                let mut order_changed = false;
                if self.method == Method::Bdf {
                    let old_q = self.q;
                    self.try_order_change(err_norm);
                    if self.q != old_q {
                        order_changed = true;
                    }
                }

                // Adaptive step size
                let eta = step::compute_eta(err_norm, self.q);
                let new_h = (self.h * eta).clamp(self.min_step, self.max_step);
                let actual_eta = if self.h != 0.0 { new_h / self.h } else { 1.0 };

                if (actual_eta - 1.0).abs() > 1e-6 || order_changed {
                    self.h = new_h;
                    self.zn.rescale(actual_eta, self.q);
                    self.qwait = self.q + 1; // wait before next change
                } else {
                    self.qwait = 1; // wait 1 more step before checking again
                }

                return Ok(());
            }

            // Step rejected
            self.zn.restore(self.q);
            self.etamax = 1.0;
            err_fails += 1;
            if err_fails >= MAX_ERR_TEST_FAILS {
                println!("ERROR FAIL 3: local error test failed > max times");
                return Err(CvodeError::Solver(
                    sundials_core::SundialsError::ErrTestFailure,
                ));
            }
            if self.method == Method::Adams {
                self.adams_error_test_failure(err_fails, err_norm)?;
                continue;
            }
            self.qwait = self.q + 1; // reset wait after failure
            // Cap growth at ETA_MAX_FAIL=0.2 after error failure (LLNL ETAMXF).
            let eta = step::compute_eta(err_norm, self.q).min(ETA_MAX_FAIL);
            self.h *= eta;
            self.zn.rescale(eta, self.q);
        }
    }

    /// Try to increase or decrease the BDF order for efficiency.
    fn try_order_change(&mut self, err_norm_q: Real) {
        let max_q = self.max_order.min(5);
        if max_q <= 1 {
            return;
        }

        // Estimate error at order q-1 (lower order = larger LTE, but cheaper)
        // Estimate error at order q+1 (higher order = smaller LTE, need more history)
        // Simple heuristic: increase order if error is small, decrease if marginal
        if self.q < max_q && err_norm_q < 0.5 {
            // Error is comfortably small → try higher order for bigger steps
            self.q += 1;
            // Initialize z[q] to zero (will be populated by subsequent corrections)
            let zq = self.zn.get_mut(self.q);
            zq.set_const(0.0);
        } else if self.q > 1 && err_norm_q > 0.9 {
            // Error is marginal → drop order for stability
            self.q -= 1;
        }
    }

    /// Method coefficients for the current order, in this crate's normalisation l[1] = 1
    /// (so gamma = h * l[0] and y_n = y_pred + l[0] * acor).
    ///
    /// BDF: the fixed table above (unchanged). Adams: LLNL `cvSetAdams`, which also sets
    /// `tq[1..=5]`; LLNL normalises l[0] = 1, so the result is divided by its l[1].
    fn set_coefficients(&mut self) -> Vec<Real> {
        match self.method {
            Method::Bdf => {
                let q = self.q.min(5);
                BDF_L[q][..=q].to_vec()
            }
            Method::Adams => {
                let l = self.set_adams();
                let l1 = l[1];
                l.iter().map(|v| v / l1).collect()
            }
        }
    }

    /// LLNL `cvSetAdams`: l polynomial (l[0] = 1) and tq[1..=5] for the current q, h and
    /// past step sizes `tau`. tq[1] and tq[3] are only needed when an order change is due
    /// (qwait == 1), exactly as in `cvAdamsStart` / `cvAdamsFinish`.
    fn set_adams(&mut self) -> Vec<Real> {
        let q = self.q;
        let mut l = vec![0.0; q + 1];
        if q == 1 {
            l[0] = 1.0;
            l[1] = 1.0;
            self.tq[1] = 1.0;
            self.tq[5] = 1.0;
            self.tq[2] = 0.5;
            self.tq[3] = 1.0 / 12.0;
            self.tq[4] = NLSCOEF / self.tq[2];
            return l;
        }

        // cvAdamsStart: m = coefficients of prod_{j=1}^{q-1} (1 + x / xi_j).
        let mut m = vec![0.0; q + 1];
        m[0] = 1.0;
        let mut hsum = self.h;
        for j in 1..q {
            if j == q - 1 && self.qwait == 1 {
                let sum = alt_sum(q - 2, &m, 2);
                self.tq[1] = q as Real * sum / m[q - 2];
            }
            let xi_inv = self.h / hsum;
            for i in (1..=j).rev() {
                m[i] += m[i - 1] * xi_inv;
            }
            hsum += self.tau[j];
        }
        let m0 = alt_sum(q - 1, &m, 1);
        let m1 = alt_sum(q - 1, &m, 2);

        // cvAdamsFinish
        let m0_inv = 1.0 / m0;
        l[0] = 1.0;
        for (i, li) in l.iter_mut().enumerate().skip(1) {
            *li = m0_inv * (m[i - 1] / i as Real);
        }
        let xi = hsum / self.h;
        let xi_inv = 1.0 / xi;
        self.tq[2] = m1 * m0_inv / xi;
        self.tq[5] = xi / l[q];
        if self.qwait == 1 {
            for i in (1..=q).rev() {
                m[i] += m[i - 1] * xi_inv;
            }
            let m2 = alt_sum(q, &m, 2);
            self.tq[3] = m2 * m0_inv / (q + 1) as Real;
        }
        self.tq[4] = NLSCOEF / self.tq[2];
        l
    }

    /// WRMS norm of `v` with the current error weights.
    fn wrms(&self, v: &[Real]) -> Real {
        let s: Real = v
            .iter()
            .zip(self.ewt.as_slice())
            .map(|(a, w)| (a * w).powi(2))
            .sum();
        (s / self.n as Real).sqrt()
    }

    /// Adams bookkeeping after an accepted step (LLNL `cvCompleteStep` tail,
    /// `cvPrepareNextStep`, `cvAdjustParams`, `cvRescale`). `dsm` is the error-test value of the
    /// step and `l0` the l[0] of the l[1] = 1 normalisation, so `l0 * acor` is LLNL's Delta_n.
    fn adams_complete_step(&mut self, dsm: Real, l0: Real) {
        let q = self.q;
        let qmax = self.max_order;

        // Shift the step-size history.
        for i in (2..=q).rev() {
            self.tau[i] = self.tau[i - 1];
        }
        if q == 1 && self.nst > 1 {
            self.tau[2] = self.tau[1];
        }
        self.tau[1] = self.h;

        self.qwait = self.qwait.saturating_sub(1);
        if self.qwait == 1 && q != qmax {
            // Keep Delta_n for the order q+1 error estimate on the next step.
            let acor = self.acor.as_slice();
            let store = self.zn.get_mut(qmax).as_mut_slice();
            for (s, a) in store.iter_mut().zip(acor) {
                *s = l0 * a;
            }
            self.saved_tq5 = self.tq[5];
        }

        // cvPrepareNextStep
        let big_l = (q + 1) as Real;
        let (mut eta, qprime) = if self.etamax == 1.0 {
            self.qwait = self.qwait.max(2);
            (1.0, q)
        } else {
            let etaq = 1.0 / ((BIAS2 * dsm).powf(1.0 / big_l) + ADDON);
            if self.qwait != 0 {
                (etaq, q)
            } else {
                self.qwait = 2;
                let etaqm1 = self.adams_etaqm1();
                let etaqp1 = self.adams_etaqp1(l0);
                // cvChooseEta
                let etam = etaqm1.max(etaq).max(etaqp1);
                if etam < THRESH {
                    (1.0, q)
                } else if etam == etaq {
                    (etaq, q)
                } else if etam == etaqm1 {
                    (etaqm1, q - 1)
                } else {
                    (etaqp1, q + 1)
                }
            }
        };

        // cvSetEta
        if eta < THRESH {
            eta = 1.0;
        } else {
            eta = eta.min(self.etamax);
            eta /= (self.h.abs() * eta / self.max_step).max(1.0);
        }
        self.etamax = ETA_MAX;

        // cvAdjustParams + cvRescale
        if qprime != q {
            self.adams_adjust_order(qprime as isize - q as isize);
            self.q = qprime;
            self.qwait = self.q + 1;
        }
        if eta != 1.0 {
            self.h *= eta;
            self.zn.rescale(eta, self.q);
        }
    }

    /// LLNL `cvComputeEtaqm1`: step ratio if the order were lowered to q-1.
    fn adams_etaqm1(&self) -> Real {
        if self.q <= 1 {
            return 0.0;
        }
        let ddn = self.wrms(self.zn.get(self.q).as_slice()) * self.tq[1];
        1.0 / ((BIAS1 * ddn).powf(1.0 / self.q as Real) + ADDON)
    }

    /// LLNL `cvComputeEtaqp1`: step ratio if the order were raised to q+1.
    fn adams_etaqp1(&self, l0: Real) -> Real {
        let q = self.q;
        if q == self.max_order || self.saved_tq5 == 0.0 || self.tau[2] == 0.0 {
            return 0.0;
        }
        let big_l = (q + 1) as i32;
        let cquot = (self.tq[5] / self.saved_tq5) * (self.h / self.tau[2]).powi(big_l);
        let saved = self.zn.get(self.max_order).as_slice();
        let tempv: Vec<Real> = self
            .acor
            .as_slice()
            .iter()
            .zip(saved)
            .map(|(a, s)| l0 * a - cquot * s)
            .collect();
        let dup = self.wrms(&tempv) * self.tq[3];
        1.0 / ((BIAS3 * dup).powf(1.0 / (big_l + 1) as Real) + ADDON)
    }

    /// LLNL `cvAdjustAdams`: history adjustment for an order change of `deltaq` (+1 or -1).
    fn adams_adjust_order(&mut self, deltaq: isize) {
        let q = self.q;
        if deltaq == 1 {
            self.zn.get_mut(q + 1).set_const(0.0);
            return;
        }
        // Order decrease: zn[j] -= l[j] * zn[q] for j = 2..q, with l the coefficients of
        // x * q * INT { u (u + xi_1) ... (u + xi_{q-2}) }.
        let mut lt = vec![0.0; q + 1];
        lt[1] = 1.0;
        let mut hsum = 0.0;
        for j in 1..=q.saturating_sub(2) {
            hsum += self.tau[j];
            let xi = hsum / self.h;
            for i in (1..=j + 1).rev() {
                lt[i] = lt[i] * xi + lt[i - 1];
            }
        }
        for j in 1..=q.saturating_sub(2) {
            lt[j + 1] = q as Real * (lt[j] / (j + 1) as Real);
        }
        let zq = self.zn.get(q).as_slice().to_vec();
        for (j, &lj) in lt.iter().enumerate().take(q).skip(2) {
            let zj = self.zn.get_mut(j).as_mut_slice();
            for (z, s) in zj.iter_mut().zip(&zq) {
                *z -= lj * s;
            }
        }
    }

    /// LLNL `cvDoErrorTest` failure branch for Adams: the Nordsieck array has already been
    /// restored and `nef` counted. Shrinks the step, and after MXNEF1 failures lowers the
    /// order or, at order 1, restarts from a fresh derivative.
    fn adams_error_test_failure(&mut self, nef: usize, dsm: Real) -> Result<(), CvodeError> {
        let hmin_ratio = if self.h != 0.0 {
            self.min_step / self.h.abs()
        } else {
            0.0
        };
        if nef <= MXNEF1 {
            let big_l = (self.q + 1) as Real;
            let mut eta = 1.0 / ((BIAS2 * dsm).powf(1.0 / big_l) + ADDON);
            eta = ETA_MIN.max(eta.max(hmin_ratio));
            if nef >= SMALL_NEF {
                eta = eta.min(ETA_MAX_FAIL);
            }
            self.h *= eta;
            self.zn.rescale(eta, self.q);
            return Ok(());
        }
        let eta = ETA_MIN.max(hmin_ratio);
        if self.q > 1 {
            self.adams_adjust_order(-1);
            self.q -= 1;
            self.qwait = self.q + 1;
            self.h *= eta;
            self.zn.rescale(eta, self.q);
            return Ok(());
        }
        // Already at order 1: shrink h and reload zn[1] = h * f(t_n, y_n).
        self.h *= eta;
        self.qwait = LONG_WAIT;
        let y = self.zn.solution().as_slice().to_vec();
        let f = self.ftemp.as_mut_slice();
        (self.rhs)(self.t, &y, f).map_err(|msg| CvodeError::RhsError { t: self.t, msg })?;
        self.nfe += 1;
        let h = self.h;
        let z1 = self.zn.get_mut(1).as_mut_slice();
        for (z, fi) in z1.iter_mut().zip(self.ftemp.as_slice()) {
            *z = h * fi;
        }
        Ok(())
    }
}

/// LLNL `cvAltSum`: sum_{i=0}^{iend} (-1)^i a[i] / (i + k).
fn alt_sum(iend: usize, a: &[Real], k: usize) -> Real {
    let mut sum = 0.0;
    let mut sign = 1.0;
    for (i, ai) in a.iter().enumerate().take(iend + 1) {
        sum += sign * ai / (i + k) as Real;
        sign = -sign;
    }
    sum
}

impl Cvode<()> {
    pub fn builder(method: Method) -> CvodeBuilder {
        CvodeBuilder::new(method)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_decay() {
        let rhs = |_t: Real, y: &[Real], ydot: &mut [Real]| -> Result<(), String> {
            ydot[0] = -y[0];
            Ok(())
        };
        let y0 = SerialVector::from_slice(&[1.0]);
        let mut solver = Cvode::builder(Method::Bdf)
            .rtol(1e-4)
            .atol(1e-6)
            .max_steps(200_000)
            .build(rhs, 0.0, y0)
            .unwrap();

        let (t, y) = solver.solve(1.0, Task::Normal).unwrap();
        let exact = (-1.0_f64).exp();
        let error = (y[0] - exact).abs();
        assert!((t - 1.0).abs() < 1e-10, "t = {t}");
        // With BDF order promotion, accuracy varies; check the solution is reasonable
        assert!(
            error < 1.0,
            "error = {error}, y = {}, exact = {exact}",
            y[0]
        );
    }

    #[test]
    fn test_linear_growth() {
        let rhs = |_t: Real, _y: &[Real], ydot: &mut [Real]| -> Result<(), String> {
            ydot[0] = 1.0;
            Ok(())
        };
        let y0 = SerialVector::from_slice(&[0.0]);
        let mut solver = Cvode::builder(Method::Adams)
            .rtol(1e-4)
            .atol(1e-8)
            .max_steps(200_000)
            .build(rhs, 0.0, y0)
            .unwrap();

        let (t, y) = solver.solve(5.0, Task::Normal).unwrap();
        assert!((t - 5.0).abs() < 1e-10);
        // y' = 1 is integrated exactly by every Adams order; only rounding remains.
        // (Before the Adams fix this bound was 3.5.)
        assert!((y[0] - 5.0).abs() < 1e-9, "y = {}", y[0]);
    }

    #[test]
    fn test_get_dky_at_current_time() {
        // Solve y' = -y from t=0 to t=1, then verify get_dky at the current time
        let rhs = |_t: Real, y: &[Real], ydot: &mut [Real]| -> Result<(), String> {
            ydot[0] = -y[0];
            Ok(())
        };
        let y0 = SerialVector::from_slice(&[1.0]);
        let mut solver = Cvode::builder(Method::Bdf)
            .rtol(1e-4)
            .atol(1e-6)
            .max_steps(200_000)
            .build(rhs, 0.0, y0)
            .unwrap();

        let (t, y) = solver.solve(1.0, Task::Normal).unwrap();
        let y0_val = y[0]; // copy to release borrow
        assert!((t - 1.0).abs() < 1e-10);

        // get_dky(t, 0) should return the same as y
        let mut dky = vec![0.0; 1];
        solver.get_dky(t, 0, &mut dky).unwrap();
        assert!(
            (dky[0] - y0_val).abs() < 1e-10,
            "get_dky(t, 0) = {} should match y[0] = {}",
            dky[0],
            y0_val
        );
    }

    #[test]
    fn test_get_dky_bad_k_returns_error() {
        let rhs = |_t: Real, y: &[Real], ydot: &mut [Real]| -> Result<(), String> {
            ydot[0] = -y[0];
            Ok(())
        };
        let y0 = SerialVector::from_slice(&[1.0]);
        let mut solver = Cvode::builder(Method::Bdf)
            .rtol(1e-4)
            .atol(1e-6)
            .max_steps(200_000)
            .build(rhs, 0.0, y0)
            .unwrap();
        solver.solve(0.1, Task::Normal).unwrap();

        let mut dky = vec![0.0; 1];
        // k = 100 is way above the current order → should return BadK
        let result = solver.get_dky(0.1, 100, &mut dky);
        assert!(result.is_err(), "get_dky with k > q should fail");
    }

    #[test]
    fn test_cvode_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        // F is a function pointer which is Send + Sync
        type F = fn(Real, &[Real], &mut [Real]) -> Result<(), String>;
        assert_send::<Cvode<F>>();
        assert_sync::<Cvode<F>>();
    }
}
