//! Core types for rusty-SUNDIALS.
//!
//! Provides the fundamental numeric types, error handling, and context
//! management used across all SUNDIALS components.
//!
//! Translated from: `sundials/sundials_types.h`, `sundials/sundials_context.h`

pub mod arkode;
pub mod band_solver;
pub mod context;
pub mod dual;
pub mod epirk;
pub mod error;
pub mod generated;
pub mod gmres;
pub mod ilu;
pub mod math;
pub mod mpir;
pub mod pinn;
pub mod sparse;

#[cfg(test)]
mod tests_coverage;

pub use context::Context;
pub use error::{Result, SundialsError};

/// The real number type used throughout SUNDIALS.
/// Corresponds to `sunrealtype` in C (typically f64).
pub type Real = f64;

/// Boolean type (maps to `sunbooleantype`).
pub type SunBool = bool;

/// Index type for vector/matrix operations.
pub type SunIndex = i64;
