//! N_Vector abstraction for rusty-SUNDIALS.
//!
//! Provides the `NVector` trait (equivalent to `N_Vector` operations in C)
//! and a serial (dense) implementation.
//!
//! Translated from: `sundials/sundials_nvector.h`, `nvector/nvector_serial.c`

mod serial;
mod traits;

pub use serial::SerialVector;
pub use traits::NVector;
