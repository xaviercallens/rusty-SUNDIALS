//! N_Vector abstraction for rusty-SUNDIALS.
//!
//! Provides the `NVector` trait and three implementations:
//! - `SerialVector`   — baseline scalar, single-threaded
//! - `SimdVector`     — chunk-based auto-vectorisation (NEON / AVX)
//! - `ParallelVector` — rayon multi-threaded, data-race-free
//!
//! Translated from: `sundials/sundials_nvector.h`, `nvector/nvector_serial.c`

mod parallel;
mod serial;
mod simd;
mod traits;

pub use parallel::ParallelVector;
pub use serial::SerialVector;
pub use simd::SimdVector;
pub use traits::NVector;
