//! Dense matrix kernels translated from SUNDIALS C (`sundials_dense.c`-style routines).
//!
//! Storage is **column-major**: `a[col][row]`, matching SUNDIALS dense kernels.
//! Numerical behavior is preserved with IEEE-754 `f64`.

use core::fmt;

/// Floating-point scalar type (`sunrealtype`).
pub type SunReal = f64;
/// Index type (`sunindextype`).
pub type SunIndex = usize;

pub const fn zero() -> SunReal {
    0.0
}
pub const fn one() -> SunReal {
    1.0
}
pub const fn two() -> SunReal {
    2.0
}

/// Errors for dense matrix operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// Matrix dimensions are inconsistent with operation requirements.
    DimensionMismatch(&'static str),
    /// Input/output vector length mismatch.
    VectorLengthMismatch(&'static str),
    /// LU factorization encountered a zero pivot at 1-based index.
    ZeroPivot { at_1_based: SunIndex },
    /// Cholesky factorization encountered non-positive diagonal at 1-based index.
    NotPositiveDefinite { at_1_based: SunIndex },
}

impl fmt::Display for CvodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch(msg) => write!(f, "dimension mismatch: {msg}"),
            Self::VectorLengthMismatch(msg) => write!(f, "vector length mismatch: {msg}"),
            Self::ZeroPivot { at_1_based } => write!(f, "zero pivot at {}", at_1_based),
            Self::NotPositiveDefinite { at_1_based } => {
                write!(f, "matrix not positive definite at {}", at_1_based)
            }
        }
    }
}

impl std::error::Error for CvodeError {}

/// Dense matrix in column-major storage (`cols[j][i] = A(i,j)`).
#[derive(Clone, Debug)]
pub struct DenseMat {
    pub m: SunIndex,
    pub n: SunIndex,
    pub cols: Vec<Vec<SunReal>>,
}

impl DenseMat {
    /// Create a zero matrix of shape `m x n`.
    #[inline]
    pub fn zeros(m: SunIndex, n: SunIndex) -> Self {
        Self {
            m,
            n,
            cols: (0..n).map(|_| vec![zero(); m]).collect(),
        }
    }

    #[inline]
    fn check_well_formed(&self) -> Result<(), CvodeError> {
        if self.cols.len() != self.n {
            return Err(CvodeError::DimensionMismatch("cols.len() != n"));
        }
        if self.cols.iter().any(|c| c.len() != self.m) {
            return Err(CvodeError::DimensionMismatch("column length != m"));
        }
        Ok(())
    }

    /// LU factorization wrapper (in-place).
    #[inline]
    pub fn dense_getrf(&mut self, p: &mut [SunIndex]) -> Result<(), CvodeError> {
        self.check_well_formed()?;
        if p.len() != self.n {
            return Err(CvodeError::VectorLengthMismatch("pivot length must be n"));
        }
        dense_getrf(&mut self.cols, self.m, self.n, p)
    }

    /// Solve `Ax=b` using LU factors from `dense_getrf`.
    #[inline]
    pub fn dense_getrs(&self, p: &[SunIndex], b: &mut [SunReal]) -> Result<(), CvodeError> {
        self.check_well_formed()?;
        if self.n == 0 {
            return Ok(());
        }
        if p.len() != self.n || b.len() != self.n {
            return Err(CvodeError::VectorLengthMismatch("need len n for p and b"));
        }
        dense_getrs(&self.cols, self.n, p, b)
    }

    /// Cholesky factorization (lower triangle in-place).
    #[inline]
    pub fn dense_potrf(&mut self) -> Result<(), CvodeError> {
        self.check_well_formed()?;
        if self.m != self.n {
            return Err(CvodeError::DimensionMismatch("POTRF requires square matrix"));
        }
        dense_potrf(&mut self.cols, self.m)
    }

    /// Solve SPD system using Cholesky factors from `dense_potrf`.
    #[inline]
    pub fn dense_potrs(&self, b: &mut [SunReal]) -> Result<(), CvodeError> {
        self.check_well_formed()?;
        if self.m != self.n {
            return Err(CvodeError::DimensionMismatch("POTRS requires square matrix"));
        }
        if b.len() != self.m {
            return Err(CvodeError::VectorLengthMismatch("b length must be m"));
        }
        dense_potrs(&self.cols, self.m, b)
    }
}

/// In-place LU factorization with partial pivoting.
#[inline]
pub fn dense_getrf(
    a: &mut [Vec<SunReal>],
    m: SunIndex,
    n: SunIndex,
    p: &mut [SunIndex],
) -> Result<(), CvodeError> {
    for k in 0..n {
        let mut l = k;
        for i in (k + 1)..m {
            if a[k][i].abs() > a[k][l].abs() {
                l = i;
            }
        }
        p[k] = l;

        if a[k][l] == zero() {
            return Err(CvodeError::ZeroPivot { at_1_based: k + 1 });
        }

        if l != k {
            a.iter_mut().take(n).for_each(|col| col.swap(l, k));
        }

        let mult = one() / a[k][k];
        for val in a[k].iter_mut().take(m).skip(k + 1) {
            *val *= mult;
        }

        for j in (k + 1)..n {
            let a_kj = a[j][k];
            if a_kj != zero() {
                for i in (k + 1)..m {
                    a[j][i] -= a_kj * a[k][i];
                }
            }
        }
    }
    Ok(())
}

/// Solve `Ax=b` from LU factors.
#[inline]
pub fn dense_getrs(
    a: &[Vec<SunReal>],
    n: SunIndex,
    p: &[SunIndex],
    b: &mut [SunReal],
) -> Result<(), CvodeError> {
    for (k, &pk) in p.iter().take(n).enumerate() {
        if pk != k {
            b.swap(k, pk);
        }
    }

    for k in 0..n.saturating_sub(1) {
        for i in (k + 1)..n {
            b[i] -= a[k][i] * b[k];
        }
    }

    for k in (1..n).rev() {
        b[k] /= a[k][k];
        for i in 0..k {
            b[i] -= a[k][i] * b[k];
        }
    }
    if n > 0 {
        b[0] /= a[0][0];
    }
    Ok(())
}

/// In-place Cholesky factorization (gaxpy form), lower triangle used/stored.
#[inline]
pub fn dense_potrf(a: &mut [Vec<SunReal>], m: SunIndex) -> Result<(), CvodeError> {
    for j in 0..m {
        if j > 0 {
            for i in j..m {
                for k in 0..j {
                    a[j][i] -= a[k][i] * a[k][j];
                }
            }
        }

        let mut a_diag = a[j][j];
        if a_diag <= zero() {
            return Err(CvodeError::NotPositiveDefinite { at_1_based: j + 1 });
        }
        a_diag = a_diag.sqrt();

        for val in a[j].iter_mut().take(m).skip(j) {
            *val /= a_diag;
        }
    }
    Ok(())
}

/// Solve SPD system using Cholesky factors from `dense_potrf`.
#[inline]
pub fn dense_potrs(a: &[Vec<SunReal>], m: SunIndex, b: &mut [SunReal]) -> Result<(), CvodeError> {
    for j in 0..m.saturating_sub(1) {
        b[j] /= a[j][j];
        for i in (j + 1)..m {
            b[i] -= b[j] * a[j][i];
        }
    }
    if m > 0 {
        b[m - 1] /= a[m - 1][m - 1];
        b[m - 1] /= a[m - 1][m - 1];
    }

    for i in (0..m.saturating_sub(1)).rev() {
        for j in (i + 1)..m {
            b[i] -= a[i][j] * b[j];
        }
        b[i] /= a[i][i];
    }
    Ok(())
}

/// QR factorization via Householder reflections.
#[inline]
pub fn dense_geqrf(
    a: &mut [Vec<SunReal>],
    m: SunIndex,
    n: SunIndex,
    beta: &mut [SunReal],
    v: &mut [SunReal],
) -> Result<(), CvodeError> {
    if beta.len() < n || v.len() < m {
        return Err(CvodeError::VectorLengthMismatch("beta>=n and v>=m required"));
    }

    for j in 0..n {
        let ajj = a[j][j];
        v[0] = one();
        let mut s = zero();
        for i in 1..(m - j) {
            v[i] = a[j][i + j];
            s += v[i] * v[i];
        }

        if s != zero() {
            let mu = (ajj * ajj + s).sqrt();
            let v1 = if ajj <= zero() {
                ajj - mu
            } else {
                -s / (ajj + mu)
            };
            let v1_2 = v1 * v1;
            beta[j] = two() * v1_2 / (s + v1_2);
            for vi in v.iter_mut().take(m - j).skip(1) {
                *vi /= v1;
            }
        } else {
            beta[j] = zero();
        }

        for k in j..n {
            let mut sk = zero();
            for i in 0..(m - j) {
                sk += a[k][i + j] * v[i];
            }
            sk *= beta[j];
            for i in 0..(m - j) {
                a[k][i + j] -= sk * v[i];
            }
        }

        if j < m - 1 {
            for i in 1..(m - j) {
                a[j][i + j] = v[i];
            }
        }
    }
    Ok(())
}

/// Compute `vm = Q * vn` from Householder representation produced by `dense_geqrf`.
#[inline]
pub fn dense_ormqr(
    a: &[Vec<SunReal>],
    m: SunIndex,
    n: SunIndex,
    beta: &[SunReal],
    vn: &[SunReal],
    vm: &mut [SunReal],
    v: &mut [SunReal],
) -> Result<(), CvodeError> {
    if beta.len() < n || vn.len() < n || vm.len() < m || v.len() < m {
        return Err(CvodeError::VectorLengthMismatch("invalid beta/vn/vm/v lengths"));
    }

    vm[..n].copy_from_slice(&vn[..n]);
    vm[n..m].iter_mut().for_each(|yi| *yi = zero());

    for j in (0..n).rev() {
        v[0] = one();
        let mut s = vm[j];
        for i in 1..(m - j) {
            v[i] = a[j][i + j];
            s += v[i] * vm[i + j];
        }
        s *= beta[j];
        for i in 0..(m - j) {
            vm[i + j] -= s * v[i];
        }
    }
    Ok(())
}

/// `b = a` copy for `m x n` column-major blocks.
#[inline]
pub fn dense_copy(a: &[Vec<SunReal>], b: &mut [Vec<SunReal>], m: SunIndex, n: SunIndex) {
    for j in 0..n {
        b[j][..m].copy_from_slice(&a[j][..m]);
    }
}

/// Scale matrix by `c`.
#[inline]
pub fn dense_scale(c: SunReal, a: &mut [Vec<SunReal>], m: SunIndex, n: SunIndex) {
    for col in a.iter_mut().take(n) {
        col.iter_mut().take(m).for_each(|x| *x *= c);
    }
}

/// Add identity: `A <- A + I` for square `n x n`.
#[inline]
pub fn dense_add_identity(a: &mut [Vec<SunReal>], n: SunIndex) {
    for (i, col) in a.iter_mut().take(n).enumerate() {
        col[i] += one();
    }
}

/// Matrix-vector product `y = A x`, with `A` of shape `m x n`.
#[inline]
pub fn dense_matvec(
    a: &[Vec<SunReal>],
    x: &[SunReal],
    y: &mut [SunReal],
    m: SunIndex,
    n: SunIndex,
) -> Result<(), CvodeError> {
    if x.len() < n || y.len() < m {
        return Err(CvodeError::VectorLengthMismatch("x>=n and y>=m required"));
    }

    y.iter_mut().take(m).for_each(|yi| *yi = zero());

    for (j, &xj) in x.iter().take(n).enumerate() {
        if xj != zero() {
            for i in 0..m {
                y[i] += a[j][i] * xj;
            }
        }
    }
    Ok(())
}