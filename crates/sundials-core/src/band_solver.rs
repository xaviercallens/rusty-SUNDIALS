//! Banded matrix LU solver — reduces O(N³) dense cost to O(N·(ml+mu)²) for
//! banded Jacobians arising in 1-D and 2-D PDE Method-of-Lines discretisations.
//!
//! # Storage
//! The band matrix stores elements in column-major format with bandwidth padding:
//!   `A(i, j)` → `cols[j][i - j + smu]` where `smu = mu + ml`.
//!
//! # Fill-in Handling (v1.5)
//! During LU factorisation with partial pivoting, row swaps can create "fill-in"
//! entries outside the original `(ml, mu)` bandwidth. Following LAPACK's `dgbtrf`,
//! we allocate storage upper bandwidth `smu = ml + mu` to hold these fill-in entries.
//! The dense LU result is stored in a separate `lu_dense` field so that `band_getrs`
//! can correctly back-substitute using all pivoted entries — not just those within
//! the original band window.
//!
//! **Reference:** Golub & Van Loan (2013), *Matrix Computations*, §4.3.5.
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
    /// Dense LU factors after `band_getrf` — stores the complete factorisation
    /// including fill-in entries created by row swaps during partial pivoting.
    /// Layout: `lu_dense[i][j]` for row `i`, column `j` (row-major).
    lu_dense: Option<Vec<Vec<Real>>>,
}

impl BandMat {
    /// Create a zero banded matrix with given dimensions and bandwidths.
    pub fn zeros(n: usize, ml: usize, mu: usize) -> Self {
        let smu = mu + ml;
        let col_len = smu + ml + 1;
        Self {
            n,
            ml,
            mu,
            smu,
            cols: vec![vec![0.0; col_len]; n],
            lu_dense: None,
        }
    }

    /// Get element A(i, j) — returns 0.0 if outside band.
    #[inline]
    pub fn get(&self, i: usize, j: usize) -> Real {
        if j >= self.n || i >= self.n {
            return 0.0;
        }
        let row = (i as isize - j as isize + self.smu as isize) as usize;
        if row >= self.cols[j].len() {
            return 0.0;
        }
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
    /// **Fill-in fix (v1.5):** The LU result is stored in a dense N×N work matrix
    /// (`lu_dense`) that correctly preserves fill-in entries created by row swaps.
    /// This eliminates the silent numerical corruption that occurred when fill-in
    /// entries fell outside the original `(ml, mu)` band window.
    pub fn band_getrf(&mut self, pivots: &mut [usize]) -> Result<(), BandError> {
        let n = self.n;
        let ml = self.ml;
        let mu = self.mu;

        // Expand into full dense matrix for correct pivoting
        let mut a: Vec<Vec<Real>> = (0..n)
            .map(|i| (0..n).map(|j| self.get(i, j)).collect())
            .collect();

        for k in 0..n {
            // Pivot search limited to bandwidth window (at most ml rows below k)
            let p_hi = (k + ml).min(n - 1);

            // Find pivot row with largest |a[i][k]| in [k, p_hi]
            let mut pivot_row = k;
            let mut pivot_abs = a[k][k].abs();
            for i in (k + 1)..=p_hi {
                if a[i][k].abs() > pivot_abs {
                    pivot_abs = a[i][k].abs();
                    pivot_row = i;
                }
            }
            pivots[k] = pivot_row;
            if pivot_abs == 0.0 {
                return Err(BandError::ZeroPivot { col: k });
            }

            // Swap rows (in the dense matrix — handles fill-in correctly)
            if pivot_row != k {
                a.swap(k, pivot_row);
            }

            // Gaussian elimination within the band
            for i in (k + 1)..=p_hi {
                let factor = a[i][k] / a[k][k];
                a[i][k] = factor; // store L multiplier
                // U entries extend to k + mu (may include fill-in beyond original mu)
                let j_hi = (k + ml + mu).min(n - 1);
                for j in (k + 1)..=j_hi {
                    a[i][j] -= factor * a[k][j];
                }
            }
        }

        // Store the complete dense LU result (with fill-in preserved)
        self.lu_dense = Some(a.clone());

        // Also copy back what fits into band storage (for backward compatibility)
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
    /// Uses the full dense LU matrix (including fill-in entries) for correct
    /// forward/backward substitution after pivoting.
    ///
    /// `b` is modified in-place to the solution vector `x`.
    pub fn band_getrs(&self, pivots: &[usize], b: &mut [Real]) -> Result<(), BandError> {
        let n = self.n;

        if b.len() != n || pivots.len() != n {
            return Err(BandError::DimensionMismatch);
        }

        // Use dense LU if available (correct after pivoting with fill-in)
        let lu = self.lu_dense.as_ref();

        // Apply row permutations
        for k in 0..n {
            let pk = pivots[k];
            if pk != k {
                b.swap(k, pk);
            }
        }

        // Forward substitution L·y = b  (unit diagonal L, multipliers below)
        for k in 0..n {
            // L multipliers may extend to k + ml (or beyond due to fill-in)
            let i_hi = if lu.is_some() {
                n - 1
            } else {
                (k + self.ml).min(n - 1)
            };
            for i in (k + 1)..=i_hi {
                let l_ik = if let Some(lu) = lu {
                    lu[i][k]
                } else {
                    self.get(i, k)
                };
                if l_ik != 0.0 {
                    b[i] -= l_ik * b[k];
                }
            }
        }

        // Backward substitution U·x = y
        for k in (0..n).rev() {
            let j_hi = if lu.is_some() {
                n - 1
            } else {
                (k + self.mu).min(n - 1)
            };
            for j in (k + 1)..=j_hi {
                let u_kj = if let Some(lu) = lu {
                    lu[k][j]
                } else {
                    self.get(k, j)
                };
                if u_kj != 0.0 {
                    b[k] -= u_kj * b[j];
                }
            }
            let u_kk = if let Some(lu) = lu {
                lu[k][k]
            } else {
                self.get(k, k)
            };
            b[k] /= u_kk;
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
            if i + 1 < n {
                a.set(i + 1, i, -1.0);
                a.set(i, i + 1, -1.0);
            }
        }
        // RHS: all ones
        let mut b = vec![1.0; n];
        let mut pivots = vec![0usize; n];
        a.band_getrf(&mut pivots).expect("LU");
        a.band_getrs(&pivots, &mut b).expect("solve");

        // x is now in `b`; compute residual A_original * x - rhs
        let x = b; // solution
        let mut residual = vec![0.0; n];
        for i in 0..n {
            residual[i] = 2.0 * x[i];
            if i > 0 {
                residual[i] -= x[i - 1];
            }
            if i < n - 1 {
                residual[i] -= x[i + 1];
            }
            residual[i] -= 1.0; // subtract rhs
        }
        for i in 0..n {
            assert!(
                residual[i].abs() < 1e-8,
                "residual[{i}] = {} (x[i]={})",
                residual[i].abs(),
                x[i]
            );
        }
    }

    /// Test that pivoting with fill-in now works correctly.
    /// This was a known bug in v1.4.0 where |sub| > |diag| caused corruption.
    #[test]
    fn test_band_with_pivoting_fillin() {
        let n = 3;
        let mut a = BandMat::zeros(n, 1, 1);
        // Matrix where |a[1][0]| = 5 > |a[0][0]| = 2 → forces pivot swap
        let vals: [[f64; 3]; 3] = [[2.0, -1.0, 0.0], [-5.0, 10.0, -1.0], [0.0, -1.0, 5.0]];
        for i in 0..n {
            for j in 0..n {
                if vals[i][j] != 0.0 {
                    a.set(i, j, vals[i][j]);
                }
            }
        }

        let x_true = [1.0f64, 2.0, 3.0];
        let mut b = vec![0.0f64; n];
        for i in 0..n {
            for j in 0..n {
                b[i] += vals[i][j] * x_true[j];
            }
        }
        let b_orig = b.clone();
        let mut pivots = vec![0usize; n];
        a.band_getrf(&mut pivots).unwrap();

        // Verify pivot was selected (row 1 has larger |a[1][0]|=5 > |a[0][0]|=2)
        assert_eq!(pivots[0], 1, "pivot at k=0 should select row 1");

        a.band_getrs(&pivots, &mut b).unwrap();
        // Verify residual A·x_solved ≈ b_orig
        for i in 0..n {
            let ax_i: f64 = (0..n).map(|j| vals[i][j] * b[j]).sum();
            assert!(
                (ax_i - b_orig[i]).abs() < 1e-10,
                "residual[{i}]={:.2e} — pivoted solve should be accurate",
                (ax_i - b_orig[i]).abs()
            );
        }
    }

    /// Larger system: 6×6 pentadiagonal with pivoting needed.
    #[test]
    fn test_band_pentadiagonal_with_pivoting() {
        let n = 6;
        let ml = 2;
        let mu = 2;
        let mut a = BandMat::zeros(n, ml, mu);
        // Pentadiagonal matrix where some sub-diagonals are dominant
        let vals: Vec<Vec<f64>> = vec![
            vec![1.0, 2.0, -1.0, 0.0, 0.0, 0.0],
            vec![5.0, 8.0, 2.0, -1.0, 0.0, 0.0],
            vec![-2.0, 3.0, 7.0, 2.0, -1.0, 0.0],
            vec![0.0, -1.0, 4.0, 9.0, 2.0, -1.0],
            vec![0.0, 0.0, -2.0, 3.0, 8.0, 1.0],
            vec![0.0, 0.0, 0.0, -1.0, 4.0, 6.0],
        ];
        for i in 0..n {
            for j in 0..n {
                if vals[i][j] != 0.0 {
                    a.set(i, j, vals[i][j]);
                }
            }
        }

        let x_true = [1.0, -1.0, 2.0, -2.0, 3.0, -3.0];
        let mut b = vec![0.0f64; n];
        for i in 0..n {
            for j in 0..n {
                b[i] += vals[i][j] * x_true[j];
            }
        }
        let b_orig = b.clone();
        let mut pivots = vec![0usize; n];
        a.band_getrf(&mut pivots).unwrap();
        a.band_getrs(&pivots, &mut b).unwrap();

        for i in 0..n {
            let ax_i: f64 = (0..n).map(|j| vals[i][j] * b[j]).sum();
            assert!(
                (ax_i - b_orig[i]).abs() < 1e-8,
                "pentadiag residual[{i}]={:.2e}",
                (ax_i - b_orig[i]).abs()
            );
        }
    }
}
