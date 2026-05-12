//! CVODE — Variable-coefficient ODE solver.
//!
//! Solves initial value problems for ODE systems of the form:
//!   dy/dt = f(t, y),  y(t0) = y0
//!
//! Supports:
//! - BDF methods (orders 1-5) for stiff problems
//! - Adams-Moulton methods (orders 1-12) for non-stiff problems
//! - Adaptive step size control with error estimation
//! - Root finding during integration
//!
//! Translated from: `cvode/cvode.c` (~5000 LOC core solver)

#![allow(clippy::all, unused_variables, dead_code, unused_imports, unused_mut)]

mod adjoint;
mod builder;
mod constants;
mod error;
mod nordsieck;
mod solver;
mod step;

pub use adjoint::{AdjointSolver, Checkpoint};
pub use builder::CvodeBuilder;
pub use constants::{Method, Task};
pub use error::CvodeError;
pub use solver::Cvode;

/// Return status from a successful solve step.
#[derive(Debug, Clone, PartialEq)]
pub enum SolveStatus {
    /// Successfully reached tout.
    Success,
    /// Reached a stop time (tstop).
    TstopReturn,
    /// A root was found.
    RootReturn,
}
