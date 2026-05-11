//! CVODE nonlinear solver interface (optimized Rust translation of `cvode_nls.c`).
//!
//! This module provides an idiomatic Rust API for attaching and driving a
//! nonlinear solver inside a CVODE-like time integrator while preserving the
//! numerical behavior of the original C implementation (IEEE-754 `f64`).

use core::any::Any;
use core::marker::PhantomData;

/// Real scalar type used by SUNDIALS.
pub type SunReal = f64;

/// Index type used by SUNDIALS.
pub type SunIndex = usize;

/// Constants from the C implementation.
pub const ONE: SunReal = 1.0;
pub const NLS_MAXCOR: usize = 3;
pub const CRDOWN: SunReal = 0.3;
pub const RDIV: SunReal = 2.0;

/// CVODE error/status mapping from integer return codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    /// Equivalent to null memory pointer / missing required object.
    MemNull,
    /// Illegal input with static diagnostic message.
    IllInput(&'static str),
    /// Nonlinear solver initialization failed.
    NlsInitFail,
    /// Linear setup callback failed.
    LSetupFail,
    /// Linear solve callback failed.
    LSolveFail,
    /// RHS function failed (unrecoverable).
    RhsFuncFail,
    /// RHS function failed (recoverable).
    RhsFuncRecoverable,
    /// Nonlinear convergence test failed recoverably.
    NlsConvRecoverable,
    /// Nonlinear iteration should continue.
    NlsContinue,
    /// Generic nonlinear solver error.
    NonlinearSolver(&'static str),
}

/// Generic vector abstraction replacing `N_Vector_Ops`.
///
/// Implementations should be zero-cost and preserve numerical behavior.
pub trait NVector: Clone {
    /// Weighted RMS norm:
    /// \[
    /// \|x\|_{\mathrm{wrms}} = \sqrt{\frac{1}{N}\sum_i (x_i w_i)^2}
    /// \]
    #[inline]
    fn wrms_norm(&self, w: &Self) -> SunReal;

    /// Compute `z = a*x + b*y`.
    fn linear_sum(a: SunReal, x: &Self, b: SunReal, y: &Self, z: &mut Self);

    /// Compute `z = c*x`.
    fn scale(c: SunReal, x: &Self, z: &mut Self);
}

/// Placeholder trait for linear solver backends.
pub trait LinearSolver {}

/// Placeholder trait for matrix backends.
pub trait SunMatrix {}

/// Nonlinear solver type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonlinearSolverType {
    /// Newton/root-finding style nonlinear solve.
    RootFind,
    /// Fixed-point nonlinear solve.
    FixedPoint,
}

/// Nonlinear solver trait (typed, no `void*`).
pub trait NonlinearSolver<V: NVector, F> {
    fn get_type(&self) -> NonlinearSolverType;
    fn set_max_iters(&mut self, max_iters: usize) -> Result<(), CvodeError>;
    fn initialize(&mut self) -> Result<(), CvodeError>;
    fn get_cur_iter(&self) -> Result<usize, CvodeError>;
    fn set_sys_fn_root(&mut self) -> Result<(), CvodeError>;
    fn set_sys_fn_fp(&mut self) -> Result<(), CvodeError>;
    fn set_conv_test_fn(&mut self) -> Result<(), CvodeError>;
    fn set_lsetup_fn(&mut self, enabled: bool) -> Result<(), CvodeError>;
    fn set_lsolve_fn(&mut self, enabled: bool) -> Result<(), CvodeError>;

    #[inline]
    fn _phantom(&self) -> PhantomData<(V, F)> {
        PhantomData
    }
}

/// RHS function trait.
pub trait RhsFn<V: NVector> {
    fn eval(
        &mut self,
        t: SunReal,
        y: &V,
        fy: &mut V,
        user_data: &mut dyn Any,
    ) -> Result<(), CvodeError>;
}

/// Builder for [`Cvode`] nonlinear-solver-related configuration.
pub struct CvodeBuilder<V, F, NLS>
where
    V: NVector,
    F: RhsFn<V>,
    NLS: NonlinearSolver<V, F>,
{
    cvode: Cvode<V, F, NLS>,
}

impl<V, F, NLS> CvodeBuilder<V, F, NLS>
where
    V: NVector,
    F: RhsFn<V>,
    NLS: NonlinearSolver<V, F>,
{
    #[inline]
    pub fn new(cvode: Cvode<V, F, NLS>) -> Self {
        Self { cvode }
    }

    #[inline]
    pub fn with_nonlinear_solver(mut self, nls: NLS) -> Result<Self, CvodeError> {
        self.cvode.set_nonlinear_solver(nls)?;
        Ok(self)
    }

    #[inline]
    pub fn with_nls_rhs_uses_main(mut self, uses_main: bool) -> Result<Self, CvodeError> {
        self.cvode.set_nls_rhs_fn(uses_main)?;
        Ok(self)
    }

    #[inline]
    pub fn build(self) -> Cvode<V, F, NLS> {
        self.cvode
    }
}

/// CVODE memory/state object.
pub struct Cvode<V, F, NLS>
where
    V: NVector,
    F: RhsFn<V>,
    NLS: NonlinearSolver<V, F>,
{
    pub nls: Option<NLS>,
    pub own_nls: bool,

    pub cv_f: Option<F>,
    pub nls_f_uses_main: bool,

    pub cv_tn: SunReal,
    pub cv_gamma: SunReal,
    pub cv_rl1: SunReal,
    pub cv_h: SunReal,

    /// Nordsieck history array; expects at least `zn[0]` and `zn[1]`.
    pub cv_zn: Vec<V>,
    pub cv_y: V,
    pub cv_ftemp: V,
    pub cv_ewt: V,
    pub cv_vtemp1: V,
    pub cv_vtemp2: V,
    pub cv_vtemp3: V,

    pub cv_acnrm: SunReal,
    pub cv_acnrmcur: bool,
    pub cv_crate: SunReal,
    pub cv_delp: SunReal,

    pub cv_jcur: bool,
    pub convfail_bad_j: bool,
    pub cv_nsetups: usize,
    pub cv_nfe: usize,
    pub cv_nstlp: usize,
    pub cv_nst: usize,
    pub cv_gamrat: SunReal,
    pub cv_gammap: SunReal,

    pub user_data: Box<dyn Any>,
}

impl<V, F, NLS> Cvode<V, F, NLS>
where
    V: NVector,
    F: RhsFn<V>,
    NLS: NonlinearSolver<V, F>,
{
    /// Attach nonlinear solver (`CVodeSetNonlinearSolver`).
    #[inline]
    pub fn set_nonlinear_solver(&mut self, mut nls: NLS) -> Result<(), CvodeError> {
        match nls.get_type() {
            NonlinearSolverType::RootFind => nls.set_sys_fn_root()?,
            NonlinearSolverType::FixedPoint => nls.set_sys_fn_fp()?,
        }
        nls.set_conv_test_fn()?;
        nls.set_max_iters(NLS_MAXCOR)?;
        self.cv_acnrmcur = false;

        if self.cv_f.is_none() {
            return Err(CvodeError::IllInput("The ODE RHS function is NULL"));
        }

        self.nls_f_uses_main = true;
        self.nls = Some(nls);
        self.own_nls = false;
        Ok(())
    }

    /// Set alternative nonlinear-system RHS (`CVodeSetNlsRhsFn` equivalent).
    #[inline]
    pub fn set_nls_rhs_fn(&mut self, f_is_main: bool) -> Result<(), CvodeError> {
        self.nls_f_uses_main = f_is_main;
        Ok(())
    }

    /// Access nonlinear system data (`CVodeGetNonlinearSystemData`).
    #[inline]
    pub fn nonlinear_system_data(
        &mut self,
    ) -> (SunReal, &V, &V, &V, SunReal, SunReal, &V, &mut dyn Any) {
        (
            self.cv_tn,
            &self.cv_zn[0],
            &self.cv_y,
            &self.cv_ftemp,
            self.cv_gamma,
            self.cv_rl1,
            &self.cv_zn[1],
            self.user_data.as_mut(),
        )
    }

    /// Initialize nonlinear solver wrappers (`cvNlsInit`).
    #[inline]
    pub fn nls_init(&mut self, has_lsetup: bool, has_lsolve: bool) -> Result<(), CvodeError> {
        let nls = self.nls.as_mut().ok_or(CvodeError::MemNull)?;
        nls.set_lsetup_fn(has_lsetup)?;
        nls.set_lsolve_fn(has_lsolve)?;
        nls.initialize().map_err(|_| CvodeError::NlsInitFail)
    }

    /// Convergence test (`cvNlsConvTest`).
    ///
    /// Uses the same logic as CVODE:
    /// - compute `del = ||delta||_wrms`
    /// - update convergence rate estimate `crate`
    /// - test `dcon = del * min(1, crate) / tol <= 1`
    /// - detect divergence if `del > RDIV * delp`
    #[inline]
    pub fn nls_conv_test(
        &mut self,
        nls_cur_iter: usize,
        ycor: &V,
        delta: &V,
        tol: SunReal,
        ewt: &V,
    ) -> Result<(), CvodeError> {
        let del = delta.wrms_norm(ewt);

        if nls_cur_iter > 0 {
            self.cv_crate = (CRDOWN * self.cv_crate).max(del / self.cv_delp);
        }

        let dcon = del * ONE.min(self.cv_crate) / tol;

        if dcon <= ONE {
            self.cv_acnrm = if nls_cur_iter == 0 {
                del
            } else {
                ycor.wrms_norm(ewt)
            };
            self.cv_acnrmcur = true;
            return Ok(());
        }

        if nls_cur_iter >= 1 && del > RDIV * self.cv_delp {
            return Err(CvodeError::NlsConvRecoverable);
        }

        self.cv_delp = del;
        Err(CvodeError::NlsContinue)
    }
}