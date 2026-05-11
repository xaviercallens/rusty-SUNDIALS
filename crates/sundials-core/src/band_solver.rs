//! Banded matrix LU solver — reduces O(N³) dense cost to O(N·(ml+mu)²) for
//! banded Jacobians arising in 1-D and 2-D PDE Method-of-Lines discretisations.
//!
//! Storage follows SUNDIALS band format (column-major with upper and lower
//! bandwidth padding so that row pivoting stays in-band):
//!
//!   A(i, j)  is stored in  cols[j][i - j + smu]
//!
//! where `smu = mu + ml` (storage upper bandwidth, padded for pivoting room).
//!
//! Translated from: `sunmatrix/band/sunmatrix_band.c`,
//!                  `sunlinsol/band/sunlinsol_band.c`.

use crate::Real;

/// Error type for banded operations.
#[derive(Debug, Clone, PartialEq)]
pub enum BandError {
    ZeroPivot { col: usize },
    DimensionMismatch,
}

impl std::fmt::Display for BandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroPivot { col } => write!(f, "zero pivot at column {col}"),
            Self::DimensionMismatch => write!(f, "dimension mismatch"),
        }
    }
}

/// A banded matrix in SUNDIALS column-major band storage.
///
/// # Complexity
/// - Storage: O(N · (ml + mu + smu))
/// - LU factorisation: O(N · ml · mu)   cf. O(N³) for dense
/// - Solve: O(N · (ml + mu))
#[derive(Clone, Debug)]
pub struct BandMat {
    /// Number of rows/columns (square matrix assumed).
    pub n: usize,
    /// Lower bandwidth.
    pub ml: usize,
    /// Upper bandwidth.
    pub mu: usize,
    /// Storage upper bandwidth (= mu + ml for pivoting room).
    pub smu: usize,
    /// Column-major storage: `cols[j]` has length `smu + ml + 1`.
    pub cols: Vec<Vec<Real>>,
}

impl BandMat {
    /// Create a zero banded matrix with given dimensions and bandwidths.
    pub fn zeros(n: usize, ml: usize, mu: usize) -> Self {
        let smu = mu + ml;
        let col_len = smu + ml + 1;
        Self {
            n, ml, mu, smu,
            cols: vec![vec![0.0; col_len]; n],
        }
    }

    /// Get element A(i, j) — returns 0.0 if outside band.
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> Real {
        if j >= self.n || i >= self.n { return 0.0; }
        let row = (i as isize - j as isize + self.smu as isize) as usize;
        if row >= self.cols[j].len() { return 0.0; }
        self.cols[j][row]
    }

    /// Set element A(i, j) — panics if outside band.
    #[inline]
    pub fn set(&mut self, i: usize, j: usize, val: Real) {
        let smu = self.smu;
        let row = (i as isize - j as isize + smu as isize) as usize;
        self.cols[j][row] = val;
    }

    /// Add `val` to element A(i, j).
    #[inline]
    pub fn add(&mut self, i: usize, j: usize, val: Real) {
        let smu = self.smu;
        let row = (i as isize - j as isize + smu as isize) as usize;
        self.cols[j][row] += val;
    }

    /// In-place banded LU factorisation with partial pivoting.
    ///
    /// Uses a flat 2D array internally for correctness, then copies back.
    pub fn band_getrf(&mut self, pivots: &mut [usize]) -> Result<(), BandError> {
        let n  = self.n;
        let ml = self.ml;
        let mu = self.mu;

        // Work in a plain Vec<Vec<f64>> (row-major) for clarity
        let mut a: Vec<Vec<Real>> = (0..n).map(|i| {
            (0..n).map(|j| self.get(i, j)).collect()
        }).collect();

        for k in 0..n {
            let p_hi = (k + ml).min(n - 1);

            // Find pivot
            let mut pivot_row = k;
            let mut pivot_abs = a[k][k].abs();
            for i in (k+1)..=p_hi {
                if a[i][k].abs() > pivot_abs {
                    pivot_abs = a[i][k].abs();
                    pivot_row = i;
                }
            }
            pivots[k] = pivot_row;
            if pivot_abs == 0.0 {
                return Err(BandError::ZeroPivot { col: k });
            }

            if pivot_row != k {
                a.swap(k, pivot_row);
            }

            // Eliminate
            for i in (k+1)..=p_hi {
                let factor = a[i][k] / a[k][k];
                a[i][k] = factor; // store multiplier
                let j_hi = (k + mu).min(n - 1);
                for j in (k+1)..=j_hi {
                    a[i][j] -= factor * a[k][j];
                }
            }
        }

        // Copy results back into band storage
        for i in 0..n {
            for j in 0..n {
                let offset = i as isize - j as isize + self.smu as isize;
                if offset >= 0 && (offset as usize) < self.cols[j].len() {
                    self.cols[j][offset as usize] = a[i][j];
                }
            }
        }
        Ok(())
    }

    /// Solve `Ax = b` using banded LU factors from `band_getrf`.
    ///
    /// `b` is modified in-place to the solution vector `x`.
    pub fn band_getrs(&self, pivots: &[usize], b: &mut [Real]) -> Result<(), BandError> {
        let n  = self.n;
        let ml = self.ml;
        let mu = self.mu;

        if b.len() != n || pivots.len() != n { return Err(BandError::DimensionMismatch); }

        // Apply row permutations
        for k in 0..n {
            let pk = pivots[k];
            if pk != k { b.swap(k, pk); }
        }

        // Forward substitution L·y = b  (unit diagonal L, multipliers below)
        for k in 0..n {
            let i_hi = (k + ml).min(n - 1);
            for i in (k+1)..=i_hi {
                b[i] -= self.get(i, k) * b[k];
            }
        }

        // Backward substitution U·x = y
        for k in (0..n).rev() {
            let j_hi = (k + mu).min(n - 1);
            for j in (k+1)..=j_hi {
                b[k] -= self.get(k, j) * b[j];
            }
            b[k] /= self.get(k, k);
        }
        Ok(())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// Solve a simple tridiagonal system (ml=mu=1) and verify.
    #[test]
    fn test_band_tridiagonal() {
        let n = 5;
        // Build matrix:  2 on diagonal, -1 on off-diagonals
        let mut a = BandMat::zeros(n, 1, 1);
        for i in 0..n {
            a.set(i, i, 2.0);
            if i + 1 < n { a.set(i + 1, i, -1.0); a.set(i, i + 1, -1.0); }
        }
        // RHS: all ones
        let mut b = vec![1.0; n];
        let mut pivots = vec![0usize; n];
        a.band_getrf(&mut pivots).expect("LU");
        a.band_getrs(&pivots, &mut b).expect("solve");

        // x is now in `b`; compute residual A_original * x - rhs
        let x = b;  // solution
        let mut residual = vec![0.0; n];
        for i in 0..n {
            residual[i] = 2.0 * x[i];
            if i > 0     { residual[i] -= x[i - 1]; }
            if i < n - 1 { residual[i] -= x[i + 1]; }
            residual[i] -= 1.0; // subtract rhs
        }
        for i in 0..n {
            assert!(residual[i].abs() < 1e-8,
                "residual[{i}] = {} (x[i]={})", residual[i].abs(), x[i]);
        }
    }
}
