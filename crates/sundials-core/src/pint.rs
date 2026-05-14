//! Parallel-in-Time (PinT) Orchestrator using the Parareal Algorithm
//!
//! This module implements the Parareal algorithm, breaking the sequential bottleneck 
//! of standard time integration. By decomposing the temporal domain $[T_0, T_f]$ into 
//! $N$ time slices, it leverages `Rayon` to parallelize the expensive fine integration 
//! steps, utilizing a cheap coarse solver to propagate initial conditions.

use rayon::prelude::*;
use crate::{Real, Result, SundialsError};

/// Defines the signatures for Coarse and Fine Solvers
pub trait TimeIntegrator: Sync + Send {
    /// Steps the system from `t_start` to `t_end` given the initial state `y_start`.
    /// Returns the final state `y_end`.
    fn step(&self, t_start: Real, t_end: Real, y_start: &[Real]) -> Result<Vec<Real>>;
}

/// The Parareal Orchestrator Configuration
pub struct PararealOrchestrator<'a, C, F>
where
    C: TimeIntegrator,
    F: TimeIntegrator,
{
    pub coarse_solver: &'a C,
    pub fine_solver: &'a F,
    pub max_iterations: usize,
    pub tolerance: Real,
}

impl<'a, C, F> PararealOrchestrator<'a, C, F>
where
    C: TimeIntegrator,
    F: TimeIntegrator,
{
    /// Constructs a new Parallel-in-Time Orchestrator
    pub fn new(coarse: &'a C, fine: &'a F, max_iters: usize, tol: Real) -> Self {
        Self {
            coarse_solver: coarse,
            fine_solver: fine,
            max_iterations: max_iters,
            tolerance: tol,
        }
    }

    /// Executes the Parareal algorithm over `num_slices`
    pub fn solve(
        &self,
        t_start: Real,
        t_end: Real,
        y0: &[Real],
        num_slices: usize,
    ) -> Result<Vec<Vec<Real>>> {
        if num_slices < 2 {
            return Err(SundialsError::IntegrationFailure);
        }

        let dt = (t_end - t_start) / (num_slices as Real);
        let mut time_slices = Vec::with_capacity(num_slices + 1);
        for i in 0..=num_slices {
            time_slices.push(t_start + (i as Real) * dt);
        }

        // Initialize states at each time slice boundary
        // y[n] holds the state at time_slices[n]
        let mut y = vec![y0.to_vec(); num_slices + 1];

        // 1. Initial Coarse Pass (Sequential)
        for n in 0..num_slices {
            y[n + 1] = self.coarse_solver.step(time_slices[n], time_slices[n + 1], &y[n])?;
        }

        // Parareal Iteration
        for k in 0..self.max_iterations {
            // 2. Parallel Fine Pass
            // F(y_n^k) computed in parallel across all time slices
            let fine_results: Vec<Result<Vec<Real>>> = (0..num_slices)
                .into_par_iter()
                .map(|n| {
                    self.fine_solver.step(time_slices[n], time_slices[n + 1], &y[n])
                })
                .collect();

            let mut fine_states = Vec::with_capacity(num_slices);
            for res in fine_results {
                fine_states.push(res?);
            }

            let mut max_diff: Real = 0.0;
            let mut y_next = y.clone();

            // 3. Sequential Coarse Correction
            for n in 0..num_slices {
                // Compute new coarse prediction from the updated y_n^{k+1}
                let coarse_new = self.coarse_solver.step(time_slices[n], time_slices[n + 1], &y_next[n])?;
                // Compute old coarse prediction from y_n^k
                let coarse_old = self.coarse_solver.step(time_slices[n], time_slices[n + 1], &y[n])?;

                // Parareal update: y_{n+1}^{k+1} = G(y_n^{k+1}) + F(y_n^k) - G(y_n^k)
                for i in 0..y_next[n + 1].len() {
                    y_next[n + 1][i] = coarse_new[i] + fine_states[n][i] - coarse_old[i];
                    
                    let diff = (y_next[n + 1][i] - y[n + 1][i]).abs();
                    if diff > max_diff {
                        max_diff = diff;
                    }
                }
            }

            y = y_next;

            // Convergence check
            if max_diff < self.tolerance {
                break;
            }
        }

        Ok(y)
    }
}
