//! N_Vector trait — defines the vector operations interface.
//!
//! This trait replaces the C function-pointer table (`N_Vector_Ops`)
//! with a Rust trait, enabling static dispatch and zero-cost abstraction.

use sundials_core::Real;

/// The core vector operations trait.
///
/// Any type implementing `NVector` can be used with CVODE and other solvers.
/// This maps to the `N_Vector_Ops` function table in C SUNDIALS.
pub trait NVector: Clone {
    /// Create a new vector of the same size, initialized to zero.
    fn clone_empty(&self) -> Self;

    /// Get the length of the vector.
    fn len(&self) -> usize;

    /// Check if the vector is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get immutable access to the underlying data.
    fn as_slice(&self) -> &[Real];

    /// Get mutable access to the underlying data.
    fn as_mut_slice(&mut self) -> &mut [Real];

    /// Set all elements to a constant value.
    fn set_const(&mut self, c: Real);

    /// z = x + y (element-wise addition).
    fn linear_sum(a: Real, x: &Self, b: Real, y: &Self, z: &mut Self);

    /// Compute the weighted RMS norm: sqrt(sum((x[i]*w[i])^2) / n).
    fn wrms_norm(&self, w: &Self) -> Real;

    /// Compute the weighted RMS norm with a mask.
    fn wrms_norm_mask(&self, w: &Self, mask: &Self) -> Real;

    /// Compute the max norm: max(|x[i]|).
    fn max_norm(&self) -> Real;

    /// Compute the minimum element.
    fn min(&self) -> Real;

    /// Compute the dot product: sum(x[i] * y[i]).
    fn dot(&self, other: &Self) -> Real;

    /// Scale: z[i] = c * x[i].
    fn scale(c: Real, x: &Self, z: &mut Self);

    /// Element-wise absolute value: z[i] = |x[i]|.
    fn abs(x: &Self, z: &mut Self);

    /// Element-wise inverse: z[i] = 1/x[i].
    fn inv(x: &Self, z: &mut Self);

    /// Element-wise product: z[i] = x[i] * y[i].
    fn prod(x: &Self, y: &Self, z: &mut Self);

    /// Element-wise division: z[i] = x[i] / y[i].
    fn div(x: &Self, y: &Self, z: &mut Self);

    /// Add a constant: z[i] = x[i] + b.
    fn add_const(x: &Self, b: Real, z: &mut Self);

    /// Check if any element satisfies a constraint violation.
    /// Returns true if all constraints are satisfied.
    fn constr_mask(c: &Self, x: &Self, m: &mut Self) -> bool;

    /// Compute the minimum quotient: min(num[i]/denom[i]) where denom[i] != 0.
    fn min_quotient(num: &Self, denom: &Self) -> Real;
}
