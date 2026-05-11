//! Serial (dense) N_Vector implementation.
//!
//! Translated from: `nvector/nvector_serial.c`
//! This is the simplest N_Vector: a contiguous array of Real values.

use sundials_core::Real;

use crate::traits::NVector;

/// A serial (dense) vector stored as a contiguous `Vec<Real>`.
///
/// This is the Rust equivalent of `N_VNew_Serial` from SUNDIALS.
/// Suitable for single-threaded, non-distributed computations.
#[derive(Debug, Clone)]
pub struct SerialVector {
    data: Vec<Real>,
}

impl SerialVector {
    /// Create a new vector of given length, initialized to zero.
    pub fn new(len: usize) -> Self {
        Self { data: vec![0.0; len] }
    }

    /// Create a vector from a slice.
    pub fn from_slice(s: &[Real]) -> Self {
        Self { data: s.to_vec() }
    }

    /// Create a vector filled with a constant value.
    pub fn filled(len: usize, val: Real) -> Self {
        Self { data: vec![val; len] }
    }
}

impl std::ops::Index<usize> for SerialVector {
    type Output = Real;
    fn index(&self, i: usize) -> &Real {
        &self.data[i]
    }
}

impl std::ops::IndexMut<usize> for SerialVector {
    fn index_mut(&mut self, i: usize) -> &mut Real {
        &mut self.data[i]
    }
}

impl NVector for SerialVector {
    fn clone_empty(&self) -> Self {
        Self::new(self.data.len())
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn as_slice(&self) -> &[Real] {
        &self.data
    }

    fn as_mut_slice(&mut self) -> &mut [Real] {
        &mut self.data
    }

    fn set_const(&mut self, c: Real) {
        self.data.fill(c);
    }

    fn linear_sum(a: Real, x: &Self, b: Real, y: &Self, z: &mut Self) {
        for i in 0..z.data.len() {
            z.data[i] = a * x.data[i] + b * y.data[i];
        }
    }

    fn wrms_norm(&self, w: &Self) -> Real {
        let n = self.data.len() as Real;
        let sum: Real = self.data.iter().zip(w.data.iter())
            .map(|(xi, wi)| (xi * wi).powi(2))
            .sum();
        (sum / n).sqrt()
    }

    fn wrms_norm_mask(&self, w: &Self, mask: &Self) -> Real {
        let mut sum = 0.0;
        let mut count = 0;
        for i in 0..self.data.len() {
            if mask.data[i] > 0.0 {
                sum += (self.data[i] * w.data[i]).powi(2);
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { (sum / count as Real).sqrt() }
    }

    fn max_norm(&self) -> Real {
        self.data.iter().map(|x| x.abs()).fold(0.0, Real::max)
    }

    fn min(&self) -> Real {
        self.data.iter().copied().fold(Real::INFINITY, Real::min)
    }

    fn dot(&self, other: &Self) -> Real {
        self.data.iter().zip(other.data.iter()).map(|(a, b)| a * b).sum()
    }

    fn scale(c: Real, x: &Self, z: &mut Self) {
        for i in 0..z.data.len() {
            z.data[i] = c * x.data[i];
        }
    }

    fn abs(x: &Self, z: &mut Self) {
        for i in 0..z.data.len() {
            z.data[i] = x.data[i].abs();
        }
    }

    fn inv(x: &Self, z: &mut Self) {
        for i in 0..z.data.len() {
            z.data[i] = 1.0 / x.data[i];
        }
    }

    fn prod(x: &Self, y: &Self, z: &mut Self) {
        for i in 0..z.data.len() {
            z.data[i] = x.data[i] * y.data[i];
        }
    }

    fn div(x: &Self, y: &Self, z: &mut Self) {
        for i in 0..z.data.len() {
            z.data[i] = x.data[i] / y.data[i];
        }
    }

    fn add_const(x: &Self, b: Real, z: &mut Self) {
        for i in 0..z.data.len() {
            z.data[i] = x.data[i] + b;
        }
    }

    fn constr_mask(c: &Self, x: &Self, m: &mut Self) -> bool {
        let mut all_ok = true;
        for i in 0..x.data.len() {
            let ci = c.data[i];
            let xi = x.data[i];
            let violated = (ci == 2.0 && xi <= 0.0)
                || (ci == 1.0 && xi < 0.0)
                || (ci == -1.0 && xi > 0.0)
                || (ci == -2.0 && xi >= 0.0);
            m.data[i] = if violated { 1.0 } else { 0.0 };
            if violated { all_ok = false; }
        }
        all_ok
    }

    fn min_quotient(num: &Self, denom: &Self) -> Real {
        let mut min_q = Real::MAX;
        for i in 0..num.data.len() {
            if denom.data[i] != 0.0 {
                let q = num.data[i] / denom.data[i];
                if q < min_q { min_q = q; }
            }
        }
        min_q
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_sum() {
        let x = SerialVector::from_slice(&[1.0, 2.0, 3.0]);
        let y = SerialVector::from_slice(&[4.0, 5.0, 6.0]);
        let mut z = SerialVector::new(3);
        SerialVector::linear_sum(2.0, &x, 3.0, &y, &mut z);
        assert_eq!(z.as_slice(), &[14.0, 19.0, 24.0]);
    }

    #[test]
    fn test_wrms_norm() {
        let v = SerialVector::from_slice(&[3.0, 4.0]);
        let w = SerialVector::from_slice(&[1.0, 1.0]);
        let norm = v.wrms_norm(&w);
        let expected = ((9.0 + 16.0) / 2.0_f64).sqrt();
        assert!((norm - expected).abs() < 1e-15);
    }

    #[test]
    fn test_dot_product() {
        let x = SerialVector::from_slice(&[1.0, 2.0, 3.0]);
        let y = SerialVector::from_slice(&[4.0, 5.0, 6.0]);
        assert_eq!(x.dot(&y), 32.0);
    }
}
