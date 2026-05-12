//! CVODE-specific error types.

use sundials_core::SundialsError;

/// Errors specific to the CVODE solver.
#[derive(Debug, Clone)]
pub enum CvodeError {
    Solver(SundialsError),

    RhsError { t: f64, msg: String },

    StepTooSmall { h: f64, t: f64 },

    MaxSteps { max: usize, t: f64 },

    NotInitialized,

    Config(String),
}

impl std::error::Error for CvodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Solver(err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for CvodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Solver(err) => write!(f, "CVODE solver error: {}", err),
            Self::RhsError { t, msg } => {
                write!(f, "RHS function returned error at t={}: {}", t, msg)
            }
            Self::StepTooSmall { h, t } => write!(f, "step size too small: h={} at t={}", h, t),
            Self::MaxSteps { max, t } => write!(f, "maximum steps exceeded ({}) at t={}", max, t),
            Self::NotInitialized => write!(f, "solver not initialized"),
            Self::Config(msg) => write!(f, "invalid configuration: {}", msg),
        }
    }
}

impl From<SundialsError> for CvodeError {
    fn from(err: SundialsError) -> Self {
        Self::Solver(err)
    }
}
