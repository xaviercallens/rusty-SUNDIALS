//! SUNDIALS Context — shared state for solver components.
//!
//! Replaces `SUNContext` from C. In Rust, this is a lightweight struct
//! that holds shared configuration rather than a void pointer.

/// Shared context for SUNDIALS solver components.
///
/// In the C implementation, `SUNContext` manages memory pools, profiling,
/// and logging. In Rust, RAII handles memory, so this is simplified to
/// hold configuration and optional profiling state.
#[derive(Debug, Clone)]
pub struct Context {
    /// Enable profiling of solver operations.
    pub profiling: bool,
    /// Enable logging of solver progress.
    pub logging: bool,
}

impl Context {
    /// Create a new default context.
    pub fn new() -> Self {
        Self {
            profiling: false,
            logging: false,
        }
    }

    /// Create a context with profiling enabled.
    pub fn with_profiling() -> Self {
        Self {
            profiling: true,
            logging: false,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
