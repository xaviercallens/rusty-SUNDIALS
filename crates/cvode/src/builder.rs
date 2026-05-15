//! Builder pattern for CVODE solver construction.

use nvector::SerialVector;
use sundials_core::Real;
use sundials_core::generated::sundials_dense::DenseMat;

use crate::constants::Method;
use crate::error::CvodeError;
use crate::solver::Cvode;

/// Builder for configuring and constructing a CVODE solver.
pub struct CvodeBuilder {
    method: Method,
    rtol: Real,
    atol: Real,
    max_steps: usize,
    max_order: Option<usize>,
    init_step: Option<Real>,
    max_step: Option<Real>,
    min_step: Option<Real>,
    jac: Option<Box<dyn FnMut(Real, &[Real], &mut DenseMat) -> Result<(), String> + Send + Sync>>,
}

impl CvodeBuilder {
    /// Create a new builder with the specified method.
    pub fn new(method: Method) -> Self {
        Self {
            method,
            rtol: 1e-4,
            atol: 1e-8,
            max_steps: 500,
            max_order: None,
            init_step: None,
            max_step: None,
            min_step: None,
            jac: None,
        }
    }

    /// Set the relative tolerance.
    pub fn rtol(mut self, rtol: Real) -> Self {
        self.rtol = rtol;
        self
    }

    /// Set the absolute tolerance (scalar).
    pub fn atol(mut self, atol: Real) -> Self {
        self.atol = atol;
        self
    }

    /// Set the maximum number of internal steps.
    pub fn max_steps(mut self, n: usize) -> Self {
        self.max_steps = n;
        self
    }

    /// Set the maximum method order.
    pub fn max_order(mut self, q: usize) -> Self {
        self.max_order = Some(q);
        self
    }

    /// Set the initial step size (0 = automatic).
    pub fn init_step(mut self, h: Real) -> Self {
        self.init_step = Some(h);
        self
    }

    /// Set the maximum step size.
    pub fn max_step(mut self, h: Real) -> Self {
        self.max_step = Some(h);
        self
    }

    /// Set the minimum step size.
    pub fn min_step(mut self, h: Real) -> Self {
        self.min_step = Some(h);
        self
    }

    /// Provide an analytical Jacobian function.
    ///
    /// The function signature is `(t, y, jac_out) -> Result<(), String>` where
    /// `jac_out` is a `DenseMat` in **column-major** order: `cols[j][i] = ∂f_i/∂y_j`.
    ///
    /// When provided, the Jacobian is evaluated analytically instead of by
    /// finite differences, eliminating `n` extra RHS evaluations per Jacobian
    /// computation and giving exact Newton directions. This is the primary
    /// fix for the step-count efficiency gap vs LLNL C SUNDIALS.
    ///
    /// # Example (Robertson kinetics)
    /// ```rust,ignore
    /// .jacobian(|_t, y, j| {
    ///     j.cols[0][0] = -0.04;          j.cols[1][0] = 1e4 * y[2];  j.cols[2][0] = 1e4 * y[1];
    ///     j.cols[0][1] =  0.04;          j.cols[1][1] = -1e4 * y[2] - 6e7 * y[1];  j.cols[2][1] = -1e4 * y[1];
    ///     j.cols[0][2] =  0.0;           j.cols[1][2] = 6e7 * y[1]; j.cols[2][2] = 0.0;
    ///     Ok(())
    /// })
    /// ```
    pub fn jacobian<J>(mut self, jac: J) -> Self
    where
        J: FnMut(Real, &[Real], &mut DenseMat) -> Result<(), String> + Send + Sync + 'static,
    {
        self.jac = Some(Box::new(jac));
        self
    }

    /// Build the solver with the given RHS function and initial conditions.
    pub fn build<F>(self, rhs: F, t0: Real, y0: SerialVector) -> Result<Cvode<F>, CvodeError>
    where
        F: FnMut(Real, &[Real], &mut [Real]) -> Result<(), String> + Send + Sync,
    {
        if self.rtol < 0.0 {
            return Err(CvodeError::Config("rtol must be non-negative".into()));
        }
        if self.atol <= 0.0 {
            return Err(CvodeError::Config("atol must be positive".into()));
        }

        let max_order = self.max_order.unwrap_or(self.method.max_order());
        if max_order > self.method.max_order() {
            return Err(CvodeError::Config(format!(
                "max_order {} exceeds method maximum {}",
                max_order,
                self.method.max_order()
            )));
        }

        Ok(Cvode::new(
            self.method,
            rhs,
            t0,
            y0,
            self.rtol,
            self.atol,
            max_order,
            self.max_steps,
            self.init_step,
            self.max_step,
            self.min_step,
            self.jac,
        ))
    }
}
