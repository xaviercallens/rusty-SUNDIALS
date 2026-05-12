//! Implicit-Explicit (IMEX) Runge-Kutta Solvers for ARKODE.
//!
//! Solves ODEs of the form: dy/dt = f^E(t, y) + f^I(t, y)
//! where f^E is the explicit (non-stiff) part and f^I is the implicit (stiff) part.
//!
//! By splitting the physics, IMEX avoids solving expensive non-linear systems
//! for the non-stiff advection/convection terms, only treating the stiff
//! diffusion/reaction terms implicitly. This is critical for plasma physics
//! where advection is fast but diffusion is ultra-fast.

use crate::Real;

/// Defines an IMEX Runge-Kutta Tableau.
#[derive(Debug, Clone)]
pub struct ImexTableau {
    pub stages: usize,
    pub c: Vec<Real>,
    pub a_e: Vec<Vec<Real>>, // Explicit Butcher matrix (strictly lower triangular)
    pub a_i: Vec<Vec<Real>>, // Implicit Butcher matrix (lower triangular)
    pub b_e: Vec<Real>,
    pub b_i: Vec<Real>,
}

impl ImexTableau {
    /// ARS222: 2nd-order, 2-stage IMEX method by Ascher, Ruuth, and Spiteri.
    pub fn ars222() -> Self {
        let gamma = 1.0 - (1.0 / 2.0_f64).sqrt();
        let delta = 1.0 - 1.0 / (2.0 * gamma);

        Self {
            stages: 2,
            c: vec![0.0, gamma, 1.0],
            a_e: vec![
                vec![0.0, 0.0, 0.0],
                vec![gamma, 0.0, 0.0],
                vec![delta, 1.0 - delta, 0.0],
            ],
            a_i: vec![
                vec![0.0, 0.0, 0.0],
                vec![0.0, gamma, 0.0],
                vec![0.0, 1.0 - gamma, gamma],
            ],
            b_e: vec![delta, 1.0 - delta, 0.0],
            b_i: vec![0.0, 1.0 - gamma, gamma],
        }
    }
}

/// A basic IMEX Solver.
pub struct ImexSolver<FE, FI> {
    tableau: ImexTableau,
    f_e: FE,
    f_i: FI,
    n: usize,
    rtol: Real,
    atol: Real,
}

impl<FE, FI> ImexSolver<FE, FI>
where
    FE: Fn(Real, &[Real], &mut [Real]) -> Result<(), String>,
    FI: Fn(Real, &[Real], &mut [Real]) -> Result<(), String>,
{
    pub fn new(tableau: ImexTableau, f_e: FE, f_i: FI, n: usize) -> Self {
        Self {
            tableau,
            f_e,
            f_i,
            n,
            rtol: 1e-4,
            atol: 1e-8,
        }
    }

    /// Set tolerances
    pub fn tolerances(mut self, rtol: Real, atol: Real) -> Self {
        self.rtol = rtol;
        self.atol = atol;
        self
    }

    /// Perform a single IMEX step of size `h` from `t` and `y`.
    ///
    /// This uses fixed-point iteration for the implicit solve to keep the
    /// implementation generic without requiring a Jacobian. For highly stiff
    /// systems, this would be replaced with a Newton-Krylov solver.
    pub fn step(&self, t: Real, y: &[Real], h: Real, y_next: &mut [Real]) -> Result<(), String> {
        let s = self.tableau.stages;
        let mut k_e = vec![vec![0.0; self.n]; s + 1];
        let mut k_i = vec![vec![0.0; self.n]; s + 1];

        let mut y_stage = vec![0.0; self.n];
        let mut z_i = vec![0.0; self.n];

        for i in 0..=s {
            // Calculate explicit prediction for stage i
            for j in 0..self.n {
                y_stage[j] = y[j];
                for j_stage in 0..i {
                    y_stage[j] += h * self.tableau.a_e[i][j_stage] * k_e[j_stage][j];
                    y_stage[j] += h * self.tableau.a_i[i][j_stage] * k_i[j_stage][j];
                }
            }

            let t_i = t + self.tableau.c[i] * h;

            // If the diagonal implicit term is 0, it's fully explicit
            if self.tableau.a_i[i][i] == 0.0 {
                (self.f_e)(t_i, &y_stage, &mut k_e[i])?;
                (self.f_i)(t_i, &y_stage, &mut k_i[i])?;
            } else {
                // Fixed point iteration for the implicit stage
                let mut iter = 0;
                let mut diff = 1.0;
                let a_ii = self.tableau.a_i[i][i];

                // Initial guess for implicit part
                for j in 0..self.n {
                    z_i[j] = y_stage[j];
                }

                while diff > self.rtol && iter < 100 {
                    let mut k_tmp = vec![0.0; self.n];
                    (self.f_i)(t_i, &z_i, &mut k_tmp)?;

                    let mut max_diff = 0.0_f64;
                    for j in 0..self.n {
                        let z_new = y_stage[j] + h * a_ii * k_tmp[j];
                        let d = (z_new - z_i[j]).abs() / (self.atol + self.rtol * z_i[j].abs());
                        if d > max_diff {
                            max_diff = d;
                        }
                        z_i[j] = z_new;
                        k_i[i][j] = k_tmp[j];
                    }
                    diff = max_diff;
                    iter += 1;
                }

                if iter == 100 {
                    return Err(format!(
                        "IMEX fixed-point solver failed to converge at stage {}",
                        i
                    ));
                }

                // Evaluate explicit part at the final solved stage
                (self.f_e)(t_i, &z_i, &mut k_e[i])?;
            }
        }

        // Final assembly
        for j in 0..self.n {
            y_next[j] = y[j];
            for i in 0..=s {
                y_next[j] += h * self.tableau.b_e[i] * k_e[i][j];
                y_next[j] += h * self.tableau.b_i[i] * k_i[i][j];
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ars222_imex() {
        // y' = -10 y + 0 (stiff part is -10y, explicit is 0)
        let f_e = |_t: Real, _y: &[Real], ydot: &mut [Real]| {
            ydot[0] = 0.0;
            Ok(())
        };
        let f_i = |_t: Real, y: &[Real], ydot: &mut [Real]| {
            ydot[0] = -10.0 * y[0];
            Ok(())
        };

        let solver = ImexSolver::new(ImexTableau::ars222(), f_e, f_i, 1);
        let mut y = [1.0];
        let mut y_next = [0.0];

        solver.step(0.0, &y, 0.01, &mut y_next).unwrap();
        y[0] = y_next[0];

        // Exact solution for y' = -10y at t=0.01 is exp(-0.1) ~ 0.904837
        assert!((y[0] - (-0.1_f64).exp()).abs() < 1e-3);
    }
}
