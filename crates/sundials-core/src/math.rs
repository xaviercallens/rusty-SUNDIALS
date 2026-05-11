//! Mathematical utility functions.
//!
//! Translated from: `sundials/sundials_math.c`

use crate::Real;

/// Machine epsilon for Real (f64).
pub const UNIT_ROUNDOFF: Real = f64::EPSILON;

/// Minimum positive Real value.
pub const TINY: Real = f64::MIN_POSITIVE;

/// Maximum Real value.
pub const BIG: Real = f64::MAX;

/// Compute the absolute value.
#[inline]
pub fn abs(x: Real) -> Real {
    x.abs()
}

/// Compute the square root (returns 0 for negative inputs).
#[inline]
pub fn sqrt(x: Real) -> Real {
    if x <= 0.0 { 0.0 } else { x.sqrt() }
}

/// Compute x raised to the power n (integer exponent).
#[inline]
pub fn powi(x: Real, n: i32) -> Real {
    x.powi(n)
}

/// Return the minimum of two values.
#[inline]
pub fn min(a: Real, b: Real) -> Real {
    a.min(b)
}

/// Return the maximum of two values.
#[inline]
pub fn max(a: Real, b: Real) -> Real {
    a.max(b)
}

/// Constrain x to the interval [lo, hi].
#[inline]
pub fn clamp(x: Real, lo: Real, hi: Real) -> Real {
    x.clamp(lo, hi)
}

/// Compute the RMS norm of a vector: sqrt(sum(v[i]^2 * w[i]^2) / n).
pub fn wrms_norm(v: &[Real], w: &[Real]) -> Real {
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    let sum: Real = v.iter().zip(w.iter()).map(|(vi, wi)| (vi * wi).powi(2)).sum();
    (sum / n as Real).sqrt()
}

/// Compute the weighted L2 norm: sqrt(sum(v[i]^2 * w[i]^2)).
pub fn wl2_norm(v: &[Real], w: &[Real]) -> Real {
    let sum: Real = v.iter().zip(w.iter()).map(|(vi, wi)| (vi * wi).powi(2)).sum();
    sum.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrms_norm() {
        let v = [1.0, 2.0, 3.0];
        let w = [1.0, 1.0, 1.0];
        let norm = wrms_norm(&v, &w);
        let expected = ((1.0 + 4.0 + 9.0) / 3.0_f64).sqrt();
        assert!((norm - expected).abs() < 1e-15);
    }

    #[test]
    fn test_sqrt_negative() {
        assert_eq!(sqrt(-1.0), 0.0);
        assert_eq!(sqrt(4.0), 2.0);
    }
}
