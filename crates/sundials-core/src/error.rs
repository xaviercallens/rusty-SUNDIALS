//! Error types for SUNDIALS operations.
//!
//! Replaces the C integer return codes with a proper Rust error enum.
//! Each variant maps to a specific SUNDIALS error condition.

use thiserror::Error;

/// Result type alias for SUNDIALS operations.
pub type Result<T> = std::result::Result<T, SundialsError>;

/// Errors that can occur during SUNDIALS operations.
///
/// Maps directly to CV_* error codes from the C implementation.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum SundialsError {
    // --- Solver errors ---
    #[error("too much work: solver took max internal steps but not reached tout")]
    TooMuchWork,

    #[error("too much accuracy requested: tolerances too tight")]
    TooMuchAccuracy,

    #[error("error test failures: too many error test failures at one step")]
    ErrTestFailure,

    #[error("convergence failure: nonlinear solver failed to converge")]
    ConvFailure,

    // --- Linear solver errors ---
    #[error("linear solver initialization failed")]
    LinInitFail,

    #[error("linear solver setup failed")]
    LinSetupFail,

    #[error("linear solver solve failed")]
    LinSolveFail,

    // --- RHS function errors ---
    #[error("RHS function failed unrecoverably")]
    RhsFuncFail,

    #[error("first RHS function call failed")]
    FirstRhsFuncErr,

    #[error("repeated recoverable RHS function errors")]
    RepeatedRhsFuncErr,

    #[error("unrecoverable RHS function error")]
    UnrecRhsFuncErr,

    // --- Root finding errors ---
    #[error("root function failed")]
    RootFuncFail,

    // --- Nonlinear solver errors ---
    #[error("nonlinear solver initialization failed")]
    NlsInitFail,

    #[error("nonlinear solver setup failed")]
    NlsSetupFail,

    #[error("nonlinear solver failed")]
    NlsFail,

    #[error("constraint satisfaction failed")]
    ConstraintFail,

    // --- Memory/input errors ---
    #[error("memory allocation failed")]
    MemFail,

    #[error("solver memory is null (not initialized)")]
    MemNull,

    #[error("illegal input: {0}")]
    IllInput(String),

    #[error("solver not initialized (CVodeInit not called)")]
    NoMalloc,

    #[error("bad k value for derivative interpolation")]
    BadK,

    #[error("bad t value: time is outside interpolation interval")]
    BadT,

    #[error("bad dky computation")]
    BadDky,

    #[error("tout too close to t0")]
    TooClose,

    #[error("vector operation error")]
    VectorOpErr,

    // --- Projection errors ---
    #[error("projection memory is null")]
    ProjMemNull,

    #[error("projection function failed")]
    ProjFuncFail,

    #[error("repeated projection function errors")]
    RepeatedProjFuncErr,

    // --- Context errors ---
    #[error("context error")]
    ContextErr,

    #[error("unrecognized error: {0}")]
    Unrecognized(i32),
}

impl SundialsError {
    /// Convert from a C-style integer return code.
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => None, // CV_SUCCESS
            -1 => Some(Self::TooMuchWork),
            -2 => Some(Self::TooMuchAccuracy),
            -3 => Some(Self::ErrTestFailure),
            -4 => Some(Self::ConvFailure),
            -5 => Some(Self::LinInitFail),
            -6 => Some(Self::LinSetupFail),
            -7 => Some(Self::LinSolveFail),
            -8 => Some(Self::RhsFuncFail),
            -9 => Some(Self::FirstRhsFuncErr),
            -10 => Some(Self::RepeatedRhsFuncErr),
            -11 => Some(Self::UnrecRhsFuncErr),
            -12 => Some(Self::RootFuncFail),
            -13 => Some(Self::NlsInitFail),
            -14 => Some(Self::NlsSetupFail),
            -15 => Some(Self::ConstraintFail),
            -16 => Some(Self::NlsFail),
            -20 => Some(Self::MemFail),
            -21 => Some(Self::MemNull),
            -22 => Some(Self::IllInput("unspecified".into())),
            -23 => Some(Self::NoMalloc),
            -24 => Some(Self::BadK),
            -25 => Some(Self::BadT),
            -26 => Some(Self::BadDky),
            -27 => Some(Self::TooClose),
            -28 => Some(Self::VectorOpErr),
            -29 => Some(Self::ProjMemNull),
            -30 => Some(Self::ProjFuncFail),
            -31 => Some(Self::RepeatedProjFuncErr),
            -32 => Some(Self::ContextErr),
            other => Some(Self::Unrecognized(other)),
        }
    }
}
