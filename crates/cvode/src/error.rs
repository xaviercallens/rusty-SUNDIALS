//! CVODE-specific error types.

use sundials_core::SundialsError;
use thiserror::Error;

/// Errors specific to the CVODE solver.
#[derive(Debug, Error, Clone)]
pub enum CvodeError {
    #[error("CVODE solver error: {0}")]
    Solver(#[from] SundialsError),

    #[error("RHS function returned error at t={t}: {msg}")]
    RhsError { t: f64, msg: String },

    #[error("step size too small: h={h} at t={t}")]
    StepTooSmall { h: f64, t: f64 },

    #[error("maximum steps exceeded ({max}) at t={t}")]
    MaxSteps { max: usize, t: f64 },

    #[error("solver not initialized")]
    NotInitialized,

    #[error("invalid configuration: {0}")]
    Config(String),
}
