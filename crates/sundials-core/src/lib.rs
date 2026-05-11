//! Core types for rusty-SUNDIALS.
//!
//! Provides the fundamental numeric types, error handling, and context
//! management used across all SUNDIALS components.
//!
//! Translated from: `sundials/sundials_types.h`, `sundials/sundials_context.h`

pub mod context;
pub mod error;
pub mod math;
pub mod generated;
pub mod band_solver;
pub mod gmres;


pub use context::Context;
pub use error::{SundialsError, Result};

/// The real number type used throughout SUNDIALS.
/// Corresponds to `sunrealtype` in C (typically f64).
pub type Real = f64;

/// Boolean type (maps to `sunbooleantype`).
pub type SunBool = bool;

/// Index type for vector/matrix operations.
pub type SunIndex = i64;
