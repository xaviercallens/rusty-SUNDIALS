//! CVODE optional input setters (optimized Rust translation scaffold).
//!
//! This module provides an idiomatic Rust API for a subset of CVODE optional
//! input functions, translated from the SUNDIALS C implementation.
//!
//! # Design
//!
//! - `Result<T, CvodeError>` replaces integer status codes
//! - typed solver memory via `Cvode<F, V, LS, M, UD>`
//! - trait-based abstractions for vectors, linear solvers, and matrices
//! - RAII ownership and no manual memory management
//! - builder-style configuration via `CvodeBuilder`
//!
//! Numerical semantics are preserved with `f64` and IEEE-754 comparisons.

#![allow(clippy::module_name_repetitions)]

use core::{fmt, marker::PhantomData};

/// Real scalar type (`sunrealtype` in SUNDIALS).
pub type Real = f64;

/// Index type (`sunindextype` in SUNDIALS).
pub type Index = usize;

/// CVODE linear multistep method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lmm {
    Adams,
    Bdf,
}

/// Errors corresponding to CVODE setter failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// Equivalent to `CV_MEM_NULL`.
    MemNull,
    /// Equivalent to `CV_ILL_INPUT` with context.
    IllInput(&'static str),
    /// Monitoring requested but crate built without monitoring support.
    MonitoringDisabled,
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemNull => write!(f, "CVODE memory is null"),
            Self::IllInput(msg) => write!(f, "Illegal input: {msg}"),
            Self::MonitoringDisabled => {
                write!(f, "SUNDIALS was not built with monitoring enabled")
            }
        }
    }
}

impl std::error::Error for CvodeError {}

/// N_Vector abstraction (static dispatch, zero-cost).
pub trait NVector: Clone {}

/// SUNLinearSolver abstraction.
pub trait LinearSolver {}

/// SUNMatrix abstraction.
pub trait SunMatrix {}

/// Monitor callback signature.
pub type MonitorFn<V, UD> = fn(t: Real, y: &V, user_data: Option<&UD>) -> i32;

// Defaults (mirroring C constants where known; placeholders where not shown).
const ZERO: Real = 0.0;
const ONE: Real = 1.0;
const DGMAX_LSETUP_DEFAULT: Real = 0.2;
const MXSTEP_DEFAULT: i64 = 500;
const HMIN_DEFAULT: Real = 0.0;
const HMAX_INV_DEFAULT: Real = 0.0;
const ETA_MIN_FX_DEFAULT: Real = 0.0;
const ETA_MAX_FX_DEFAULT: Real = 1.5;
const ETA_MAX_FS_DEFAULT: Real = 10.0;
const ETA_MAX_ES_DEFAULT: Real = 10.0;
const SMALL_NST_DEFAULT: i64 = 10;
const ETA_MAX_GS_DEFAULT: Real = 10.0;
const ETA_MIN_DEFAULT: Real = 0.1;
const ETA_MIN_EF_DEFAULT: Real = 0.1;
const ETA_MAX_EF_DEFAULT: Real = 0.2;
const SMALL_NEF_DEFAULT: i32 = 2;
const MXHNIL_DEFAULT: i32 = 10;

#[inline]
const fn default_monitor_interval() -> i64 {
    0
}

#[inline]
const fn default_hin() -> Real {
    0.0
}

/// Typed CVODE memory.
///
/// Stores solver configuration and optional runtime hooks.
///
/// The step-size bounds satisfy:
/// - `hmin >= 0`
/// - `hmax >= 0` represented internally as `hmax_inv = 1/hmax` (or `0` if unlimited)
/// - consistency constraint `hmin <= hmax` enforced as `hmin * hmax_inv <= 1`.
pub struct Cvode<F, V, LS, M, UD>
where
    V: NVector,
    LS: LinearSolver,
    M: SunMatrix,
{
    pub rhs: F,
    pub lmm: Lmm,
    pub user_data: Option<UD>,
    pub dgmax_lsetup: Real,
    pub monitor_fn: Option<MonitorFn<V, UD>>,
    pub monitor_interval: i64,
    pub qmax_alloc: i32,
    pub qmax: i32,
    pub mxstep: i64,
    pub mxhnil: i32,
    pub sldeton: bool,
    pub hin: Real,
    pub hmin: Real,
    pub hmax_inv: Real,
    pub eta_min_fx: Real,
    pub eta_max_fx: Real,
    pub eta_max_fs: Real,
    pub eta_max_es: Real,
    pub small_nst: i64,
    pub eta_max_gs: Real,
    pub eta_min: Real,
    pub eta_min_ef: Real,
    pub eta_max_ef: Real,
    pub small_nef: i32,
    _phantom_ls: PhantomData<LS>,
    _phantom_m: PhantomData<M>,
}

impl<F, V, LS, M, UD> Cvode<F, V, LS, M, UD>
where
    V: NVector,
    LS: LinearSolver,
    M: SunMatrix,
{
    /// Create a builder for `Cvode`.
    #[inline]
    pub fn builder(rhs: F, lmm: Lmm, qmax_alloc: i32) -> CvodeBuilder<F, V, LS, M, UD> {
        CvodeBuilder::new(rhs, lmm, qmax_alloc)
    }

    #[inline]
    pub fn set_delta_gamma_max_lsetup(&mut self, dgmax_lsetup: Real) -> Result<(), CvodeError> {
        self.dgmax_lsetup = if dgmax_lsetup < ZERO {
            DGMAX_LSETUP_DEFAULT
        } else {
            dgmax_lsetup
        };
        Ok(())
    }

    #[inline]
    pub fn set_user_data(&mut self, user_data: Option<UD>) -> Result<(), CvodeError> {
        self.user_data = user_data;
        Ok(())
    }

    #[inline]
    pub fn set_monitor_frequency(&mut self, nst: i64) -> Result<(), CvodeError> {
        if nst < 0 {
            return Err(CvodeError::IllInput("step interval must be >= 0"));
        }
        self.monitor_interval = nst;
        Ok(())
    }

    #[inline]
    pub fn set_max_ord(&mut self, maxord: i32) -> Result<(), CvodeError> {
        if maxord <= 0 {
            return Err(CvodeError::IllInput("maxord must be > 0"));
        }
        if maxord > self.qmax_alloc {
            return Err(CvodeError::IllInput(
                "maxord cannot exceed allocation-time maximum order",
            ));
        }
        self.qmax = maxord;
        Ok(())
    }

    #[inline]
    pub fn set_max_num_steps(&mut self, mxsteps: i64) -> Result<(), CvodeError> {
        self.mxstep = if mxsteps == 0 { MXSTEP_DEFAULT } else { mxsteps };
        Ok(())
    }

    #[inline]
    pub fn set_max_hnil_warns(&mut self, mxhnil: i32) -> Result<(), CvodeError> {
        self.mxhnil = mxhnil;
        Ok(())
    }

    #[inline]
    pub fn set_stab_lim_det(&mut self, sldet: bool) -> Result<(), CvodeError> {
        if sldet && self.lmm != Lmm::Bdf {
            return Err(CvodeError::IllInput(
                "stability limit detection is only valid for BDF",
            ));
        }
        self.sldeton = sldet;
        Ok(())
    }

    #[inline]
    pub fn set_init_step(&mut self, hin: Real) -> Result<(), CvodeError> {
        self.hin = hin;
        Ok(())
    }

    #[inline]
    pub fn set_min_step(&mut self, hmin: Real) -> Result<(), CvodeError> {
        if hmin < ZERO {
            return Err(CvodeError::IllInput("hmin must be >= 0"));
        }
        if hmin == ZERO {
            self.hmin = HMIN_DEFAULT;
            return Ok(());
        }
        if hmin * self.hmax_inv > ONE {
            return Err(CvodeError::IllInput("inconsistent hmin/hmax"));
        }
        self.hmin = hmin;
        Ok(())
    }

    #[inline]
    pub fn set_max_step(&mut self, hmax: Real) -> Result<(), CvodeError> {
        if hmax < ZERO {
            return Err(CvodeError::IllInput("hmax must be >= 0"));
        }
        if hmax == ZERO {
            self.hmax_inv = HMAX_INV_DEFAULT;
            return Ok(());
        }
        let hmax_inv = ONE / hmax;
        if hmax_inv * self.hmin > ONE {
            return Err(CvodeError::IllInput("inconsistent hmin/hmax"));
        }
        self.hmax_inv = hmax_inv;
        Ok(())
    }
}

/// Builder for `Cvode`.
pub struct CvodeBuilder<F, V, LS, M, UD>
where
    V: NVector,
    LS: LinearSolver,
    M: SunMatrix,
{
    rhs: F,
    lmm: Lmm,
    qmax_alloc: i32,
    _p: PhantomData<(V, LS, M, UD)>,
}

impl<F, V, LS, M, UD> CvodeBuilder<F, V, LS, M, UD>
where
    V: NVector,
    LS: LinearSolver,
    M: SunMatrix,
{
    #[inline]
    pub const fn new(rhs: F, lmm: Lmm, qmax_alloc: i32) -> Self {
        Self {
            rhs,
            lmm,
            qmax_alloc,
            _p: PhantomData,
        }
    }

    #[inline]
    pub fn build(self) -> Result<Cvode<F, V, LS, M, UD>, CvodeError> {
        if self.qmax_alloc <= 0 {
            return Err(CvodeError::IllInput("qmax_alloc must be > 0"));
        }

        Ok(Cvode {
            rhs: self.rhs,
            lmm: self.lmm,
            user_data: None,
            dgmax_lsetup: DGMAX_LSETUP_DEFAULT,
            monitor_fn: None,
            monitor_interval: default_monitor_interval(),
            qmax_alloc: self.qmax_alloc,
            qmax: self.qmax_alloc,
            mxstep: MXSTEP_DEFAULT,
            mxhnil: MXHNIL_DEFAULT,
            sldeton: false,
            hin: default_hin(),
            hmin: HMIN_DEFAULT,
            hmax_inv: HMAX_INV_DEFAULT,
            eta_min_fx: ETA_MIN_FX_DEFAULT,
            eta_max_fx: ETA_MAX_FX_DEFAULT,
            eta_max_fs: ETA_MAX_FS_DEFAULT,
            eta_max_es: ETA_MAX_ES_DEFAULT,
            small_nst: SMALL_NST_DEFAULT,
            eta_max_gs: ETA_MAX_GS_DEFAULT,
            eta_min: ETA_MIN_DEFAULT,
            eta_min_ef: ETA_MIN_EF_DEFAULT,
            eta_max_ef: ETA_MAX_EF_DEFAULT,
            small_nef: SMALL_NEF_DEFAULT,
            _phantom_ls: PhantomData,
            _phantom_m: PhantomData,
        })
    }
}