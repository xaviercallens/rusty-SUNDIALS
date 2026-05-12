//! Simple math utilities mirroring selected SUNDIALS C routines.
//!
//! This module is a numerically faithful Rust translation of selected C helpers,
//! preserving IEEE-754 behavior for comparisons and floating-point operations.

use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

/// SUNDIALS real scalar type (`sunrealtype`).
pub type SunReal = f64;

/// SUNDIALS index/integer type (`sunindextype`).
pub type SunIndex = usize;

/// SUNDIALS boolean type.
pub type SunBool = bool;

/// SUNDIALS boolean constant `SUNTRUE`.
pub const SUNTRUE: SunBool = true;
/// SUNDIALS boolean constant `SUNFALSE`.
pub const SUNFALSE: SunBool = false;

/// Unit roundoff for `f64` (`2^-53`), matching SUNDIALS convention.
pub const SUN_UNIT_ROUNDOFF: SunReal = f64::EPSILON / 2.0;

/// Large finite bound used in SUNDIALS-style scaling.
pub const SUN_BIG_REAL: SunReal = f64::MAX;

/// Error type for translated routines that can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// String-to-real conversion failed.
    ParseRealError { input: String },
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseRealError { input } => {
                write!(
                    f,
                    "failed to parse floating-point value from input: {input:?}"
                )
            }
        }
    }
}

impl std::error::Error for CvodeError {}

/// Integer power with C-loop semantics:
/// computes `base^exponent` for `exponent >= 1`, otherwise returns `1`.
///
/// Equivalent to:
/// `prod = 1; for (i=1; i<=exponent; i++) prod *= base;`
#[inline]
pub fn suni_power_i(base: i32, exponent: i32) -> i32 {
    if exponent < 1 {
        return 1;
    }
    (0..exponent).fold(1_i32, |acc, _| acc.wrapping_mul(base))
}

/// Real power with integer exponent, matching SUNDIALS behavior.
///
/// Computes:
/// - `prod = 1`
/// - multiply by `base` exactly `abs(exponent)` times
/// - if `exponent < 0`, return `1/prod`
#[inline]
pub fn sunr_power_i(base: SunReal, exponent: i32) -> SunReal {
    let expt = exponent.unsigned_abs();
    let prod = (0..expt).fold(1.0_f64, |acc, _| acc * base);
    if exponent < 0 { 1.0 / prod } else { prod }
}

/// Compare two reals using default tolerance `10 * SUN_UNIT_ROUNDOFF`.
///
/// Returns `SUNFALSE` when values are considered equal, `SUNTRUE` otherwise.
#[inline]
pub fn sunr_compare(a: SunReal, b: SunReal) -> SunBool {
    sunr_compare_tol(a, b, 10.0 * SUN_UNIT_ROUNDOFF)
}

/// Compare two reals with tolerance, preserving C/IEEE behavior.
///
/// Returns `SUNFALSE` if values are considered equal, `SUNTRUE` otherwise.
///
/// Logic:
/// 1. If `a == b`, return `SUNFALSE` (also handles `+inf == +inf`, `-inf == -inf`).
/// 2. `diff = |a - b|`
/// 3. `norm = min(|a + b|, SUN_BIG_REAL)`
/// 4. Return `!isless(diff, max(10*uroundoff, tol*norm))`
///
/// Using `partial_cmp`-style `isless` behavior ensures NaN handling analogous to C `isless`.
#[inline]
pub fn sunr_compare_tol(a: SunReal, b: SunReal, tol: SunReal) -> SunBool {
    if a == b {
        return SUNFALSE;
    }

    let diff = (a - b).abs();
    let norm = (a + b).abs().min(SUN_BIG_REAL);
    let rhs = (10.0 * SUN_UNIT_ROUNDOFF).max(tol * norm);

    // C: return !isless(diff, rhs);
    // isless(x,y) is false for NaN/unordered; preserve that behavior.
    !matches!(diff.partial_cmp(&rhs), Some(Ordering::Less))
}

/// Parse a string into `SunReal` (`f64`).
///
/// In C this uses `strtod/strtof/strtold` depending on precision.
/// Here, `sunrealtype -> f64`, so we use `f64::from_str`.
#[inline]
pub fn sun_str_to_real(s: &str) -> Result<SunReal, CvodeError> {
    f64::from_str(s).map_err(|_| CvodeError::ParseRealError {
        input: s.to_string(),
    })
}
