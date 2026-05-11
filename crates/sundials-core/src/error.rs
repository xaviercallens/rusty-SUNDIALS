//! Error types for SUNDIALS operations.
//!
//! Replaces the C integer return codes with a proper Rust error enum.
//! Each variant maps to a specific SUNDIALS error condition.



/// Result type alias for SUNDIALS operations.
pub type Result<T> = std::result::Result<T, SundialsError>;

/// Errors that can occur during SUNDIALS operations.
///
/// Maps directly to CV_* error codes from the C implementation.
#[derive(Debug, Clone, PartialEq)]
pub enum SundialsError {
    // --- Solver errors ---

    TooMuchWork,


    TooMuchAccuracy,


    ErrTestFailure,


    ConvFailure,

    // --- Linear solver errors ---

    LinInitFail,


    LinSetupFail,


    LinSolveFail,

    // --- RHS function errors ---

    RhsFuncFail,


    FirstRhsFuncErr,


    RepeatedRhsFuncErr,


    UnrecRhsFuncErr,

    // --- Root finding errors ---

    RootFuncFail,

    // --- Nonlinear solver errors ---

    NlsInitFail,


    NlsSetupFail,


    NlsFail,


    ConstraintFail,

    // --- Memory/input errors ---

    MemFail,


    MemNull,


    IllInput(String),


    NoMalloc,


    BadK,


    BadT,


    BadDky,


    TooClose,


    VectorOpErr,

    // --- Projection errors ---

    ProjMemNull,


    ProjFuncFail,


    RepeatedProjFuncErr,

    // --- Context errors ---

    ContextErr,

    Unrecognized(i32),
}

impl std::error::Error for SundialsError {}

impl std::fmt::Display for SundialsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooMuchWork => write!(f, "too much work: solver took max internal steps but not reached tout"),
            Self::TooMuchAccuracy => write!(f, "too much accuracy requested: tolerances too tight"),
            Self::ErrTestFailure => write!(f, "error test failures: too many error test failures at one step"),
            Self::ConvFailure => write!(f, "convergence failure: nonlinear solver failed to converge"),
            Self::LinInitFail => write!(f, "linear solver initialization failed"),
            Self::LinSetupFail => write!(f, "linear solver setup failed"),
            Self::LinSolveFail => write!(f, "linear solver solve failed"),
            Self::RhsFuncFail => write!(f, "RHS function failed unrecoverably"),
            Self::FirstRhsFuncErr => write!(f, "first RHS function call failed"),
            Self::RepeatedRhsFuncErr => write!(f, "repeated recoverable RHS function errors"),
            Self::UnrecRhsFuncErr => write!(f, "unrecoverable RHS function error"),
            Self::RootFuncFail => write!(f, "root function failed"),
            Self::NlsInitFail => write!(f, "nonlinear solver initialization failed"),
            Self::NlsSetupFail => write!(f, "nonlinear solver setup failed"),
            Self::NlsFail => write!(f, "nonlinear solver failed"),
            Self::ConstraintFail => write!(f, "constraint satisfaction failed"),
            Self::MemFail => write!(f, "memory allocation failed"),
            Self::MemNull => write!(f, "solver memory is null (not initialized)"),
            Self::IllInput(s) => write!(f, "illegal input: {}", s),
            Self::NoMalloc => write!(f, "solver not initialized (CVodeInit not called)"),
            Self::BadK => write!(f, "bad k value for derivative interpolation"),
            Self::BadT => write!(f, "bad t value: time is outside interpolation interval"),
            Self::BadDky => write!(f, "bad dky computation"),
            Self::TooClose => write!(f, "tout too close to t0"),
            Self::VectorOpErr => write!(f, "vector operation error"),
            Self::ProjMemNull => write!(f, "projection memory is null"),
            Self::ProjFuncFail => write!(f, "projection function failed"),
            Self::RepeatedProjFuncErr => write!(f, "repeated projection function errors"),
            Self::ContextErr => write!(f, "context error"),
            Self::Unrecognized(code) => write!(f, "unrecognized error: {}", code),
        }
    }
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
