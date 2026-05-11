//! Rayon-parallel N_Vector implementation.
//!
//! Partitions the state array into chunks and dispatches operations across
//! all CPU threads using rayon.  This gives near-linear scaling with core
//! count for large systems (N ≫ 10 000).
//!
//! Data-race freedom is *structurally guaranteed* by Rust's ownership model:
//! `par_iter_mut` gives exclusive access to disjoint chunks, matching the
//! Separation Logic axioms in `proofs/lean4/nvector_parallel.lean`.

use rayon::prelude::*;
use sundials_core::Real;
use crate::traits::NVector;

/// A parallel vector backed by a contiguous `Vec<Real>`, distributed across
/// Rayon's global thread pool for element-wise operations.
#[derive(Debug, Clone)]
pub struct ParallelVector {
    data: Vec<Real>,
}

impl ParallelVector {
    pub fn new(len: usize) -> Self { Self { data: vec![0.0; len] } }

    pub fn from_slice(s: &[Real]) -> Self { Self { data: s.to_vec() } }

    pub fn filled(len: usize, val: Real) -> Self { Self { data: vec![val; len] } }
}

impl std::ops::Index<usize> for ParallelVector {
    type Output = Real;
    fn index(&self, i: usize) -> &Real { &self.data[i] }
}
impl std::ops::IndexMut<usize> for ParallelVector {
    fn index_mut(&mut self, i: usize) -> &mut Real { &mut self.data[i] }
}

impl NVector for ParallelVector {
    fn clone_empty(&self) -> Self { Self::new(self.data.len()) }
    fn len(&self) -> usize  { self.data.len() }
    fn as_slice(&self)  -> &[Real]      { &self.data }
    fn as_mut_slice(&mut self) -> &mut [Real] { &mut self.data }

    fn set_const(&mut self, c: Real) {
        self.data.par_iter_mut().for_each(|v| *v = c);
    }

    /// Parallel z = a·x + b·y using rayon zip.
    fn linear_sum(a: Real, x: &Self, b: Real, y: &Self, z: &mut Self) {
        z.data.par_iter_mut()
            .zip(x.data.par_iter())
            .zip(y.data.par_iter())
            .for_each(|((zi, xi), yi)| *zi = a * xi + b * yi);
    }

    /// Parallel WRMS norm — tree-reduce preserves associativity (see formal spec).
    fn wrms_norm(&self, w: &Self) -> Real {
        let n = self.data.len() as Real;
        let sum: Real = self.data.par_iter()
            .zip(w.data.par_iter())
            .map(|(xi, wi)| { let v = xi * wi; v * v })
            .sum();
        (sum / n).sqrt()
    }

    fn wrms_norm_mask(&self, w: &Self, mask: &Self) -> Real {
        let (sum, count) = self.data.par_iter()
            .zip(w.data.par_iter())
            .zip(mask.data.par_iter())
            .filter(|(_, mi)| **mi > 0.0)
            .map(|((xi, wi), _)| { let v = xi * wi; (v * v, 1usize) })
            .reduce(|| (0.0, 0), |(s1, c1), (s2, c2)| (s1 + s2, c1 + c2));
        if count == 0 { 0.0 } else { (sum / count as Real).sqrt() }
    }

    fn max_norm(&self) -> Real {
        self.data.par_iter()
            .map(|x| x.abs())
            .reduce(|| 0.0, f64::max)
    }

    fn min(&self) -> Real {
        self.data.par_iter().copied()
            .reduce(|| Real::INFINITY, Real::min)
    }

    fn dot(&self, other: &Self) -> Real {
        self.data.par_iter()
            .zip(other.data.par_iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    fn scale(c: Real, x: &Self, z: &mut Self) {
        z.data.par_iter_mut()
            .zip(x.data.par_iter())
            .for_each(|(zi, xi)| *zi = c * xi);
    }

    fn abs(x: &Self, z: &mut Self) {
        z.data.par_iter_mut()
            .zip(x.data.par_iter())
            .for_each(|(zi, xi)| *zi = xi.abs());
    }

    fn inv(x: &Self, z: &mut Self) {
        z.data.par_iter_mut()
            .zip(x.data.par_iter())
            .for_each(|(zi, xi)| *zi = 1.0 / xi);
    }

    fn prod(x: &Self, y: &Self, z: &mut Self) {
        z.data.par_iter_mut()
            .zip(x.data.par_iter())
            .zip(y.data.par_iter())
            .for_each(|((zi, xi), yi)| *zi = xi * yi);
    }

    fn div(x: &Self, y: &Self, z: &mut Self) {
        z.data.par_iter_mut()
            .zip(x.data.par_iter())
            .zip(y.data.par_iter())
            .for_each(|((zi, xi), yi)| *zi = xi / yi);
    }

    fn add_const(x: &Self, b: Real, z: &mut Self) {
        z.data.par_iter_mut()
            .zip(x.data.par_iter())
            .for_each(|(zi, xi)| *zi = xi + b);
    }

    fn constr_mask(c: &Self, x: &Self, m: &mut Self) -> bool {
        let violated: Vec<bool> = c.data.par_iter()
            .zip(x.data.par_iter())
            .map(|(ci, xi)| {
                (ci == &2.0 && xi <= &0.0)
                || (ci == &1.0 && xi <  &0.0)
                || (ci == &-1.0 && xi >  &0.0)
                || (ci == &-2.0 && xi >= &0.0)
            })
            .collect();

        let mut ok = true;
        for (i, &v) in violated.iter().enumerate() {
            m.data[i] = if v { 1.0 } else { 0.0 };
            if v { ok = false; }
        }
        ok
    }

    fn min_quotient(num: &Self, denom: &Self) -> Real {
        num.data.par_iter()
            .zip(denom.data.par_iter())
            .filter(|(_, d)| **d != 0.0)
            .map(|(n, d)| n / d)
            .reduce(|| Real::MAX, Real::min)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_linear_sum() {
        let n = 1024;
        let x = ParallelVector::filled(n, 1.0);
        let y = ParallelVector::filled(n, 2.0);
        let mut z = ParallelVector::new(n);
        ParallelVector::linear_sum(3.0, &x, 4.0, &y, &mut z);
        for v in z.as_slice() {
            assert!((v - 11.0).abs() < 1e-14, "got {v}");
        }
    }

    #[test]
    fn test_parallel_dot() {
        let n = 1024;
        let x = ParallelVector::filled(n, 2.0);
        let y = ParallelVector::filled(n, 3.0);
        assert!((x.dot(&y) - 6.0 * n as f64).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_wrms_norm() {
        let v = ParallelVector::from_slice(&[3.0, 4.0]);
        let w = ParallelVector::from_slice(&[1.0, 1.0]);
        let expected = ((9.0 + 16.0) / 2.0f64).sqrt();
        assert!((v.wrms_norm(&w) - expected).abs() < 1e-14);
    }
}
