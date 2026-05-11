//! Generic SUNLinearSolver base implementation (Rust translation of SUNDIALS C core).
//!
//! This module provides the base "class-like" linear solver object used by concrete
//! linear solver implementations. It mirrors the C behavior from SUNDIALS while
//! exposing an idiomatic Rust API with `Result`-based error handling and RAII.
//!
//! # Numerical notes
//!
//! The linear solve operation conceptually computes
//!
//! \[
//! A x = b
//! \]
//!
//! up to a user-provided tolerance `tol` in `f64` (IEEE-754 semantics preserved).
//! This base module does not impose a specific algorithm; concrete implementations
//! provide direct, iterative, or matrix-embedded methods via trait dispatch.

use core::fmt;
use core::marker::PhantomData;

/// SUNDIALS real type (`sunrealtype`).
pub type SunReal = f64;

/// SUNDIALS index type (`sunindextype`).
pub type SunIndex = usize;

/// Error type replacing integer return codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// Equivalent to argument corruption / null input in C.
    NullInput(&'static str),
    /// Incompatible arguments.
    ArgIncompatible(&'static str),
    /// Parse error for command-line options.
    InvalidOptionValue { key: String, value: String },
    /// Generic operation failure.
    OperationFailed(&'static str),
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullInput(s) => write!(f, "null/invalid input: {s}"),
            Self::ArgIncompatible(s) => write!(f, "incompatible argument: {s}"),
            Self::InvalidOptionValue { key, value } => {
                write!(f, "invalid option value for {key}: {value}")
            }
            Self::OperationFailed(s) => write!(f, "operation failed: {s}"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Minimal NVector trait (static dispatch, zero-cost abstraction).
pub trait NVector: Clone {}

/// Marker trait for SUNMatrix.
pub trait SunMatrix {}

/// Marker trait for linear solver backends.
pub trait LinearSolver {}

/// Linear solver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunLinearSolverType {
    Direct,
    Iterative,
    MatrixEmbedded,
    Custom,
}

/// Linear solver ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SunLinearSolverId {
    Custom,
    Other(&'static str),
}

/// Context object (logger/profiler hooks can be added here).
#[derive(Debug, Clone, Default)]
pub struct SunContext {}

/// Operations supported by a concrete linear solver implementation.
///
/// Default methods mirror C behavior where missing function pointers imply
/// no-op success or default values.
pub trait SunLinearSolverOps<V: NVector, M: SunMatrix> {
    fn get_type(&self) -> SunLinearSolverType;

    #[inline]
    fn get_id(&self) -> SunLinearSolverId {
        SunLinearSolverId::Custom
    }

    #[inline]
    fn set_atimes(&mut self) -> Result<(), CvodeError> {
        Ok(())
    }

    #[inline]
    fn set_preconditioner(&mut self) -> Result<(), CvodeError> {
        Ok(())
    }

    #[inline]
    fn set_scaling_vectors(&mut self, _s1: &V, _s2: &V) -> Result<(), CvodeError> {
        Ok(())
    }

    #[inline]
    fn set_options(
        &mut self,
        _lsid: Option<&str>,
        _file_name: Option<&str>,
        _argv: &[String],
    ) -> Result<(), CvodeError> {
        Ok(())
    }

    #[inline]
    fn set_zero_guess(&mut self, _onoff: bool) -> Result<(), CvodeError> {
        Ok(())
    }

    #[inline]
    fn initialize(&mut self) -> Result<(), CvodeError> {
        Ok(())
    }

    #[inline]
    fn setup(&mut self, _a: &M) -> Result<(), CvodeError> {
        Ok(())
    }

    fn solve(&mut self, _a: &M, _x: &mut V, _b: &V, _tol: SunReal) -> Result<(), CvodeError>;

    #[inline]
    fn num_iters(&self) -> i32 {
        0
    }

    #[inline]
    fn res_norm(&self) -> SunReal {
        0.0
    }

    #[inline]
    fn resid(&self) -> Option<V> {
        None
    }

    #[inline]
    fn last_flag(&self) -> SunIndex {
        0
    }

    #[inline]
    fn space(&self) -> Result<(i64, i64), CvodeError> {
        Ok((0, 0))
    }
}

/// Generic SUNLinearSolver object.
pub struct SunLinearSolver<V: NVector, M: SunMatrix, O: SunLinearSolverOps<V, M>> {
    ops: O,
    _phantom_v: PhantomData<V>,
    _phantom_m: PhantomData<M>,
    pub sunctx: SunContext,
}

impl<V: NVector, M: SunMatrix, O: SunLinearSolverOps<V, M>> SunLinearSolver<V, M, O> {
    /// Create a new solver object from concrete ops.
    #[inline]
    pub const fn new(ops: O, sunctx: SunContext) -> Self {
        Self {
            ops,
            _phantom_v: PhantomData,
            _phantom_m: PhantomData,
            sunctx,
        }
    }

    #[inline]
    pub fn get_type(&self) -> SunLinearSolverType {
        self.ops.get_type()
    }

    #[inline]
    pub fn get_id(&self) -> SunLinearSolverId {
        self.ops.get_id()
    }

    #[inline]
    pub fn set_atimes(&mut self) -> Result<(), CvodeError> {
        self.ops.set_atimes()
    }

    #[inline]
    pub fn set_preconditioner(&mut self) -> Result<(), CvodeError> {
        self.ops.set_preconditioner()
    }

    #[inline]
    pub fn set_scaling_vectors(&mut self, s1: &V, s2: &V) -> Result<(), CvodeError> {
        self.ops.set_scaling_vectors(s1, s2)
    }

    /// Set options from file/CLI, matching C flow:
    /// 1) validate file option support
    /// 2) process base-class CLI options
    /// 3) delegate remaining options to implementation
    #[inline]
    pub fn set_options(
        &mut self,
        lsid: Option<&str>,
        file_name: Option<&str>,
        argv: &[String],
    ) -> Result<(), CvodeError> {
        if file_name.is_some_and(|name| !name.is_empty()) {
            return Err(CvodeError::ArgIncompatible(
                "file-based options are unimplemented",
            ));
        }

        if !argv.is_empty() {
            self.set_from_command_line(lsid, argv)?;
        }

        self.ops.set_options(lsid, file_name, argv)
    }

    /// Internal base-class command-line parser.
    ///
    /// Recognized option:
    /// - `<prefix>.zero_guess <0|1>`
    ///
    /// where `prefix = lsid.unwrap_or("sunlinearsolver")`.
    #[inline]
    fn set_from_command_line(
        &mut self,
        lsid: Option<&str>,
        argv: &[String],
    ) -> Result<(), CvodeError> {
        let id = lsid.filter(|s| !s.is_empty()).unwrap_or("sunlinearsolver");
        let prefix = format!("{id}.");
        let key_full = format!("{prefix}zero_guess");

        let mut iter = argv.iter().enumerate().skip(1).peekable();
        while let Some((_, arg)) = iter.next() {
            if !arg.starts_with(&prefix) {
                continue;
            }

            let key = &arg[prefix.len()..];
            if key == "zero_guess" {
                let raw = iter
                    .next()
                    .map(|(_, s)| s.as_str())
                    .ok_or_else(|| CvodeError::InvalidOptionValue {
                        key: key_full.clone(),
                        value: "<missing>".to_string(),
                    })?;

                let parsed: i32 = raw.parse().map_err(|_| CvodeError::InvalidOptionValue {
                    key: key_full.clone(),
                    value: raw.to_string(),
                })?;

                self.set_zero_guess(parsed != 0)?;
            }
        }

        Ok(())
    }

    #[inline]
    pub fn set_zero_guess(&mut self, onoff: bool) -> Result<(), CvodeError> {
        self.ops.set_zero_guess(onoff)
    }

    #[inline]
    pub fn initialize(&mut self) -> Result<(), CvodeError> {
        self.ops.initialize()
    }

    #[inline]
    pub fn setup(&mut self, a: &M) -> Result<(), CvodeError> {
        self.ops.setup(a)
    }

    #[inline]
    pub fn solve(&mut self, a: &M, x: &mut V, b: &V, tol: SunReal) -> Result<(), CvodeError> {
        self.ops.solve(a, x, b, tol)
    }

    #[inline]
    pub fn num_iters(&self) -> i32 {
        self.ops.num_iters()
    }

    #[inline]
    pub fn res_norm(&self) -> SunReal {
        self.ops.res_norm()
    }

    #[inline]
    pub fn resid(&self) -> Option<V> {
        self.ops.resid()
    }

    #[inline]
    pub fn last_flag(&self) -> SunIndex {
        self.ops.last_flag()
    }

    #[inline]
    pub fn space(&self) -> Result<(i64, i64), CvodeError> {
        self.ops.space()
    }
}

/// Empty ops implementation corresponding to `SUNLinSolNewEmpty`.
#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyLinearSolverOps;

impl<V: NVector, M: SunMatrix> SunLinearSolverOps<V, M> for EmptyLinearSolverOps {
    #[inline]
    fn get_type(&self) -> SunLinearSolverType {
        SunLinearSolverType::Custom
    }

    #[inline]
    fn solve(&mut self, _a: &M, _x: &mut V, _b: &V, _tol: SunReal) -> Result<(), CvodeError> {
        Err(CvodeError::OperationFailed(
            "solve not implemented for empty linear solver",
        ))
    }
}

/// Rust equivalent of `SUNLinSolNewEmpty`.
#[inline]
pub const fn sun_lin_sol_new_empty<V: NVector, M: SunMatrix>(
    sunctx: SunContext,
) -> SunLinearSolver<V, M, EmptyLinearSolverOps> {
    SunLinearSolver::new(EmptyLinearSolverOps, sunctx)
}

/// Rust equivalent of `SUNLinSolFreeEmpty`.
///
/// In Rust this is a no-op helper since dropping handles cleanup automatically.
#[inline]
pub fn sun_lin_sol_free_empty<V: NVector, M: SunMatrix, O: SunLinearSolverOps<V, M>>(
    _solver: SunLinearSolver<V, M, O>,
) {
}