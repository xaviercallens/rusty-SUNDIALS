//! Step size control — adaptive step selection with error estimation.
//!
//! Implements the error-based step size controller from CVODE:
//! - Error estimation via local truncation error
//! - Step size selection: h_new = h * eta where eta = safety * (1/err)^(1/(q+1))
//! - Order selection for BDF and Adams methods
//!
//! Translated from: cvode.c (CVodeStep, CVodeSetEta*, error estimation)

use sundials_core::Real;

use crate::constants::{ETA_MAX, ETA_MIN, SAFETY};

/// Compute the step size adjustment factor eta from the error estimate.
///
/// eta = safety * (1/err_norm)^(1/(q+1))
///
/// where err_norm is the WRMS norm of the local error estimate.
pub(crate) fn compute_eta(err_norm: Real, order: usize) -> Real {
    if err_norm <= 0.0 {
        return ETA_MAX;
    }
    let exp = 1.0 / (order as Real + 1.0);
    let eta = SAFETY * (1.0 / err_norm).powf(exp);
    eta.clamp(ETA_MIN, ETA_MAX)
}

/// Compute the error estimate for order q using the Nordsieck array.
///
/// For BDF: err = (q+1) * z[q+1] / (tq[2] * h)
/// Simplified: err_norm = ||z[q+1]|| * error_coefficient
pub(crate) fn error_estimate_norm(z_qp1: &[Real], ewt: &[Real], error_coeff: Real) -> Real {
    let n = z_qp1.len();
    if n == 0 {
        return 0.0;
    }
    let sum: Real = z_qp1
        .iter()
        .zip(ewt.iter())
        .map(|(zi, wi)| (zi * wi * error_coeff).powi(2))
        .sum();
    (sum / n as Real).sqrt()
}

/// Compute the error weight vector: ewt[i] = 1 / (rtol * |y[i]| + atol).
pub(crate) fn compute_ewt(y: &[Real], rtol: Real, atol: Real, ewt: &mut [Real]) {
    for i in 0..y.len() {
        let tol = rtol * y[i].abs() + atol;
        ewt[i] = if tol > 0.0 { 1.0 / tol } else { 1.0 / atol };
    }
}

/// Select the initial step size based on the RHS evaluation.
///
/// h0 = (rtol)^(1/(q+1)) / max(|f(t0,y0)| / (rtol*|y0| + atol))
pub(crate) fn initial_step(y0: &[Real], f0: &[Real], rtol: Real, atol: Real, order: usize) -> Real {
    let n = y0.len();
    if n == 0 {
        return 1.0;
    }

    // Compute ||f0|| / ||y0|| in weighted norm
    let mut max_ratio: Real = 0.0;
    for i in 0..n {
        let tol = rtol * y0[i].abs() + atol;
        if tol > 0.0 {
            let ratio = f0[i].abs() / tol;
            max_ratio = max_ratio.max(ratio);
        }
    }

    if max_ratio <= 0.0 {
        return 1e-4; // Fallback
    }

    let exp = 1.0 / (order as Real + 1.0);
    let h0 = rtol.powf(exp) / max_ratio;
    h0.clamp(1e-12, 1e6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_eta() {
        // Perfect step (err = 1.0) should give eta ≈ safety
        let eta = compute_eta(1.0, 2);
        assert!((eta - SAFETY).abs() < 0.01);

        // Small error should give large eta (clamped to ETA_MAX)
        let eta = compute_eta(1e-10, 2);
        assert_eq!(eta, ETA_MAX);

        // Large error should give small eta (clamped to ETA_MIN)
        let eta = compute_eta(1e10, 2);
        assert_eq!(eta, ETA_MIN);
    }

    #[test]
    fn test_compute_ewt() {
        let y = [1.0, 0.0, -2.0];
        let mut ewt = [0.0; 3];
        compute_ewt(&y, 1e-4, 1e-8, &mut ewt);
        // ewt[0] = 1 / (1e-4 * 1.0 + 1e-8) ≈ 10000
        assert!((ewt[0] - 1.0 / (1e-4 + 1e-8)).abs() < 1.0);
        // ewt[1] = 1 / (1e-4 * 0.0 + 1e-8) = 1e8
        assert!((ewt[1] - 1e8).abs() < 1.0);
    }
}
