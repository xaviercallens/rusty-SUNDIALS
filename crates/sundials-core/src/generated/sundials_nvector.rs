//! Generic `NVector` infrastructure for rusty-SUNDIALS.
//!
//! This module provides an idiomatic Rust translation of core generic NVECTOR
//! scaffolding from SUNDIALS C, adapted to:
//! - static dispatch via traits (`NVector`),
//! - RAII ownership,
//! - `Result`-based error handling,
//! - `f64` (`sunrealtype`) and `usize` (`sunindextype`).
//!
//! The API mirrors the visible C operations while preserving IEEE-754 behavior.

#![allow(clippy::needless_pass_by_value)]

use std::fmt;
use std::sync::Arc;

/// SUNDIALS real type (`sunrealtype`).
pub type SunReal = f64;

/// SUNDIALS index type (`sunindextype`).
pub type SunIndex = usize;

/// Error type replacing integer return codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// Equivalent to allocation failure (`CV_MEM_FAIL` / `SUN_ERR_MALLOC_FAIL`).
    MemFail,
    /// Corrupt or invalid argument (`SUN_ERR_ARG_CORRUPT`-style).
    ArgCorrupt(&'static str),
    /// Operation is not implemented by this vector backend.
    NotImplemented(&'static str),
    /// Generic failure with message.
    Failure(&'static str),
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemFail => write!(f, "memory allocation failed"),
            Self::ArgCorrupt(msg) => write!(f, "argument corrupt: {msg}"),
            Self::NotImplemented(op) => write!(f, "operation not implemented: {op}"),
            Self::Failure(msg) => write!(f, "failure: {msg}"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Abstract communicator handle (placeholder for MPI/etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunComm {
    /// Null communicator.
    Null,
}

/// Optional profiler handle.
#[derive(Debug, Clone, Default)]
pub struct SunProfiler;

/// SUNDIALS context.
#[derive(Debug, Clone, Default)]
pub struct SunContext {
    /// Optional profiler.
    pub profiler: Option<Arc<SunProfiler>>,
}

/// Vector backend identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NVectorId {
    /// Serial CPU vector.
    Serial,
    /// Custom backend.
    Custom(u32),
}

/// Trait replacing the C `N_Vector_Ops` function table.
pub trait NVector: Clone {
    /// Backend ID.
    fn vector_id(&self) -> NVectorId;

    /// Clone with data.
    #[inline]
    fn clone_vec(&self) -> Self {
        self.clone()
    }

    /// Clone structure without data ownership semantics change.
    #[inline]
    fn clone_empty(&self) -> Self {
        self.clone()
    }

    /// Space usage `(lrw, liw)`.
    fn space(&self) -> (SunIndex, SunIndex);

    /// Optional host array pointer view.
    fn get_array_pointer(&self) -> Option<&[SunReal]>;

    /// Optional mutable host array pointer view.
    fn get_array_pointer_mut(&mut self) -> Option<&mut [SunReal]>;

    /// Optional device pointer exposure (opaque).
    #[inline]
    fn get_device_array_pointer(&self) -> Option<*const SunReal> {
        None
    }

    /// Set array pointer semantics (backend-defined).
    #[inline]
    fn set_array_pointer(&mut self, _data: Vec<SunReal>) -> Result<(), CvodeError> {
        Err(CvodeError::NotImplemented("set_array_pointer"))
    }

    /// Communicator.
    #[inline]
    fn communicator(&self) -> SunComm {
        SunComm::Null
    }

    /// Global length.
    fn length(&self) -> SunIndex;

    /// Local length.
    #[inline]
    fn local_length(&self) -> SunIndex {
        self.length()
    }

    fn linear_sum(&self, a: SunReal, y: &Self, b: SunReal, z: &mut Self) -> Result<(), CvodeError>;
    fn set_const(&mut self, c: SunReal) -> Result<(), CvodeError>;
    fn prod(&self, y: &Self, z: &mut Self) -> Result<(), CvodeError>;
    fn div(&self, y: &Self, z: &mut Self) -> Result<(), CvodeError>;
    fn scale(&self, c: SunReal, z: &mut Self) -> Result<(), CvodeError>;
    fn abs(&self, z: &mut Self) -> Result<(), CvodeError>;
    fn inv(&self, z: &mut Self) -> Result<(), CvodeError>;
    fn add_const(&self, b: SunReal, z: &mut Self) -> Result<(), CvodeError>;
    fn dot_prod(&self, y: &Self) -> Result<SunReal, CvodeError>;
    fn max_norm(&self) -> Result<SunReal, CvodeError>;
    fn wrms_norm(&self, w: &Self) -> Result<SunReal, CvodeError>;
    fn wrms_norm_mask(&self, w: &Self, id: &Self) -> Result<SunReal, CvodeError>;
}

/// Generic vector wrapper carrying context (C `N_Vector` analogue).
#[derive(Debug, Clone)]
pub struct Vector<V: NVector> {
    content: V,
    sunctx: Arc<SunContext>,
}

impl<V: NVector> Vector<V> {
    /// Create a new vector wrapper.
    #[inline]
    pub const fn new(content: V, sunctx: Arc<SunContext>) -> Self {
        Self { content, sunctx }
    }

    /// Equivalent to `N_VGetVectorID`.
    #[inline]
    pub fn get_vector_id(&self) -> NVectorId {
        self.content.vector_id()
    }

    /// Equivalent to `N_VClone`.
    #[inline]
    pub fn clone_vector(&self) -> Self {
        Self {
            content: self.content.clone_vec(),
            sunctx: Arc::clone(&self.sunctx),
        }
    }

    /// Equivalent to `N_VCloneEmpty`.
    #[inline]
    pub fn clone_empty(&self) -> Self {
        Self {
            content: self.content.clone_empty(),
            sunctx: Arc::clone(&self.sunctx),
        }
    }

    /// Equivalent to `N_VSpace`.
    #[inline]
    pub fn space(&self) -> (SunIndex, SunIndex) {
        self.content.space()
    }

    /// Equivalent to `N_VGetArrayPointer`.
    #[inline]
    pub fn get_array_pointer(&self) -> Option<&[SunReal]> {
        self.content.get_array_pointer()
    }

    /// Mutable array pointer view.
    #[inline]
    pub fn get_array_pointer_mut(&mut self) -> Option<&mut [SunReal]> {
        self.content.get_array_pointer_mut()
    }

    /// Equivalent to `N_VGetDeviceArrayPointer`.
    #[inline]
    pub fn get_device_array_pointer(&self) -> Option<*const SunReal> {
        self.content.get_device_array_pointer()
    }

    /// Equivalent to `N_VSetArrayPointer`.
    #[inline]
    pub fn set_array_pointer(&mut self, data: Vec<SunReal>) -> Result<(), CvodeError> {
        self.content.set_array_pointer(data)?;
        Ok(())
    }

    /// Equivalent to `N_VGetCommunicator`.
    #[inline]
    pub fn communicator(&self) -> SunComm {
        self.content.communicator()
    }

    /// Equivalent to `N_VGetLength`.
    #[inline]
    pub fn length(&self) -> SunIndex {
        self.content.length()
    }

    /// Equivalent to `N_VGetLocalLength`.
    #[inline]
    pub fn local_length(&self) -> SunIndex {
        self.content.local_length()
    }

    /// Equivalent to `N_VLinearSum`: `z = a x + b y`.
    #[inline]
    pub fn linear_sum(&self, a: SunReal, y: &Self, b: SunReal, z: &mut Self) -> Result<(), CvodeError> {
        self.content.linear_sum(a, &y.content, b, &mut z.content)?;
        Ok(())
    }

    /// Equivalent to `N_VConst`.
    #[inline]
    pub fn set_const(&mut self, c: SunReal) -> Result<(), CvodeError> {
        self.content.set_const(c)?;
        Ok(())
    }

    /// Equivalent to `N_VProd`.
    #[inline]
    pub fn prod(&self, y: &Self, z: &mut Self) -> Result<(), CvodeError> {
        self.content.prod(&y.content, &mut z.content)?;
        Ok(())
    }

    /// Equivalent to `N_VDiv`.
    #[inline]
    pub fn div(&self, y: &Self, z: &mut Self) -> Result<(), CvodeError> {
        self.content.div(&y.content, &mut z.content)?;
        Ok(())
    }

    /// Equivalent to `N_VScale`.
    #[inline]
    pub fn scale(&self, c: SunReal, z: &mut Self) -> Result<(), CvodeError> {
        self.content.scale(c, &mut z.content)?;
        Ok(())
    }

    /// Equivalent to `N_VAbs`.
    #[inline]
    pub fn abs(&self, z: &mut Self) -> Result<(), CvodeError> {
        self.content.abs(&mut z.content)?;
        Ok(())
    }

    /// Equivalent to `N_VInv`.
    #[inline]
    pub fn inv(&self, z: &mut Self) -> Result<(), CvodeError> {
        self.content.inv(&mut z.content)?;
        Ok(())
    }

    /// Equivalent to `N_VAddConst`.
    #[inline]
    pub fn add_const(&self, b: SunReal, z: &mut Self) -> Result<(), CvodeError> {
        self.content.add_const(b, &mut z.content)?;
        Ok(())
    }

    /// Equivalent to `N_VDotProd`.
    #[inline]
    pub fn dot_prod(&self, y: &Self) -> Result<SunReal, CvodeError> {
        self.content.dot_prod(&y.content)
    }

    /// Equivalent to `N_VMaxNorm`.
    #[inline]
    pub fn max_norm(&self) -> Result<SunReal, CvodeError> {
        self.content.max_norm()
    }

    /// Equivalent to `N_VWrmsNorm`.
    ///
    /// Computes
    /// \[
    /// \|x\|_{\mathrm{wrms}} = \sqrt{\frac{1}{N}\sum_{i=1}^N (x_i w_i)^2 }.
    /// \]
    #[inline]
    pub fn wrms_norm(&self, w: &Self) -> Result<SunReal, CvodeError> {
        self.content.wrms_norm(&w.content)
    }

    /// Equivalent to `N_VWrmsNormMask`.
    #[inline]
    pub fn wrms_norm_mask(&self, w: &Self, id: &Self) -> Result<SunReal, CvodeError> {
        self.content.wrms_norm_mask(&w.content, &id.content)
    }
}