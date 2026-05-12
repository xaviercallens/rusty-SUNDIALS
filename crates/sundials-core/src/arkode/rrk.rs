//! Relaxation Runge-Kutta (RRK) — Structure-Preserving Integration
//!
//! Standard explicit Runge-Kutta methods suffer from "energy drift" when
//! simulating conservative physical systems (e.g., orbital mechanics, plasma physics).
//!
//! "Relaxation" Runge-Kutta (RRK) applies a scalar multiplier `γ` to the
//! standard RK update step, making any RK method strictly energy/entropy conserving.
//!
//! `y_{n+1} = y_n + γ * Δy`
//!
//! where `γ` is chosen such that the invariant `E` is perfectly conserved:
//! `E(y_n + γ * Δy) = E(y_n)`.
//!
//! # Reference
//! Ketcheson, D. I. (2019). "Relaxation Runge-Kutta Methods: Conservation and
//! Stability for Inner-Product Norms." SIAM Journal on Numerical Analysis.

use crate::Real;

/// Error type for the RRK relaxation parameter solver.
#[derive(Debug, Clone, PartialEq)]
pub enum RrkError {
    /// The root-finding iterations failed to converge.
    NotConverged,
    /// The derivative of the invariant function was zero (or nearly zero)
    /// in the search direction, preventing the Newton step.
    ZeroDerivative,
    /// The baseline step size `Δy` was practically zero.
    ZeroStep,
}

/// Computes the Relaxation Runge-Kutta scalar parameter `γ`.
///
/// Finds `γ` close to 1.0 such that `E(y_n + γ * Δy) = E(y_n)`.
/// We solve the nonlinear scalar equation `g(γ) = E(y_n + γ * Δy) - E(y_n) = 0`
/// using Newton's method.
///
/// To perform Newton's method without requiring the user to supply the gradient
/// `∇E`, we use central finite differences to approximate `g'(γ)` along the
/// direction `Δy`.
///
/// # Arguments
/// * `y_n` - The state at the beginning of the step.
/// * `delta_y` - The proposed update step from the baseline RK method (`y_{n+1}^* - y_n`).
/// * `invariant_fn` - A closure `E(y)` that computes the scalar conserved quantity.
/// * `tol` - The tolerance for the root-finder (e.g., 1e-12).
/// * `max_iters` - Maximum Newton iterations (typically converges in 1-3).
///
/// # Returns
/// The scalar `γ`, or an error if the root-finder fails.
pub fn compute_relaxation_parameter<F>(
    y_n: &[Real],
    delta_y: &[Real],
    mut invariant_fn: F,
    tol: Real,
    max_iters: usize,
) -> Result<Real, RrkError>
where
    F: FnMut(&[Real]) -> Real,
{
    let n = y_n.len();
    assert_eq!(n, delta_y.len(), "Dimension mismatch");

    // Check if the baseline step is virtually zero. If so, γ = 1.0 is exact.
    let mut dy_norm2 = 0.0;
    for &dy in delta_y {
        dy_norm2 += dy * dy;
    }
    if dy_norm2 < 1e-30 {
        return Ok(1.0);
    }

    let e_n = invariant_fn(y_n);
    let mut gamma = 1.0; // Initial guess is the standard RK step

    // Scratch buffer to hold y_n + γ * Δy
    let mut y_eval = vec![0.0; n];

    // Newton iteration: γ_{k+1} = γ_k - g(γ_k) / g'(γ_k)
    // where g(γ) = E(y_n + γ * Δy) - E_n
    for _ in 0..max_iters {
        // Evaluate g(γ)
        for i in 0..n {
            y_eval[i] = y_n[i] + gamma * delta_y[i];
        }
        let e_eval = invariant_fn(&y_eval);
        let g_val = e_eval - e_n;

        // Check for convergence
        if g_val.abs() < tol {
            return Ok(gamma);
        }

        // Approximate g'(γ) using central finite differences along the direction Δy.
        // g'(γ) = [g(γ + h) - g(γ - h)] / 2h
        // where h is a small perturbation.
        let h_fd = 1e-6_f64.max(1e-8 * gamma.abs());

        for i in 0..n {
            y_eval[i] = y_n[i] + (gamma + h_fd) * delta_y[i];
        }
        let g_plus = invariant_fn(&y_eval) - e_n;

        for i in 0..n {
            y_eval[i] = y_n[i] + (gamma - h_fd) * delta_y[i];
        }
        let g_minus = invariant_fn(&y_eval) - e_n;

        let g_prime = (g_plus - g_minus) / (2.0 * h_fd);

        if g_prime.abs() < 1e-14 {
            return Err(RrkError::ZeroDerivative);
        }

        gamma -= g_val / g_prime;
    }

    Err(RrkError::NotConverged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrk_quadratic_invariant() {
        // Harmonic oscillator: y'' = -y
        // State: [q, p]. Invariant E = (q^2 + p^2)/2
        //
        // Let's take a state and a small step that drifts energy, and see if RRK fixes it.
        let y_n = [1.0, 0.0]; // E = 0.5
        let delta_y = [-0.1, 0.9]; // y_new = [0.9, 0.9] -> E = (0.81 + 0.81)/2 = 0.81 (drift!)

        let invariant = |y: &[Real]| -> Real { 0.5 * (y[0] * y[0] + y[1] * y[1]) };

        let e_n = invariant(&y_n);
        assert_eq!(e_n, 0.5);

        let gamma = compute_relaxation_parameter(&y_n, &delta_y, invariant, 1e-12, 10).unwrap();

        // Compute the relaxed step
        let y_relaxed = [y_n[0] + gamma * delta_y[0], y_n[1] + gamma * delta_y[1]];

        let e_relaxed = invariant(&y_relaxed);

        // Gamma should not be 1.0 (since 1.0 drifts)
        assert!((gamma - 1.0).abs() > 0.01);

        // But the energy should be perfectly conserved
        assert!(
            (e_relaxed - e_n).abs() < 1e-10,
            "E_relaxed={} != E_n={}",
            e_relaxed,
            e_n
        );
    }
}
