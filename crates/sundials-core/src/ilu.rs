//! Incomplete LU Factorisation with 0 fill-in (ILU(0)).
//!
//! Provides a simple ILU(0) preconditioner for dense matrices.
//! Elements that are zero in the original matrix remain zero
//! during the factorisation.

use crate::generated::sundials_dense::DenseMat;
use crate::Real;

/// ILU(0) preconditioner.
pub struct Ilu0 {
    mat: DenseMat,
}

impl Ilu0 {
    /// Compute the ILU(0) factorisation of a given dense matrix.
    ///
    /// Values in `A` that are exactly 0.0 (or within a tight tolerance) are
    /// treated as structural zeros and will not be filled in during LU.
    pub fn new(mut a: DenseMat) -> Result<Self, &'static str> {
        let n = a.cols.len();
        if n == 0 {
            return Ok(Self { mat: a });
        }
        
        let m = a.cols[0].len();
        if n != m {
            return Err("Matrix must be square for ILU(0)");
        }

        // We identify the non-zero structure.
        let mut is_nz = vec![vec![false; n]; n];
        for j in 0..n {
            for i in 0..n {
                if a.cols[j][i].abs() > 0.0 {
                    is_nz[j][i] = true;
                }
            }
        }

        // Perform IKJ or KJI ILU(0) factorisation
        for k in 0..n-1 {
            if a.cols[k][k] == 0.0 {
                return Err("Zero pivot encountered in ILU(0) (pivoting not supported)");
            }
            
            for i in k+1..n {
                if is_nz[k][i] {
                    a.cols[k][i] /= a.cols[k][k];
                    for j in k+1..n {
                        if is_nz[j][i] {
                            // Only update if it's a structural non-zero
                            a.cols[j][i] -= a.cols[k][i] * a.cols[j][k];
                        }
                    }
                }
            }
        }

        Ok(Self { mat: a })
    }

    /// Apply the preconditioner: solve M x = b, where M = L * U.
    /// x is solved in-place.
    pub fn solve(&self, x: &mut [Real]) {
        let n = self.mat.cols.len();
        if n == 0 { return; }

        // Forward solve L y = b (L has unit diagonal)
        for i in 0..n {
            let mut sum = x[i];
            for j in 0..i {
                if self.mat.cols[j][i] != 0.0 {
                    sum -= self.mat.cols[j][i] * x[j];
                }
            }
            x[i] = sum;
        }

        // Backward solve U x = y
        for i in (0..n).rev() {
            let mut sum = x[i];
            for j in i+1..n {
                if self.mat.cols[j][i] != 0.0 {
                    sum -= self.mat.cols[j][i] * x[j];
                }
            }
            x[i] = sum / self.mat.cols[i][i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ilu0_identity() {
        let mut mat = DenseMat::zeros(3, 3);
        mat.cols[0][0] = 1.0;
        mat.cols[1][1] = 1.0;
        mat.cols[2][2] = 1.0;
        
        let ilu = Ilu0::new(mat).unwrap();
        let mut b = [1.0, 2.0, 3.0];
        ilu.solve(&mut b);
        assert_eq!(b, [1.0, 2.0, 3.0]);
    }
}
