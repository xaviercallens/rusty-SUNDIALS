//! Diagonal linear solver module for a CVODE-like integrator.
//!
//! This module is an idiomatic Rust translation of the SUNDIALS `cvode_diag.c`
//! logic, preserving IEEE-754 `f64` numerical behavior while replacing C-style
//! memory/error handling with RAII and `Result`-based errors.

#![allow(clippy::needless_return)]

use core::fmt;

/// Scalar type used by SUNDIALS.
pub type SunReal = f64;
/// Index type used by SUNDIALS.
pub type SunIndex = usize;

const FRACT: SunReal = 0.1;
const ONE: SunReal = 1.0;

/// Errors corresponding to CVDIAG/CVODE status codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    MemNull,
    LMemNull,
    IllInput(&'static str),
    MemFail,
    InvFail,
    RhsFuncUnrecoverable,
    RhsFuncRecoverable,
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemNull => write!(f, "cvode memory is null/uninitialized"),
            Self::LMemNull => write!(f, "linear solver memory is null/uninitialized"),
            Self::IllInput(msg) => write!(f, "illegal input: {msg}"),
            Self::MemFail => write!(f, "memory allocation failed"),
            Self::InvFail => write!(f, "vector inverse test failed (zero component)"),
            Self::RhsFuncUnrecoverable => write!(f, "RHS function failed unrecoverably"),
            Self::RhsFuncRecoverable => write!(f, "RHS function failed recoverably"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Minimal NVector trait (static dispatch, zero-cost abstraction).
pub trait NVector: Clone {
    fn linear_sum(a: SunReal, x: &Self, b: SunReal, y: &Self, z: &mut Self);
    fn prod(x: &Self, y: &Self, z: &mut Self);
    fn div(x: &Self, y: &Self, z: &mut Self);
    fn add_const(x: &Self, b: SunReal, z: &mut Self);
    fn scale(c: SunReal, x: &Self, z: &mut Self);
    fn inv(x: &Self, z: &mut Self);
    fn inv_test(x: &Self, z: &mut Self) -> bool;
    fn compare(c: SunReal, x: &Self, z: &mut Self);
}

/// RHS callback trait.
pub trait RhsFn<V: NVector> {
    fn eval(&mut self, t: SunReal, y: &V, fy: &mut V) -> Result<(), CvodeError>;
}

/// Placeholder traits requested by workspace conventions.
pub trait LinearSolver {}
pub trait SunMatrix {}

/// Diagonal linear solver memory (owns vectors via RAII).
#[derive(Clone)]
pub struct CvDiagMem<V: NVector> {
    pub di_last_flag: Result<(), CvodeError>,
    pub di_m: V,
    pub di_bit: V,
    pub di_bitcomp: V,
    pub di_nfe_di: usize,
    pub di_gammasv: SunReal,
}

/// Main CVODE memory (typed, generic over RHS and vector type).
pub struct Cvode<F, V>
where
    F: RhsFn<V>,
    V: NVector,
{
    pub cv_tempv: V,
    pub cv_lrw1: usize,
    pub cv_liw1: usize,
    pub cv_rl1: SunReal,
    pub cv_h: SunReal,
    pub cv_tn: SunReal,
    pub cv_gamma: SunReal,
    pub cv_uround: SunReal,
    pub cv_zn1: V,
    pub cv_ewt: V,
    pub cv_f: F,
    pub lmem_diag: Option<CvDiagMem<V>>,
}

impl<F, V> Cvode<F, V>
where
    F: RhsFn<V>,
    V: NVector,
{
    #[inline]
    const fn workspace_scale() -> usize {
        3
    }

    #[inline]
    fn lmem_diag_ref(&self) -> Result<&CvDiagMem<V>, CvodeError> {
        self.lmem_diag.as_ref().ok_or(CvodeError::LMemNull)
    }

    #[inline]
    fn lmem_diag_mut(&mut self) -> Result<&mut CvDiagMem<V>, CvodeError> {
        self.lmem_diag.as_mut().ok_or(CvodeError::LMemNull)
    }

    /// Attach/initialize the diagonal linear solver.
    pub fn cvdiag_attach(&mut self) -> Result<(), CvodeError> {
        let m = self.cv_tempv.clone();
        let bit = self.cv_tempv.clone();
        let bitcomp = self.cv_tempv.clone();

        self.lmem_diag = Some(CvDiagMem {
            di_last_flag: Ok(()),
            di_m: m,
            di_bit: bit,
            di_bitcomp: bitcomp,
            di_nfe_di: 0,
            di_gammasv: ONE,
        });
        Ok(())
    }

    /// Returns `(lenrwLS, leniwLS)`.
    #[inline]
    pub fn cvdiag_get_workspace(&self) -> (usize, usize) {
        let s = Self::workspace_scale();
        (s * self.cv_lrw1, s * self.cv_liw1)
    }

    #[inline]
    pub fn cvdiag_get_num_rhs_evals(&self) -> Result<usize, CvodeError> {
        Ok(self.lmem_diag_ref()?.di_nfe_di)
    }

    #[inline]
    pub fn cvdiag_get_last_flag(&self) -> Result<Result<(), CvodeError>, CvodeError> {
        Ok(self.lmem_diag_ref()?.di_last_flag.clone())
    }

    #[inline]
    pub const fn cvdiag_get_return_flag_name(flag: &Result<(), CvodeError>) -> &'static str {
        match flag {
            Ok(()) => "CVDIAG_SUCCESS",
            Err(CvodeError::MemNull) => "CVDIAG_MEM_NULL",
            Err(CvodeError::LMemNull) => "CVDIAG_LMEM_NULL",
            Err(CvodeError::IllInput(_)) => "CVDIAG_ILL_INPUT",
            Err(CvodeError::MemFail) => "CVDIAG_MEM_FAIL",
            Err(CvodeError::InvFail) => "CVDIAG_INV_FAIL",
            Err(CvodeError::RhsFuncUnrecoverable) => "CVDIAG_RHSFUNC_UNRECVR",
            Err(CvodeError::RhsFuncRecoverable) => "CVDIAG_RHSFUNC_RECVR",
        }
    }

    /// Initialize diagonal solver counters.
    pub fn cvdiag_init(&mut self) -> Result<(), CvodeError> {
        let lm = self.lmem_diag_mut()?;
        lm.di_nfe_di = 0;
        lm.di_last_flag = Ok(());
        Ok(())
    }

    /// Setup diagonal approximation:
    /// \( M = I - \gamma J \), with diagonal finite-difference Jacobian.
    pub fn cvdiag_setup(
        &mut self,
        ypred: &V,
        fpred: &V,
        jcur: &mut bool,
        vtemp1: &mut V,
        vtemp2: &mut V,
    ) -> Result<(), CvodeError> {
        let r = FRACT * self.cv_rl1;
        V::linear_sum(self.cv_h, fpred, -ONE, &self.cv_zn1, vtemp1);
        V::linear_sum(r, vtemp1, ONE, ypred, vtemp2);

        {
            let lm = self.lmem_diag_mut()?;
            self.cv_f.eval(self.cv_tn, vtemp2, &mut lm.di_m)?;
            lm.di_nfe_di += 1;

            V::linear_sum(ONE, &lm.di_m, -ONE, fpred, &mut lm.di_m);
            V::linear_sum(FRACT, vtemp1, -self.cv_h, &lm.di_m, &mut lm.di_m);
            V::prod(vtemp1, &self.cv_ewt, vtemp2);
            V::compare(self.cv_uround, vtemp2, &mut lm.di_bit);
            V::add_const(&lm.di_bit, -ONE, &mut lm.di_bitcomp);
            V::prod(vtemp1, &lm.di_bit, vtemp2);
            V::linear_sum(FRACT, vtemp2, -ONE, &lm.di_bitcomp, vtemp2);
            V::div(&lm.di_m, vtemp2, &mut lm.di_m);
            V::prod(&lm.di_m, &lm.di_bit, &mut lm.di_m);
            V::linear_sum(ONE, &lm.di_m, -ONE, &lm.di_bitcomp, &mut lm.di_m);

            if !V::inv_test(&lm.di_m, &mut lm.di_m) {
                lm.di_last_flag = Err(CvodeError::InvFail);
                return Err(CvodeError::InvFail);
            }

            lm.di_gammasv = self.cv_gamma;
            lm.di_last_flag = Ok(());
        }

        *jcur = true;
        Ok(())
    }

    /// Solve \( Mx=b \) using stored diagonal inverse in `M`.
    pub fn cvdiag_solve(&mut self, b: &mut V) -> Result<(), CvodeError> {
        let lm = self.lmem_diag_mut()?;

        if lm.di_gammasv != self.cv_gamma {
            let r = self.cv_gamma / lm.di_gammasv;
            V::inv(&lm.di_m, &mut lm.di_m);
            V::add_const(&lm.di_m, -ONE, &mut lm.di_m);
            V::scale(r, &lm.di_m, &mut lm.di_m);
            V::add_const(&lm.di_m, ONE, &mut lm.di_m);

            if !V::inv_test(&lm.di_m, &mut lm.di_m) {
                lm.di_last_flag = Err(CvodeError::InvFail);
                return Err(CvodeError::InvFail);
            }
            lm.di_gammasv = self.cv_gamma;
        }

        V::prod(b, &lm.di_m, b);
        lm.di_last_flag = Ok(());
        Ok(())
    }

    /// Free diagonal solver memory (explicit API; RAII also handles drop).
    #[inline]
    pub fn cvdiag_free(&mut self) {
        self.lmem_diag = None;
    }
}