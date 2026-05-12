//! Band-matrix kernels translated from SUNDIALS C to idiomatic Rust.
//!
//! This module implements LU factorization and solves for matrices stored in
//! compact band-column format, preserving the original numerical behavior
//! (IEEE-754 `f64`) and algorithmic structure.

#![allow(clippy::needless_range_loop)]

use core::fmt;

/// Floating-point scalar type (`sunrealtype` in SUNDIALS C).
pub type SunReal = f64;
/// Index type (`sunindextype` in SUNDIALS C).
pub type SunIndex = usize;

/// Error type replacing C integer return codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// Input dimensions or storage layout are invalid.
    InvalidInput(&'static str),
    /// Zero pivot encountered at 1-based elimination step.
    ZeroPivot { step_1based: usize },
}

impl fmt::Display for CvodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::ZeroPivot { step_1based } => write!(f, "zero pivot at step {}", step_1based),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Compute compact-band row offset: `ROW(i,j,smu) = i - j + smu`.
#[inline]
const fn row(i: usize, j: usize, smu: usize) -> usize {
    i + smu - j
}

/// Band matrix in column-major compact storage.
///
/// Each column `j` is a vector of length at least `s_mu + ml + 1`.
/// Diagonal entry `A[j,j]` is stored at `cols[j][s_mu]`.
#[derive(Clone, Debug)]
pub struct BandMat {
    pub cols: Vec<Vec<SunReal>>,
    pub m: usize,
    pub mu: usize,
    pub ml: usize,
    pub s_mu: usize,
}

impl BandMat {
    #[inline]
    pub fn validate(&self) -> Result<(), CvodeError> {
        if self.cols.len() != self.m {
            return Err(CvodeError::InvalidInput("cols.len() != m"));
        }
        if self.s_mu < self.mu {
            return Err(CvodeError::InvalidInput("s_mu < mu"));
        }
        let need = self.s_mu + self.ml + 1;
        if self.cols.iter().any(|c| c.len() < need) {
            return Err(CvodeError::InvalidInput("column storage too small"));
        }
        Ok(())
    }
}

#[inline]
fn validate_cols(a: &[Vec<SunReal>], n: usize, need: usize) -> Result<(), CvodeError> {
    if a.len() != n {
        return Err(CvodeError::InvalidInput("bad matrix column count"));
    }
    if a.iter().any(|c| c.len() < need) {
        return Err(CvodeError::InvalidInput("column storage too small"));
    }
    Ok(())
}

/// LU factorization with partial pivoting for compact band storage.
///
/// Returns `Ok(())` on success, or `Err(CvodeError::ZeroPivot{...})` with
/// 1-based step index exactly matching SUNDIALS/C behavior.
pub fn band_gbtrf(
    a: &mut [Vec<SunReal>],
    n: usize,
    mu: usize,
    ml: usize,
    smu: usize,
    p: &mut [usize],
) -> Result<(), CvodeError> {
    if p.len() < n {
        return Err(CvodeError::InvalidInput("bad dimensions for p"));
    }
    validate_cols(a, n, smu + ml + 1)?;

    let num_rows = smu.saturating_sub(mu);
    if num_rows > 0 {
        a.iter_mut()
            .for_each(|col| col.iter_mut().take(num_rows).for_each(|v| *v = 0.0));
    }

    for k in 0..n.saturating_sub(1) {
        let last_row_k = (n - 1).min(k + ml);

        let mut l = k;
        let mut max = a[k][smu].abs();
        for i in (k + 1)..=last_row_k {
            let v = a[k][row(i, k, smu)].abs();
            if v > max {
                l = i;
                max = v;
            }
        }

        p[k] = l;
        let storage_l = row(l, k, smu);
        if a[k][storage_l] == 0.0 {
            return Err(CvodeError::ZeroPivot { step_1based: k + 1 });
        }

        let swap = l != k;
        if swap {
            a[k].swap(storage_l, smu);
        }

        let mult = -1.0 / a[k][smu];
        for i in (k + 1)..=last_row_k {
            a[k][row(i, k, smu)] *= mult;
        }

        let last_col_k = (k + smu).min(n - 1);
        for j in (k + 1)..=last_col_k {
            let storage_lj = row(l, j, smu);
            let storage_kj = row(k, j, smu);
            let a_kj = a[j][storage_lj];

            if swap {
                a[j].swap(storage_lj, storage_kj);
            }

            if a_kj != 0.0 {
                for i in (k + 1)..=last_row_k {
                    let kptr = row(i, k, smu);
                    let jptr = row(i, j, smu);
                    a[j][jptr] += a_kj * a[k][kptr];
                }
            }
        }
    }

    if n > 0 {
        p[n - 1] = n - 1;
        if a[n - 1][smu] == 0.0 {
            return Err(CvodeError::ZeroPivot { step_1based: n });
        }
    }

    Ok(())
}

/// Solve `A x = b` using LU factors from [`band_gbtrf`], in-place on `b`.
pub fn band_gbtrs(
    a: &[Vec<SunReal>],
    n: usize,
    smu: usize,
    ml: usize,
    p: &[usize],
    b: &mut [SunReal],
) -> Result<(), CvodeError> {
    if p.len() < n || b.len() < n {
        return Err(CvodeError::InvalidInput("bad dimensions for solve"));
    }
    validate_cols(a, n, smu + ml + 1)?;

    for k in 0..n.saturating_sub(1) {
        let l = p[k];
        let mult = b[l];
        if l != k {
            b.swap(l, k);
        }
        let last_row_k = (n - 1).min(k + ml);
        for i in (k + 1)..=last_row_k {
            b[i] += mult * a[k][smu + (i - k)];
        }
    }

    for kk in 0..n {
        let k = n - 1 - kk;
        b[k] /= a[k][smu];
        let mult = -b[k];
        let first_row_k = k.saturating_sub(smu);
        for i in first_row_k..k {
            b[i] += mult * a[k][row(i, k, smu)];
        }
    }

    Ok(())
}

/// Copy a band block from `a` to `b`.
pub fn band_copy(
    a: &[Vec<SunReal>],
    b: &mut [Vec<SunReal>],
    n: usize,
    a_smu: usize,
    b_smu: usize,
    copymu: usize,
    copyml: usize,
) -> Result<(), CvodeError> {
    if a.len() != n || b.len() != n {
        return Err(CvodeError::InvalidInput("bad dimensions for copy"));
    }
    if a_smu < copymu || b_smu < copymu {
        return Err(CvodeError::InvalidInput("smu < copymu"));
    }

    let copy_size = copymu + copyml + 1;
    let a0 = a_smu - copymu;
    let b0 = b_smu - copymu;

    for (ac, bc) in a.iter().zip(b.iter_mut()) {
        if ac.len() < a0 + copy_size || bc.len() < b0 + copy_size {
            return Err(CvodeError::InvalidInput("column storage too small"));
        }
        bc[b0..b0 + copy_size].copy_from_slice(&ac[a0..a0 + copy_size]);
    }
    Ok(())
}

/// Scale the active band entries by `c`.
#[inline]
pub fn band_scale(
    c: SunReal,
    a: &mut [Vec<SunReal>],
    n: usize,
    mu: usize,
    ml: usize,
    smu: usize,
) -> Result<(), CvodeError> {
    if smu < mu {
        return Err(CvodeError::InvalidInput("smu < mu"));
    }
    validate_cols(a, n, smu + ml + 1)?;

    let col_size = mu + ml + 1;
    let start = smu - mu;
    a.iter_mut().for_each(|col| {
        col[start..start + col_size]
            .iter_mut()
            .for_each(|v| *v *= c)
    });
    Ok(())
}

/// Add identity: `A ← A + I`.
#[inline]
pub fn band_add_identity(a: &mut [Vec<SunReal>], n: usize, smu: usize) -> Result<(), CvodeError> {
    validate_cols(a, n, smu + 1)?;
    a.iter_mut().for_each(|col| col[smu] += 1.0);
    Ok(())
}

/// Matrix-vector product for compact band matrix: `y = A x`.
pub fn band_matvec(
    a: &[Vec<SunReal>],
    x: &[SunReal],
    y: &mut [SunReal],
    n: usize,
    mu: usize,
    ml: usize,
    smu: usize,
) -> Result<(), CvodeError> {
    if x.len() < n || y.len() < n {
        return Err(CvodeError::InvalidInput("bad dimensions for matvec"));
    }
    if smu < mu {
        return Err(CvodeError::InvalidInput("smu < mu"));
    }
    validate_cols(a, n, smu + ml + 1)?;

    y.iter_mut().take(n).for_each(|yi| *yi = 0.0);

    let base = smu - mu;
    for (j, col) in a.iter().enumerate().take(n) {
        let is = j.saturating_sub(mu);
        let ie = (n - 1).min(j + ml);
        let xj = x[j];
        for i in is..=ie {
            y[i] += col[base + (i + mu - j)] * xj;
        }
    }
    Ok(())
}
