//! Builder pattern for CVODE solver construction.

use nvector::SerialVector;
use sundials_core::Real;

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

    /// Build the solver with the given RHS function and initial conditions.
    pub fn build<F>(self, rhs: F, t0: Real, y0: SerialVector) -> Result<Cvode<F>, CvodeError>
    where
        F: FnMut(Real, &[Real], &mut [Real]) -> Result<(), String>,
    {
        if self.rtol <= 0.0 {
            return Err(CvodeError::Config("rtol must be positive".into()));
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
        ))
    }
}
