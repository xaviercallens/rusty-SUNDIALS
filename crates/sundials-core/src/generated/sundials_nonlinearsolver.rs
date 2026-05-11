//! Generic SUNNonlinearSolver core infrastructure (optimized Rust translation of SUNDIALS base layer).
//!
//! This module provides the base nonlinear solver object and dispatch layer used by
//! concrete nonlinear solver implementations (e.g., Newton, fixed-point).
//!
//! Design goals:
//! - `Result<T, NlsError>` in place of C integer return codes
//! - trait-based operation table in place of nullable C function pointers
//! - RAII ownership and no manual memory management
//! - no `unsafe` in public API
//! - IEEE-754 preserving scalar type (`f64`)

use std::fmt;
use std::sync::Arc;

/// Floating-point scalar type (`sunrealtype` in SUNDIALS).
pub type SunReal = f64;

/// Index type (`sunindextype` in SUNDIALS).
pub type SunIndex = usize;

/// Generic nonlinear solver error codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NlsError {
    /// Allocation failure (maps C `SUN_ERR_MALLOC_FAIL`).
    MallocFail,
    /// Corrupt or null argument (maps C `SUN_ERR_ARG_CORRUPT`).
    ArgCorrupt,
    /// Incompatible argument combination (maps C `SUN_ERR_ARG_INCOMPATIBLE`).
    ArgIncompatible,
    /// Required operation is missing.
    MissingOperation(&'static str),
    /// Parse error in command-line option.
    InvalidOptionValue(String),
    /// Backend/implementation-specific error.
    Backend(String),
}

impl fmt::Display for NlsError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MallocFail => write!(f, "memory allocation failed"),
            Self::ArgCorrupt => write!(f, "argument is corrupt or null"),
            Self::ArgIncompatible => write!(f, "incompatible argument"),
            Self::MissingOperation(op) => {
                write!(f, "missing required nonlinear solver operation: {op}")
            }
            Self::InvalidOptionValue(v) => write!(f, "invalid option value: {v}"),
            Self::Backend(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for NlsError {}

/// Nonlinear solver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonlinearSolverType {
    Unknown,
    Newton,
    FixedPoint,
}

/// Minimal profiler placeholder.
#[derive(Debug, Default)]
pub struct SunProfiler;

/// SUNDIALS context.
#[derive(Debug, Default)]
pub struct SunContext {
    pub profiler: Option<Arc<SunProfiler>>,
}

/// Generic vector abstraction replacing `N_Vector`.
pub trait NVector: Clone {
    /// Vector length.
    fn len(&self) -> usize;
    /// Returns `true` if the vector has length 0.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Nonlinear system callback: computes residual \(F(y)\).
pub trait SysFn<V: NVector, M>: Send + Sync {
    fn eval(&self, y: &V, mem: &mut M) -> Result<(), NlsError>;
}

/// Optional linear setup callback.
pub trait LSetupFn<M>: Send + Sync {
    fn setup(&self, mem: &mut M) -> Result<(), NlsError>;
}

/// Optional linear solve callback.
pub trait LSolveFn<V: NVector, M>: Send + Sync {
    fn solve(&self, rhs: &V, mem: &mut M) -> Result<(), NlsError>;
}

/// Optional convergence test callback.
pub trait ConvTestFn<V: NVector, M, CData>: Send + Sync {
    fn test(&self, y: &V, data: &mut CData, mem: &mut M) -> Result<bool, NlsError>;
}

/// Operation table for nonlinear solvers (trait-based replacement for C function table).
pub trait NonlinearSolverOps<V: NVector, M, CData>: Send {
    fn get_type(&self) -> NonlinearSolverType;

    #[inline]
    fn initialize(&mut self) -> Result<(), NlsError> {
        Ok(())
    }

    #[inline]
    fn setup(&mut self, _y: &V, _mem: &mut M) -> Result<(), NlsError> {
        Ok(())
    }

    fn solve(
        &mut self,
        y0: &V,
        y: &mut V,
        w: &V,
        tol: SunReal,
        call_lsetup: bool,
        mem: &mut M,
    ) -> Result<(), NlsError>;

    #[inline]
    fn set_sys_fn(&mut self, _sys: Arc<dyn SysFn<V, M>>) -> Result<(), NlsError> {
        Ok(())
    }

    #[inline]
    fn set_lsetup_fn(&mut self, _f: Arc<dyn LSetupFn<M>>) -> Result<(), NlsError> {
        Ok(())
    }

    #[inline]
    fn set_lsolve_fn(&mut self, _f: Arc<dyn LSolveFn<V, M>>) -> Result<(), NlsError> {
        Ok(())
    }

    #[inline]
    fn set_conv_test_fn(
        &mut self,
        _f: Arc<dyn ConvTestFn<V, M, CData>>,
        _data: CData,
    ) -> Result<(), NlsError>
    where
        CData: Clone,
    {
        Ok(())
    }

    #[inline]
    fn set_options(
        &mut self,
        _nls_id: Option<&str>,
        _file_name: Option<&str>,
        _argv: &[String],
    ) -> Result<(), NlsError> {
        Ok(())
    }

    #[inline]
    fn set_max_iters(&mut self, _maxiters: i32) -> Result<(), NlsError> {
        Ok(())
    }

    #[inline]
    fn get_num_iters(&self) -> Result<i64, NlsError> {
        Ok(0)
    }

    #[inline]
    fn get_cur_iter(&self) -> Result<i32, NlsError> {
        Ok(-1)
    }

    #[inline]
    fn get_num_conv_fails(&self) -> Result<i64, NlsError> {
        Ok(0)
    }
}

/// Generic nonlinear solver object.
pub struct SunNonlinearSolver<V: NVector, M, CData> {
    pub sunctx: Arc<SunContext>,
    ops: Box<dyn NonlinearSolverOps<V, M, CData>>,
}

impl<V: NVector, M, CData> SunNonlinearSolver<V, M, CData> {
    const DEFAULT_ID: &'static str = "sunnonlinearsolver";
    const MAX_ITERS_KEY: &'static str = "max_iters";

    /// Create a new nonlinear solver from an implementation ops object.
    #[inline]
    pub fn new(
        sunctx: Arc<SunContext>,
        ops: Box<dyn NonlinearSolverOps<V, M, CData>>,
    ) -> Result<Self, NlsError> {
        Ok(Self { sunctx, ops })
    }

    /// Get solver type.
    #[inline]
    pub fn get_type(&self) -> NonlinearSolverType {
        self.ops.get_type()
    }

    /// Initialize solver.
    #[inline]
    pub fn initialize(&mut self) -> Result<(), NlsError> {
        self.ops.initialize()
    }

    /// Setup nonlinear solve.
    #[inline]
    pub fn setup(&mut self, y: &V, mem: &mut M) -> Result<(), NlsError> {
        self.ops.setup(y, mem)
    }

    /// Solve nonlinear system.
    #[inline]
    pub fn solve(
        &mut self,
        y0: &V,
        y: &mut V,
        w: &V,
        tol: SunReal,
        call_lsetup: bool,
        mem: &mut M,
    ) -> Result<(), NlsError> {
        self.ops.solve(y0, y, w, tol, call_lsetup, mem)
    }

    /// Set nonlinear system function (required by concrete methods).
    #[inline]
    pub fn set_sys_fn(&mut self, sys: Arc<dyn SysFn<V, M>>) -> Result<(), NlsError> {
        self.ops.set_sys_fn(sys)
    }

    /// Set optional linear setup callback.
    #[inline]
    pub fn set_lsetup_fn(&mut self, f: Arc<dyn LSetupFn<M>>) -> Result<(), NlsError> {
        self.ops.set_lsetup_fn(f)
    }

    /// Set optional linear solve callback.
    #[inline]
    pub fn set_lsolve_fn(&mut self, f: Arc<dyn LSolveFn<V, M>>) -> Result<(), NlsError> {
        self.ops.set_lsolve_fn(f)
    }

    /// Set optional convergence test callback.
    #[inline]
    pub fn set_conv_test_fn(
        &mut self,
        f: Arc<dyn ConvTestFn<V, M, CData>>,
        data: CData,
    ) -> Result<(), NlsError>
    where
        CData: Clone,
    {
        self.ops.set_conv_test_fn(f, data)
    }

    /// Set options from command-line style arguments.
    ///
    /// Base-class options:
    /// - `{id}.max_iters <int>`
    ///
    /// where `id` defaults to `"sunnonlinearsolver"` if not provided.
    #[inline]
    pub fn set_options(
        &mut self,
        nls_id: Option<&str>,
        file_name: Option<&str>,
        argv: &[String],
    ) -> Result<(), NlsError> {
        if file_name.is_some_and(|name| !name.is_empty()) {
            return Err(NlsError::ArgIncompatible);
        }

        self.set_from_command_line(nls_id, argv)?;
        self.ops.set_options(nls_id, file_name, argv)
    }

    #[inline]
    fn set_from_command_line(
        &mut self,
        nls_id: Option<&str>,
        argv: &[String],
    ) -> Result<(), NlsError> {
        let id = nls_id.filter(|s| !s.is_empty()).unwrap_or(Self::DEFAULT_ID);
        let prefix = format!("{id}.");

        let mut iter = argv.iter().skip(1).peekable();
        while let Some(arg) = iter.next() {
            if !arg.starts_with(&prefix) {
                continue;
            }

            let key = &arg[prefix.len()..];
            if key == Self::MAX_ITERS_KEY {
                let value = iter
                    .next()
                    .ok_or_else(|| NlsError::InvalidOptionValue("missing value for max_iters".into()))?;
                let maxiters = value
                    .parse::<i32>()
                    .map_err(|_| NlsError::InvalidOptionValue(value.to_string()))?;
                self.set_max_iters(maxiters)?;
            }
        }

        Ok(())
    }

    /// Set maximum nonlinear iterations.
    #[inline]
    pub fn set_max_iters(&mut self, maxiters: i32) -> Result<(), NlsError> {
        self.ops.set_max_iters(maxiters)
    }

    /// Get total number of nonlinear iterations.
    #[inline]
    pub fn get_num_iters(&self) -> Result<i64, NlsError> {
        self.ops.get_num_iters()
    }

    /// Get current nonlinear iteration index.
    #[inline]
    pub fn get_cur_iter(&self) -> Result<i32, NlsError> {
        self.ops.get_cur_iter()
    }

    /// Get total number of nonlinear convergence failures.
    #[inline]
    pub fn get_num_conv_fails(&self) -> Result<i64, NlsError> {
        self.ops.get_num_conv_fails()
    }
}