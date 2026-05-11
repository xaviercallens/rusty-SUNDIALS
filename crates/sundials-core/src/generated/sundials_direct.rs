//! Direct-matrix utilities translated from SUNDIALS C implementation.
//!
//! This module provides dense and band matrix storage constructors and basic
//! operations (`set_to_zero`, `add_identity`, pretty-print), using safe Rust
//! ownership (RAII) and `Result`-based error handling while preserving IEEE-754
//! numerical behavior.

use core::fmt;
use std::io::{self, Write};

/// SUNDIALS scalar type (`sunrealtype`).
pub type SunReal = f64;
/// SUNDIALS index type (`sunindextype`).
pub type SunIndex = usize;

/// Errors corresponding to C-side allocation/argument failures.
#[derive(Debug)]
pub enum CvodeError {
    /// Invalid dimension or argument.
    IllegalInput(&'static str),
    /// Allocation failed.
    MemFail(&'static str),
    /// I/O error while printing.
    Io(io::Error),
}

impl fmt::Display for CvodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalInput(msg) => write!(f, "illegal input: {msg}"),
            Self::MemFail(msg) => write!(f, "memory allocation failed: {msg}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for CvodeError {}

impl From<io::Error> for CvodeError {
    #[inline]
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// Matrix storage kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatType {
    Dense,
    Band,
}

/// Direct matrix equivalent of `SUNDlsMat`.
///
/// Dense storage is column-major with index
/// \[ k = j \cdot m + i \].
///
/// Band storage is column-major with leading dimension `ldim = s_mu + ml + 1`,
/// and element `(i,j)` stored at
/// \[ k = j \cdot ldim + (i - j + s\_mu) \]
/// when `(i,j)` lies in the stored band.
#[derive(Debug)]
pub struct SundlsMat {
    data: Vec<SunReal>,
    m: SunIndex,
    n: SunIndex,
    mu: SunIndex,
    ml: SunIndex,
    s_mu: SunIndex,
    ldim: SunIndex,
    ldata: SunIndex,
    mtype: MatType,
}

impl SundlsMat {
    /// Create dense matrix with shape `m x n`.
    pub fn new_dense(m: SunIndex, n: SunIndex) -> Result<Self, CvodeError> {
        if m == 0 || n == 0 {
            return Err(CvodeError::IllegalInput("dense dimensions must be > 0"));
        }
        let ldata = m
            .checked_mul(n)
            .ok_or(CvodeError::MemFail("dense size overflow"))?;
        let mut data = Vec::new();
        data.try_reserve_exact(ldata)
            .map_err(|_| CvodeError::MemFail("dense data"))?;
        data.resize(ldata, 0.0);

        Ok(Self {
            data,
            m,
            n,
            mu: 0,
            ml: 0,
            s_mu: 0,
            ldim: m,
            ldata,
            mtype: MatType::Dense,
        })
    }

    /// Create band matrix with `n x n`, upper bandwidth `mu`, lower bandwidth `ml`,
    /// and stored upper bandwidth `s_mu`.
    pub fn new_band(
        n: SunIndex,
        mu: SunIndex,
        ml: SunIndex,
        s_mu: SunIndex,
    ) -> Result<Self, CvodeError> {
        if n == 0 {
            return Err(CvodeError::IllegalInput("band dimension must be > 0"));
        }
        let col_size = s_mu
            .checked_add(ml)
            .and_then(|v| v.checked_add(1))
            .ok_or(CvodeError::MemFail("band col size overflow"))?;
        let ldata = n
            .checked_mul(col_size)
            .ok_or(CvodeError::MemFail("band size overflow"))?;

        let mut data = Vec::new();
        data.try_reserve_exact(ldata)
            .map_err(|_| CvodeError::MemFail("band data"))?;
        data.resize(ldata, 0.0);

        Ok(Self {
            data,
            m: n,
            n,
            mu,
            ml,
            s_mu,
            ldim: col_size,
            ldata,
            mtype: MatType::Band,
        })
    }

    #[inline]
    const fn dense_idx(&self, i: SunIndex, j: SunIndex) -> SunIndex {
        j * self.m + i
    }

    #[inline]
    const fn band_idx(&self, i: SunIndex, j: SunIndex) -> SunIndex {
        let row = i + self.s_mu - j;
        j * self.ldim + row
    }

    /// Set matrix entries to zero exactly as in SUNDIALS C implementation.
    #[inline]
    pub fn set_to_zero(&mut self) {
        match self.mtype {
            MatType::Dense => self.data.iter_mut().for_each(|x| *x = 0.0),
            MatType::Band => {
                let col_size = self.mu + self.ml + 1;
                let start = self.s_mu - self.mu;
                self.data
                    .chunks_exact_mut(self.ldim)
                    .take(self.m)
                    .for_each(|col| col[start..start + col_size].iter_mut().for_each(|x| *x = 0.0));
            }
        }
    }

    /// Add identity matrix: `A <- A + I`.
    #[inline]
    pub fn add_identity(&mut self) {
        match self.mtype {
            MatType::Dense => {
                let n = self.n.min(self.m);
                (0..n).for_each(|i| {
                    let k = self.dense_idx(i, i);
                    self.data[k] += 1.0;
                });
            }
            MatType::Band => {
                self.data
                    .chunks_exact_mut(self.ldim)
                    .take(self.m)
                    .for_each(|col| col[self.s_mu] += 1.0);
            }
        }
    }

    /// Print matrix in SUNDIALS-like format.
    pub fn print_mat<W: Write>(&self, out: &mut W) -> Result<(), CvodeError> {
        writeln!(out)?;
        match self.mtype {
            MatType::Dense => {
                for i in 0..self.m {
                    for j in 0..self.n {
                        write!(out, "{:>12.6e}  ", self.data[self.dense_idx(i, j)])?;
                    }
                    writeln!(out)?;
                }
            }
            MatType::Band => {
                for i in 0..self.n {
                    let start = i.saturating_sub(self.ml);
                    let finish = (self.n - 1).min(i + self.mu);
                    for _ in 0..start {
                        write!(out, "{:>12}  ", "")?;
                    }
                    for j in start..=finish {
                        write!(out, "{:>12.6e}  ", self.data[self.band_idx(i, j)])?;
                    }
                    writeln!(out)?;
                }
            }
        }
        Ok(())
    }

    /// Matrix type.
    #[inline]
    pub const fn mat_type(&self) -> MatType {
        self.mtype
    }

    /// Leading dimension.
    #[inline]
    pub const fn ldim(&self) -> SunIndex {
        self.ldim
    }

    /// Total data length.
    #[inline]
    pub const fn ldata(&self) -> SunIndex {
        self.ldata
    }
}

#[inline]
fn alloc_zeroed<T: Default + Clone>(n: usize, what: &'static str) -> Result<Vec<T>, CvodeError> {
    if n == 0 {
        return Err(CvodeError::IllegalInput("array length must be > 0"));
    }
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| CvodeError::MemFail(what))?;
    v.resize(n, T::default());
    Ok(v)
}

/// Allocate integer array (`int*` equivalent).
#[inline]
pub fn new_int_array(n: usize) -> Result<Vec<i32>, CvodeError> {
    alloc_zeroed(n, "int array")
}

/// Allocate index array (`sunindextype*` equivalent).
#[inline]
pub fn new_index_array(n: usize) -> Result<Vec<SunIndex>, CvodeError> {
    alloc_zeroed(n, "index array")
}

/// Allocate real array (`sunrealtype*` equivalent).
#[inline]
pub fn new_real_array(n: usize) -> Result<Vec<SunReal>, CvodeError> {
    alloc_zeroed(n, "real array")
}