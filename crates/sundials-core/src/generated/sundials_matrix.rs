//! Generic SUNMatrix infrastructure (idiomatic Rust translation of SUNDIALS C core).
//!
//! This module provides a safe, zero-cost abstraction over matrix backends,
//! replacing C ops tables with Rust traits and `Result`-based error handling.
//!
//! ## Type mappings
//! - `sunrealtype` → [`SunReal`] = `f64`
//! - `sunindextype` → [`SunIndex`] = `usize`
//! - integer error codes → [`CvodeError`]
//!
//! ## Numerical semantics
//! All arithmetic remains IEEE-754 `f64`, preserving numerical behavior.

use core::fmt;

/// SUNDIALS real scalar type.
pub type SunReal = f64;

/// SUNDIALS index type.
pub type SunIndex = usize;

/// Error type used across this translated module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// Allocation failure (`SUN_ERR_MALLOC_FAIL` / `CV_MEM_FAIL`-style).
    MallocFail,
    /// Corrupt or invalid argument (`SUN_ERR_ARG_CORRUPT`).
    ArgCorrupt(&'static str),
    /// Operation is not implemented (`SUN_ERR_NOT_IMPLEMENTED`).
    NotImplemented(&'static str),
    /// Generic operation failure.
    OperationFailed(&'static str),
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MallocFail => write!(f, "memory allocation failed"),
            Self::ArgCorrupt(msg) => write!(f, "argument corrupt: {msg}"),
            Self::NotImplemented(msg) => write!(f, "not implemented: {msg}"),
            Self::OperationFailed(msg) => write!(f, "operation failed: {msg}"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Minimal NVector trait (static dispatch, zero-cost abstraction).
pub trait NVector: Clone {
    /// Vector length.
    fn len(&self) -> usize;

    /// Whether vector is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Matrix identifier (analogous to `SUNMatrix_ID`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunMatrixId {
    Dense,
    Band,
    Sparse,
    Custom,
}

/// Trait for matrix backends (analogous to `SUNMatrix_Ops` function table).
pub trait SunMatrix<V: NVector>: Clone {
    /// Return matrix backend ID.
    fn get_id(&self) -> SunMatrixId;

    /// Set all entries to zero.
    fn zero(&mut self) -> Result<(), CvodeError>;

    /// Copy `self` into `dst`.
    fn copy_to(&self, dst: &mut Self) -> Result<(), CvodeError>;

    /// Compute `B <- c*A + B` where `A=self`.
    fn scale_add(&self, c: SunReal, b: &mut Self) -> Result<(), CvodeError>;

    /// Compute `A <- c*A + I`.
    fn scale_add_i(&mut self, c: SunReal) -> Result<(), CvodeError>;

    /// Optional setup for matvec.
    #[inline]
    fn matvec_setup(&mut self) -> Result<(), CvodeError> {
        Ok(())
    }

    /// Compute `y <- A*x`.
    fn matvec(&self, x: &V, y: &mut V) -> Result<(), CvodeError>;

    /// Optional Hermitian-transpose matvec: `y <- Aᴴ x`.
    #[inline]
    fn hermitian_transpose_vec(&self, _x: &V, _y: &mut V) -> Result<(), CvodeError> {
        Err(CvodeError::NotImplemented(
            "hermitian transpose matvec is not provided by this matrix type",
        ))
    }

    /// Return `(lenrw, leniw)` workspace usage.
    fn space(&self) -> Result<(usize, usize), CvodeError>;
}

/// Generic matrix wrapper corresponding to C `SUNMatrix`.
#[derive(Debug)]
pub struct GenericSunMatrix<M> {
    inner: M,
}

impl<M> GenericSunMatrix<M> {
    /// Create a new matrix wrapper.
    #[inline]
    pub const fn new(inner: M) -> Self {
        Self { inner }
    }

    /// Consume wrapper and return inner matrix.
    #[inline]
    pub fn into_inner(self) -> M {
        self.inner
    }

    /// Borrow inner matrix.
    #[inline]
    pub const fn inner(&self) -> &M {
        &self.inner
    }

    /// Mutably borrow inner matrix.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut M {
        &mut self.inner
    }
}

impl<M: Clone> Clone for GenericSunMatrix<M> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

/// Equivalent to `SUNMatNewEmpty` in spirit.
#[inline]
pub fn sun_mat_new_empty<M>(content: Option<M>) -> Option<GenericSunMatrix<M>> {
    content.map(GenericSunMatrix::new)
}

/// Equivalent to `SUNMatFreeEmpty`: dropping is automatic via RAII.
#[inline]
pub fn sun_mat_free_empty<M>(_a: Option<GenericSunMatrix<M>>) {}

/// Equivalent to `SUNMatCopyOps` semantics at high level.
#[inline]
pub fn sun_mat_copy_ops<M>(
    _a: &GenericSunMatrix<M>,
    _b: &mut GenericSunMatrix<M>,
) -> Result<(), CvodeError> {
    Ok(())
}

/// `SUNMatGetID`.
#[inline]
pub fn sun_mat_get_id<M, V>(a: &GenericSunMatrix<M>) -> SunMatrixId
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.get_id()
}

/// `SUNMatClone`.
#[inline]
pub fn sun_mat_clone<M>(a: &GenericSunMatrix<M>) -> GenericSunMatrix<M>
where
    M: Clone,
{
    a.clone()
}

/// `SUNMatDestroy` equivalent: RAII drop.
#[inline]
pub fn sun_mat_destroy<M>(_a: GenericSunMatrix<M>) {}

/// `SUNMatZero`.
#[inline]
pub fn sun_mat_zero<M, V>(a: &mut GenericSunMatrix<M>) -> Result<(), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.zero()?;
    Ok(())
}

/// `SUNMatCopy`.
#[inline]
pub fn sun_mat_copy<M, V>(
    a: &GenericSunMatrix<M>,
    b: &mut GenericSunMatrix<M>,
) -> Result<(), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.copy_to(&mut b.inner)?;
    Ok(())
}

/// `SUNMatScaleAdd`: `B <- c*A + B`.
#[inline]
pub fn sun_mat_scale_add<M, V>(
    c: SunReal,
    a: &GenericSunMatrix<M>,
    b: &mut GenericSunMatrix<M>,
) -> Result<(), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.scale_add(c, &mut b.inner)?;
    Ok(())
}

/// `SUNMatScaleAddI`: `A <- c*A + I`.
#[inline]
pub fn sun_mat_scale_add_i<M, V>(c: SunReal, a: &mut GenericSunMatrix<M>) -> Result<(), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.scale_add_i(c)?;
    Ok(())
}

/// `SUNMatMatvecSetup`.
#[inline]
pub fn sun_mat_matvec_setup<M, V>(a: &mut GenericSunMatrix<M>) -> Result<(), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.matvec_setup()?;
    Ok(())
}

/// `SUNMatMatvec`: compute `y = A x`.
#[inline]
pub fn sun_mat_matvec<M, V>(
    a: &GenericSunMatrix<M>,
    x: &V,
    y: &mut V,
) -> Result<(), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.matvec(x, y)?;
    Ok(())
}

/// `SUNMatHermitianTransposeVec`: compute `y = A^H x` if implemented.
#[inline]
pub fn sun_mat_hermitian_transpose_vec<M, V>(
    a: &GenericSunMatrix<M>,
    x: &V,
    y: &mut V,
) -> Result<(), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    a.inner.hermitian_transpose_vec(x, y)?;
    Ok(())
}

/// `SUNMatSpace`: return `(lenrw, leniw)`.
#[inline]
pub fn sun_mat_space<M, V>(a: &GenericSunMatrix<M>) -> Result<(usize, usize), CvodeError>
where
    M: SunMatrix<V>,
    V: NVector,
{
    let space = a.inner.space()?;
    Ok(space)
}