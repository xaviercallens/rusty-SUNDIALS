//! cvode.rs
//!
//! Idiomatic Rust skeleton for selected CVODE creation/initialization logic,
//! translated from SUNDIALS C style into typed, RAII-safe Rust.
//!
//! This module is intentionally self-contained and compilable, while preserving
//! key numerical semantics and constants (`f64`, IEEE-754 behavior).

#![allow(clippy::too_many_arguments)]

use core::fmt;

/// Floating-point type used by SUNDIALS (`sunrealtype`).
pub type Real = f64;

/// Index type used by SUNDIALS (`sunindextype`).
pub type Index = usize;

/// Linear multistep method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lmm {
    Adams,
    Bdf,
}

/// CVODE error/status mapping from C integer codes to Rust `Result`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    MemNull,
    IllInput(&'static str),
    MemFail,
    RhsFuncFail,
    RootFuncFail,
    NonlinearSolverFail,
    Other(&'static str),
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemNull => write!(f, "CVODE memory is null/uninitialized"),
            Self::IllInput(msg) => write!(f, "Illegal input: {msg}"),
            Self::MemFail => write!(f, "Memory allocation failed"),
            Self::RhsFuncFail => write!(f, "RHS function failed"),
            Self::RootFuncFail => write!(f, "Root function failed"),
            Self::NonlinearSolverFail => write!(f, "Nonlinear solver failed"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Trait replacing `N_Vector_Ops` with static dispatch.
pub trait NVector: Clone {
    /// Returns `(lrw, liw)` analogous to `N_VSpace`.
    fn space(&self) -> (usize, usize);
}

/// Placeholder trait for SUNMatrix.
pub trait SunMatrix {}

/// Placeholder trait for SUNLinearSolver.
pub trait LinearSolver<V: NVector, M: SunMatrix> {}

/// RHS function trait (`CVRhsFn` equivalent).
pub trait RhsFn<V: NVector> {
    fn eval(&mut self, t: Real, y: &V, ydot: &mut V) -> Result<(), CvodeError>;
}

impl<V, F> RhsFn<V> for F
where
    V: NVector,
    F: FnMut(Real, &V, &mut V) -> Result<(), CvodeError>,
{
    #[inline]
    fn eval(&mut self, t: Real, y: &V, ydot: &mut V) -> Result<(), CvodeError> {
        self(t, y, ydot)
    }
}

/// CVODE constants (selected).
pub mod constants {
    use super::Real;

    pub const ZERO: Real = 0.0;
    pub const TINY: Real = 1.0e-10;
    pub const HALF: Real = 0.5;
    pub const ONE: Real = 1.0;
    pub const HUNDRED: Real = 100.0;
    pub const CORTES: Real = 0.1;

    pub const ADAMS_Q_MAX: usize = 12;
    pub const BDF_Q_MAX: usize = 5;

    pub const MXSTEP_DEFAULT: usize = 500;
    pub const MXHNIL_DEFAULT: usize = 10;
    pub const HMIN_DEFAULT: Real = 0.0;
    pub const HMAX_INV_DEFAULT: Real = 0.0;

    pub const fn default_lrw() -> usize {
        58 + 2 * 5 + 5
    }

    pub const fn default_liw() -> usize {
        40
    }
}

#[inline]
const fn maxord_for_lmm(lmm: Lmm) -> usize {
    match lmm {
        Lmm::Adams => constants::ADAMS_Q_MAX,
        Lmm::Bdf => constants::BDF_Q_MAX,
    }
}

/// Main typed CVODE memory block (`void* cvode_mem` replacement).
pub struct Cvode<V, F>
where
    V: NVector,
    F: RhsFn<V>,
{
    lmm: Lmm,
    qmax: usize,
    qmax_alloc: usize,
    uround: Real,

    // user callbacks/data
    f: Option<F>,
    user_data: Option<Box<dyn std::any::Any + Send + Sync>>,

    // tolerances/config
    mxstep: usize,
    mxhnil: usize,
    hin: Real,
    hmin: Real,
    hmax_inv: Real,
    nlscoef: Real,

    // state
    t0: Option<Real>,
    y0: Option<V>,

    // workspace accounting
    lrw: usize,
    liw: usize,
    lrw1: usize,
    liw1: usize,

    malloc_done: bool,
}

impl<V, F> Cvode<V, F>
where
    V: NVector,
    F: RhsFn<V>,
{
    /// Create a new CVODE solver memory block.
    #[inline]
    pub fn create(lmm: Lmm) -> Result<Self, CvodeError> {
        let maxord = maxord_for_lmm(lmm);

        Ok(Self {
            lmm,
            qmax: maxord,
            qmax_alloc: maxord,
            uround: f64::EPSILON,
            f: None,
            user_data: None,
            mxstep: constants::MXSTEP_DEFAULT,
            mxhnil: constants::MXHNIL_DEFAULT,
            hin: constants::ZERO,
            hmin: constants::HMIN_DEFAULT,
            hmax_inv: constants::HMAX_INV_DEFAULT,
            nlscoef: constants::CORTES,
            t0: None,
            y0: None,
            lrw: constants::default_lrw(),
            liw: constants::default_liw(),
            lrw1: 0,
            liw1: 0,
            malloc_done: false,
        })
    }

    /// Initialize solver memory with RHS, initial time, and initial state.
    ///
    /// Mathematically, this sets the IVP:
    /// \[
    /// \dot{y}(t) = f(t, y(t)), \quad y(t_0)=y_0.
    /// \]
    #[inline]
    pub fn init(&mut self, f: F, t0: Real, y0: V) -> Result<(), CvodeError> {
        let (lrw1, liw1) = y0.space();
        self.lrw1 = lrw1;
        self.liw1 = liw1;

        self.f = Some(f);
        self.t0 = Some(t0);
        self.y0 = Some(y0);
        self.malloc_done = true;
        Ok(())
    }

    /// Returns current method.
    #[inline]
    pub fn lmm(&self) -> Lmm {
        self.lmm
    }

    /// Returns configured maximum order.
    #[inline]
    pub fn qmax(&self) -> usize {
        self.qmax
    }

    /// Returns allocated maximum order.
    #[inline]
    pub fn qmax_alloc(&self) -> usize {
        self.qmax_alloc
    }

    /// Access machine roundoff used internally.
    #[inline]
    pub fn uround(&self) -> Real {
        self.uround
    }

    /// Returns workspace usage `(lrw, liw)`.
    #[inline]
    pub fn workspace(&self) -> (usize, usize) {
        (self.lrw + self.lrw1, self.liw + self.liw1)
    }

    /// Returns whether internal memory has been initialized.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.malloc_done
    }

    /// Set opaque user data.
    #[inline]
    pub fn set_user_data<T>(&mut self, data: T)
    where
        T: std::any::Any + Send + Sync,
    {
        self.user_data = Some(Box::new(data));
    }
}

/// Builder replacing many `CVodeSet*` style calls.
pub struct CvodeBuilder {
    lmm: Lmm,
    mxstep: usize,
    mxhnil: usize,
    hin: Real,
    hmin: Real,
    hmax_inv: Real,
    nlscoef: Real,
}

impl CvodeBuilder {
    #[inline]
    pub const fn new(lmm: Lmm) -> Self {
        Self {
            lmm,
            mxstep: constants::MXSTEP_DEFAULT,
            mxhnil: constants::MXHNIL_DEFAULT,
            hin: constants::ZERO,
            hmin: constants::HMIN_DEFAULT,
            hmax_inv: constants::HMAX_INV_DEFAULT,
            nlscoef: constants::CORTES,
        }
    }

    #[inline]
    pub const fn mxstep(mut self, v: usize) -> Self {
        self.mxstep = v;
        self
    }

    #[inline]
    pub const fn mxhnil(mut self, v: usize) -> Self {
        self.mxhnil = v;
        self
    }

    #[inline]
    pub const fn hin(mut self, v: Real) -> Self {
        self.hin = v;
        self
    }

    #[inline]
    pub const fn hmin(mut self, v: Real) -> Self {
        self.hmin = v;
        self
    }

    #[inline]
    pub const fn hmax_inv(mut self, v: Real) -> Self {
        self.hmax_inv = v;
        self
    }

    #[inline]
    pub const fn nlscoef(mut self, v: Real) -> Self {
        self.nlscoef = v;
        self
    }

    #[inline]
    pub fn build<V, F>(self) -> Result<Cvode<V, F>, CvodeError>
    where
        V: NVector,
        F: RhsFn<V>,
    {
        let mut cv = Cvode::<V, F>::create(self.lmm)?;
        cv.mxstep = self.mxstep;
        cv.mxhnil = self.mxhnil;
        cv.hin = self.hin;
        cv.hmin = self.hmin;
        cv.hmax_inv = self.hmax_inv;
        cv.nlscoef = self.nlscoef;
        Ok(cv)
    }
}