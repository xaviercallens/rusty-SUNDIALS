//! CVODE linear-solver interface (optimized Rust translation scaffold).
//!
//! This module provides an idiomatic Rust API corresponding to the visible
//! behavior of the SUNDIALS C routines around `CVodeSetLinearSolver` and
//! related optional linear-solver setters.
//!
//! # Design mapping from C to Rust
//! - `void* cvode_mem` → typed [`Cvode<F, V>`]
//! - integer return codes → `Result<_, CvodeError>`
//! - `N_Vector_Ops` table → [`NVector`] trait (static dispatch)
//! - manual memory management → RAII-owned [`CvLsMem`]
//! - optional setter calls → [`CvLsConfigBuilder`]
//!
//! Numerical scalar/index aliases preserve SUNDIALS semantics:
//! - `sunrealtype` → `f64`
//! - `sunindextype` → `usize`

use core::fmt;

/// Floating-point scalar type (`sunrealtype`).
pub type Real = f64;

/// Index type (`sunindextype`).
pub type Index = usize;

/// CVODE multistep method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lmm {
    Adams,
    Bdf,
}

/// Errors replacing C return codes.
#[derive(Debug, Clone)]
pub enum CvodeError {
    MemNull(&'static str),
    IllInput(&'static str),
    MemFail(&'static str),
    SunLsFail(&'static str),
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemNull(m) => write!(f, "memory null: {m}"),
            Self::IllInput(m) => write!(f, "illegal input: {m}"),
            Self::MemFail(m) => write!(f, "memory failure: {m}"),
            Self::SunLsFail(m) => write!(f, "linear solver failure: {m}"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Minimal NVector trait (static dispatch, zero-cost abstraction).
pub trait NVector: Clone {
    fn len(&self) -> Index;
    fn const_fill(&mut self, c: Real);
    fn dot_prod(&self, other: &Self) -> Real;
    fn wrms_norm(&self, w: &Self) -> Real;
}

/// Matrix abstraction.
pub trait SunMatrix {}

/// Linear solver type tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearSolverType {
    Direct,
    Iterative,
    MatrixEmbedded,
    Other,
}

/// Linear solver abstraction.
pub trait LinearSolver<V: NVector, M: SunMatrix> {
    fn get_type(&self) -> LinearSolverType;
    fn solve(&mut self, _x: &mut V, _b: &V) -> Result<(), CvodeError>;
    fn set_atimes(&mut self) -> Result<(), CvodeError> {
        Ok(())
    }
    fn set_preconditioner(&mut self) -> Result<(), CvodeError> {
        Ok(())
    }
}

/// RHS function trait.
pub trait RhsFn<V: NVector> {
    fn eval(&mut self, t: Real, y: &V, fy: &mut V) -> Result<(), CvodeError>;
}

/// Internal LS memory (RAII-owned).
pub struct CvLsMem<V, LS, M>
where
    V: NVector,
    LS: LinearSolver<V, M>,
    M: SunMatrix,
{
    pub ls: LS,
    pub a: Option<M>,
    pub iterative: bool,
    pub matrixbased: bool,
    pub jac_dq: bool,
    pub user_linsys: bool,
    pub ytemp: V,
    pub x: V,
    pub nrmfac: Real,
    pub msbj: i64,
    pub jbad: bool,
    pub dgmax_jbad: Real,
    pub eplifac: Real,
    pub scalesol: bool,
}

/// Main CVODE memory.
pub struct Cvode<F, V>
where
    F: RhsFn<V>,
    V: NVector,
{
    pub f: F,
    pub tempv: V,
    pub user_data_present: bool,
    pub lmm: Lmm,
    pub ls_attached: bool,
}

impl<F, V> Cvode<F, V>
where
    F: RhsFn<V>,
    V: NVector,
{
    /// Attach and validate a linear solver.
    ///
    /// Mirrors `CVodeSetLinearSolver` checks:
    /// - compatibility of LS type and matrix presence
    /// - iterative solver callback setup
    /// - default LS parameters and temporary vectors
    pub fn set_linear_solver<LS, M>(
        &mut self,
        mut ls: LS,
        a: Option<M>,
    ) -> Result<CvLsMem<V, LS, M>, CvodeError>
    where
        LS: LinearSolver<V, M>,
        M: SunMatrix,
    {
        let ls_type = ls.get_type();
        let iterative = ls_type != LinearSolverType::Direct;
        let matrixbased = !matches!(
            ls_type,
            LinearSolverType::Iterative | LinearSolverType::MatrixEmbedded
        );

        if ls_type == LinearSolverType::MatrixEmbedded && a.is_some() {
            return Err(CvodeError::IllInput(
                "matrix-embedded LS requires NULL matrix",
            ));
        }

        if iterative {
            if matrixbased && a.is_none() {
                return Err(CvodeError::IllInput(
                    "matrix-iterative LS requires non-NULL matrix",
                ));
            }
            ls.set_atimes()?;
        } else if a.is_none() {
            return Err(CvodeError::IllInput("direct LS requires non-NULL matrix"));
        }

        ls.set_preconditioner()?;

        let ytemp = self.tempv.clone();
        let x = self.tempv.clone();

        let nrmfac = if iterative {
            (ytemp.len() as Real).sqrt()
        } else {
            0.0
        };

        let scalesol = matrixbased && self.lmm == Lmm::Bdf;
        self.ls_attached = true;

        Ok(CvLsMem {
            ls,
            a,
            iterative,
            matrixbased,
            jac_dq: true,
            user_linsys: false,
            ytemp,
            x,
            nrmfac,
            msbj: 50,
            jbad: true,
            dgmax_jbad: 0.2,
            eplifac: 0.05,
            scalesol,
        })
    }
}

/// Builder for optional LS settings (replaces `CVodeSet*` calls).
pub struct CvLsConfigBuilder {
    dgmax_jbad: Option<Real>,
    eplifac: Option<Real>,
    nrmfac: Option<Real>,
    msbj: Option<i64>,
    scalesol: Option<bool>,
}

impl CvLsConfigBuilder {
    #[inline]
    pub const fn new() -> Self {
        Self {
            dgmax_jbad: None,
            eplifac: None,
            nrmfac: None,
            msbj: None,
            scalesol: None,
        }
    }

    #[inline]
    pub const fn delta_gamma_max_bad_jac(mut self, v: Real) -> Self {
        self.dgmax_jbad = Some(v);
        self
    }

    #[inline]
    pub const fn eps_lin(mut self, v: Real) -> Self {
        self.eplifac = Some(v);
        self
    }

    #[inline]
    pub const fn ls_norm_factor(mut self, v: Real) -> Self {
        self.nrmfac = Some(v);
        self
    }

    #[inline]
    pub const fn jac_eval_frequency(mut self, v: i64) -> Self {
        self.msbj = Some(v);
        self
    }

    #[inline]
    pub const fn linear_solution_scaling(mut self, on: bool) -> Self {
        self.scalesol = Some(on);
        self
    }

    #[inline]
    pub fn apply<V, LS, M>(self, mem: &mut CvLsMem<V, LS, M>) -> Result<(), CvodeError>
    where
        V: NVector,
        LS: LinearSolver<V, M>,
        M: SunMatrix,
    {
        if let Some(v) = self.dgmax_jbad {
            mem.dgmax_jbad = if v <= 0.0 { 0.2 } else { v };
        }

        if let Some(v) = self.eplifac {
            if v < 0.0 {
                return Err(CvodeError::IllInput("eplifac must be nonnegative"));
            }
            mem.eplifac = if v == 0.0 { 0.05 } else { v };
        }

        if let Some(v) = self.nrmfac {
            mem.nrmfac = if v > 0.0 {
                v
            } else if v < 0.0 {
                let mut ones = mem.ytemp.clone();
                ones.const_fill(1.0);
                ones.dot_prod(&ones).sqrt()
            } else {
                (mem.ytemp.len() as Real).sqrt()
            };
        }

        if let Some(v) = self.msbj {
            if v < 0 {
                return Err(CvodeError::IllInput(
                    "negative Jacobian evaluation frequency",
                ));
            }
            mem.msbj = if v == 0 { 50 } else { v };
        }

        if let Some(on) = self.scalesol {
            mem.scalesol = on;
        }

        Ok(())
    }
}

impl Default for CvLsConfigBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}