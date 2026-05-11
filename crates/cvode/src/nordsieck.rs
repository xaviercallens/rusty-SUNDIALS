//! Nordsieck history array — stores the polynomial representation of the solution.
//!
//! The Nordsieck array z[i] stores h^i/i! * y^(i) for i = 0, ..., q
//! where q is the current method order and h is the step size.
//!
//! Translated from: cvode.c (Nordsieck array management)

use nvector::{NVector, SerialVector};
use sundials_core::Real;

use crate::constants::NORDSIECK_SIZE;

/// Nordsieck history array for the multistep method.
///
/// Stores the scaled derivatives of the solution polynomial.
/// z[0] = y_n (current solution)
/// z[i] = h^i/i! * y_n^(i) for i = 1, ..., q
pub(crate) struct NordsieckArray {
    /// The array of vectors: z[0..=q].
    pub z: Vec<SerialVector>,
    /// Problem dimension.
    pub n: usize,
}

impl NordsieckArray {
    /// Create a new Nordsieck array for a problem of dimension `n`.
    pub fn new(n: usize) -> Self {
        let z = (0..NORDSIECK_SIZE).map(|_| SerialVector::new(n)).collect();
        Self { z, n }
    }

    /// Get the current solution (z[0]).
    pub fn solution(&self) -> &SerialVector {
        &self.z[0]
    }

    /// Get a mutable reference to z[k].
    pub fn get_mut(&mut self, k: usize) -> &mut SerialVector {
        &mut self.z[k]
    }

    /// Get a reference to z[k].
    pub fn get(&self, k: usize) -> &SerialVector {
        &self.z[k]
    }

    /// Rescale the Nordsieck array when step size changes.
    ///
    /// When h changes to h_new = eta * h, we rescale:
    ///   z[i] *= eta^i
    pub fn rescale(&mut self, eta: Real, order: usize) {
        let mut factor = eta;
        for i in 1..=order {
            let z_i = &mut self.z[i];
            let data = z_i.as_mut_slice();
            for val in data.iter_mut() {
                *val *= factor;
            }
            factor *= eta;
        }
    }

    /// Apply the predictor: compute y_predicted from the Nordsieck array.
    ///
    /// y_pred = sum_{i=0}^{q} z[i]  (Pascal's triangle evaluation)
    pub fn predict(&self, order: usize, result: &mut SerialVector) {
        // Start with z[order] and work backwards (Horner-like)
        let data = result.as_mut_slice();
        let z_q = self.z[order].as_slice();
        data.copy_from_slice(z_q);

        for i in (0..order).rev() {
            let z_i = self.z[i].as_slice();
            for j in 0..self.n {
                data[j] += z_i[j];
            }
        }
    }

    /// Update the Nordsieck array after a successful step.
    ///
    /// z[i] += l[i] * delta_correction for i = 0, ..., q
    pub fn correct(&mut self, l: &[Real], correction: &SerialVector, order: usize) {
        let corr = correction.as_slice();
        for i in 0..=order {
            let z_i = self.z[i].as_mut_slice();
            let li = l[i];
            for j in 0..self.n {
                z_i[j] += li * corr[j];
            }
        }
    }
}
