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

    /// Rescale the Nordsieck array with interpolation when the step size change is large.
    ///
    /// When `η = h_new / h` is far from 1.0, simple rescaling is inaccurate.
    /// This method interpolates the polynomial to the new scaled time points to
    /// maintain numerical correctness and accuracy for high-order methods.
    ///
    /// The interpolating transformation matrix is upper triangular with elements:
    ///   $T_{i,j} = \binom{j}{i} \eta^i$  for $j \ge i$
    ///
    /// Corresponds to `CVAdjustNordsieck` in SUNDIALS CVODE.
    pub fn rescale_with_interpolation(&mut self, eta: Real, order: usize) {
        let n = self.n;
        // Scratch buffer for the new scaled array
        let mut new_z = vec![vec![0.0; n]; order + 1];

        // Compute combinations C(j, i) dynamically or via simple loops.
        // For Nordsieck size q <= 5, Pascal's triangle is trivial.
        let mut binom = [[0.0; NORDSIECK_SIZE]; NORDSIECK_SIZE];
        for j in 0..NORDSIECK_SIZE {
            binom[j][0] = 1.0;
            for i in 1..=j {
                binom[j][i] = binom[j - 1][i - 1] + binom[j - 1][i];
            }
        }

        // T_{i,j} = C(j, i) * η^i
        let mut factor = eta;
        for i in 1..=order {
            for j in i..=order {
                let coeff = binom[j][i] * factor;
                let z_j = self.z[j].as_slice();
                for k in 0..n {
                    new_z[i][k] += coeff * z_j[k];
                }
            }
            factor *= eta;
        }

        // Apply back to self (z_0 remains unchanged)
        for i in 1..=order {
            self.z[i].as_mut_slice().copy_from_slice(&new_z[i]);
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

    /// Dense output: evaluate the k-th derivative of the interpolating polynomial
    /// at time `t = t_n + s`, where `s` is the offset from the current step.
    ///
    /// The Nordsieck array at time $t_n$ stores:
    ///   $z_i = \frac{h^i}{i!} y^{(i)}(t_n)$
    ///
    /// The k-th derivative at $t_n + s$ is given by the shifted Taylor polynomial:
    ///   $\frac{d^k y}{dt^k}(t_n + s) = \sum_{j=k}^{q} \binom{j}{k} \frac{s^{j-k}}{h^j} \cdot j! \cdot z_j / h^0$
    ///
    /// For k=0 (the value), this simplifies to Horner evaluation of the Nordsieck
    /// polynomial in the normalised variable $\sigma = s / h$.
    ///
    /// # Arguments
    /// * `s` — offset from `t_n` (must satisfy `0 ≤ s ≤ h`)
    /// * `h` — current step size
    /// * `order` — current method order `q`
    /// * `k` — derivative order (0 = value, 1 = first derivative, ...)
    /// * `result` — output vector (must have length `n`)
    ///
    /// # Reference
    /// Hindmarsh & Serban (2005), *CVODE User Guide*, §4.5.8 — `CVodeGetDky`
    pub fn get_dky(&self, s: Real, h: Real, order: usize, k: usize, result: &mut [Real]) {
        let n = self.n;
        debug_assert!(
            k <= order,
            "derivative order k={k} exceeds method order q={order}"
        );
        debug_assert!(result.len() == n);

        let sigma = if h.abs() > 0.0 { s / h } else { 0.0 };

        // Horner evaluation of the shifted Nordsieck polynomial for derivative k.
        // For k=0: y(t_n + s) = z[q]*σ^q + z[q-1]*σ^(q-1) + ... + z[0]
        //        = ((z[q]*σ + z[q-1])*σ + z[q-2])*σ + ... + z[0]
        //
        // For k>0: we need the k-th derivative of the polynomial.
        // The coefficient of z[j] in the k-th derivative is:
        //   C(j,k) * j!/(j-k)! * σ^(j-k) / h^k
        // Using Horner scheme starting from z[q].

        // Start: result = z[order] (scaled by the leading combinatorial factor)
        let z_q = self.z[order].as_slice();
        for i in 0..n {
            result[i] = z_q[i];
        }

        // Horner-like evaluation
        for j in (k..order).rev() {
            // Combinatorial factor for derivative k:
            // For k=0, we just multiply by σ and add z[j]
            // For k>0, factor in the falling factorial
            let c = if k == 0 {
                sigma
            } else {
                sigma * ((j + 1) as Real) / ((j + 1 - k) as Real)
            };
            let z_j = self.z[j].as_slice();
            for i in 0..n {
                result[i] = result[i] * c + z_j[i];
            }
        }

        // Scale by 1/h^k for the k-th derivative (convert from Nordsieck scaling)
        if k > 0 && h.abs() > 0.0 {
            let h_inv_k = h.powi(-(k as i32));
            // Multiply by k! (the Nordsieck array stores h^k/k! * y^(k))
            let k_factorial: Real = (1..=k).map(|i| i as Real).product();
            let scale = h_inv_k * k_factorial;
            for i in 0..n {
                result[i] *= scale;
            }
        }
    }
}
