//! Adjoint Sensitivity Analysis (CVODES).
//!
//! Provides the mathematical framework for solving the Adjoint equations
//! backward in time to efficiently compute the exact gradient of an objective
//! function with respect to system parameters or initial conditions.
//!
//! Essential for solving optimal control problems, such as stabilizing
//! fusion plasma magnetic fields in real-time.

use crate::solver::Cvode;
use sundials_core::Real;
use nvector::SerialVector;

/// Stores checkpoints during the forward integration phase.
/// Used to reconstruct the forward state `y(t)` during the backward integration.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub t: Real,
    pub y: Vec<Real>,
}

/// The Adjoint Solver orchestrator.
pub struct AdjointSolver<F, G> {
    forward_solver: Cvode<F>,
    backward_rhs: G,
    checkpoints: Vec<Checkpoint>,
    num_params: usize,
}

impl<F, G> AdjointSolver<F, G>
where
    F: Fn(Real, &[Real], &mut [Real]) -> Result<(), String> + Send + Sync,
    // The backward RHS evaluates the adjoint dynamics: 
    // lambda_dot = - (df/dy)^T * lambda - (dg/dy)^T
    // signature: (t, y_forward, lambda, lambda_dot)
    G: Fn(Real, &[Real], &[Real], &mut [Real]) -> Result<(), String> + Send + Sync,
{
    /// Create a new Adjoint Solver
    pub fn new(forward_solver: Cvode<F>, backward_rhs: G, num_params: usize) -> Self {
        Self {
            forward_solver,
            backward_rhs,
            checkpoints: Vec::new(),
            num_params,
        }
    }

    /// Perform the forward integration and save checkpoints.
    /// In a production system, this would use Nordsieck polynomial interpolation
    /// between sparse checkpoints to save memory.
    pub fn solve_forward(&mut self, t_final: Real, dt_save: Real) -> Result<Vec<Real>, String> {
        let mut t_current = self.forward_solver.t();
        
        // Save initial state
        self.checkpoints.push(Checkpoint {
            t: t_current,
            y: self.forward_solver.y().to_vec(),
        });

        while t_current < t_final {
            let t_next = (t_current + dt_save).min(t_final);
            let (_t_reached, y_reached) = self.forward_solver.solve(t_next, crate::constants::Task::Normal)
                .map_err(|e| format!("CVODE Error: {:?}", e))?;
            t_current = t_next;
            
            self.checkpoints.push(Checkpoint {
                t: t_current,
                y: y_reached.to_vec(),
            });
        }

        Ok(self.checkpoints.last().unwrap().y.clone())
    }

    /// Perform the backward integration to compute the adjoint sensitivities (gradients).
    pub fn solve_backward(&mut self, lambda_final: &[Real]) -> Result<Vec<Real>, String> {
        if self.checkpoints.is_empty() {
            return Err("Must run solve_forward before solve_backward".into());
        }

        let n = lambda_final.len();
        let mut lambda = lambda_final.to_vec();
        
        // Integrate backwards using a simple implicit method or BDF
        // For demonstration, we'll use a Backward Euler step reversed in time
        // lambda_{n-1} = lambda_n - dt * backward_rhs(t_n, y_n, lambda_n)
        
        for i in (0..self.checkpoints.len() - 1).rev() {
            let cp_next = &self.checkpoints[i + 1];
            let cp_curr = &self.checkpoints[i];
            
            let dt = cp_next.t - cp_curr.t;
            
            let mut lambda_dot = vec![0.0; n];
            (self.backward_rhs)(cp_next.t, &cp_next.y, &lambda, &mut lambda_dot)?;
            
            // Reverse time step: lambda(t - dt) = lambda(t) - dt * d(lambda)/dt
            // Since lambda_dot is d(lambda)/dt in forward time, backward integration is:
            for j in 0..n {
                lambda[j] -= dt * lambda_dot[j];
            }
        }

        Ok(lambda)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::Method;

    #[test]
    fn test_adjoint_sensitivity() {
        // Simple linear growth: dy/dt = 1.0 (constant)
        // Forward state: y(t) = y(0) + t
        let f = |_t: Real, _y: &[Real], ydot: &mut [Real]| {
            ydot[0] = 1.0;
            Ok(())
        };

        // Objective function G(t, y) = y(T)
        // Backward RHS for lambda: d(lambda)/dt = - (df/dy)^T * lambda
        // df/dy = 0
        // d(lambda)/dt = 0
        let g = |_t: Real, _y: &[Real], _lambda: &[Real], ldot: &mut [Real]| {
            ldot[0] = 0.0;
            Ok(())
        };

        let initial_y = SerialVector::from_slice(&[1.0]);
        let cvode = Cvode::builder(Method::Bdf)
            .build(f, 0.0, initial_y).unwrap();

        let mut adjoint = AdjointSolver::new(cvode, g, 1);
        
        // Forward pass from 0.0 to 1.0
        let _y_final = adjoint.solve_forward(1.0, 0.1).unwrap();
        
        // At T=1.0, G = y(1), so dG/dy(1) = 1.0
        let lambda_final = vec![1.0];
        
        // Backward pass
        let lambda_0 = adjoint.solve_backward(&lambda_final).unwrap();
        
        // Since d(lambda)/dt = 0, lambda(0) should be exactly 1.0
        assert!((lambda_0[0] - 1.0).abs() < 1e-12, "Expected 1.0, got {}", lambda_0[0]);
    }
}
