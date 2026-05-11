//! CVODE projection support (Rust translation of `cvode_proj.c`).
//!
//! This module provides projection configuration and execution logic for CVODE,
//! translated to idiomatic Rust with typed errors and RAII ownership.

#![allow(clippy::module_name_repetitions)]

use core::fmt;

/// Real scalar type (`sunrealtype` in SUNDIALS C).
pub type SunReal = f64;

/// Index type (`sunindextype` in SUNDIALS C).
pub type SunIndex = usize;

pub const ZERO: SunReal = 0.0;
pub const ONE: SunReal = 1.0;
pub const ONEPSM: SunReal = 1.000001;

pub const PROJ_MAX_FAILS: i32 = 10;
pub const PROJ_EPS: SunReal = 0.1;
pub const PROJ_FAIL_ETA: SunReal = 0.25;

/// CVODE linear multistep method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lmm {
    Bdf,
    Adams,
}

/// CVODE projection-related errors and status conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    MemNull,
    ProjMemNull,
    IllInput(&'static str),
    MemFail,
    ProjFuncFail,
    RepeatedProjFuncErr,
    PredictAgain,
}

impl fmt::Display for CvodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemNull => write!(f, "CVODE memory is null/uninitialized"),
            Self::ProjMemNull => write!(f, "Projection memory is null/uninitialized"),
            Self::IllInput(msg) => write!(f, "Illegal input: {msg}"),
            Self::MemFail => write!(f, "Memory allocation failure"),
            Self::ProjFuncFail => write!(f, "Projection function failed unrecoverably"),
            Self::RepeatedProjFuncErr => write!(f, "Repeated recoverable projection failures"),
            Self::PredictAgain => write!(f, "Step should be predicted again"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Minimal NVector trait (static dispatch, zero-cost abstraction).
pub trait NVector: Clone {
    fn scale(c: SunReal, x: &Self, z: &mut Self);
    fn wrms_norm(x: &Self, w: &Self) -> SunReal;
}

/// Linear solver abstraction placeholder.
pub trait LinearSolver {}

/// Matrix abstraction placeholder.
pub trait SunMatrix {}

/// Projection callback trait.
pub trait ProjFn<V: NVector, FData> {
    fn project(
        &mut self,
        t: SunReal,
        y: &V,
        acor_p: &mut V,
        eps_proj: SunReal,
        err_p: Option<&mut V>,
        user_data: &mut FData,
    ) -> Result<(), i32>;
}

/// Projection memory block.
#[derive(Clone)]
pub struct CvodeProjMem<P> {
    pub internal_proj: bool,
    pub err_proj: bool,
    pub first_proj: bool,
    pub freq: i64,
    pub nstlprj: i64,
    pub max_fails: i32,
    pub pfun: Option<P>,
    pub eps_proj: SunReal,
    pub eta_pfail: SunReal,
    pub nproj: i64,
    pub npfails: i64,
}

impl<P> Default for CvodeProjMem<P> {
    #[inline]
    fn default() -> Self {
        Self {
            internal_proj: true,
            err_proj: true,
            first_proj: true,
            freq: 1,
            nstlprj: 0,
            max_fails: PROJ_MAX_FAILS,
            pfun: None,
            eps_proj: PROJ_EPS,
            eta_pfail: PROJ_FAIL_ETA,
            nproj: 0,
            npfails: 0,
        }
    }
}

#[inline]
const fn default_freq(freq: i64) -> i64 {
    if freq < 0 { 1 } else { freq }
}

#[inline]
const fn default_max_fails(max_fails: i32) -> i32 {
    if max_fails < 1 { PROJ_MAX_FAILS } else { max_fails }
}

#[inline]
const fn default_eps(eps: SunReal) -> SunReal {
    if eps <= ZERO { PROJ_EPS } else { eps }
}

#[inline]
const fn default_eta(eta: SunReal) -> SunReal {
    if eta <= ZERO || eta > ONE {
        PROJ_FAIL_ETA
    } else {
        eta
    }
}

/// Main CVODE memory.
pub struct Cvode<V, P, FData>
where
    V: NVector,
    P: ProjFn<V, FData>,
{
    pub cv_lmm: Lmm,
    pub proj_mem: Option<CvodeProjMem<P>>,
    pub proj_enabled: bool,
    pub proj_applied: bool,

    pub cv_tn: SunReal,
    pub cv_y: V,
    pub cv_acor: V,
    pub cv_tempv: V,
    pub cv_ftemp: V,
    pub cv_ewt: V,
    pub cv_acnrm: SunReal,

    pub cv_h: SunReal,
    pub cv_hmin: SunReal,
    pub cv_eta: SunReal,
    pub cv_etamax: SunReal,

    pub user_data: FData,
}

impl<V, P, FData> Cvode<V, P, FData>
where
    V: NVector,
    P: ProjFn<V, FData>,
{
    #[inline]
    fn access_proj_mem_mut(&mut self) -> Result<&mut CvodeProjMem<P>, CvodeError> {
        self.proj_mem.as_mut().ok_or(CvodeError::ProjMemNull)
    }

    #[inline]
    fn proj_create_if_needed(&mut self) {
        if self.proj_mem.is_none() {
            self.proj_mem = Some(CvodeProjMem::default());
        }
    }

    /// Set user projection function.
    #[inline]
    pub fn set_proj_fn(&mut self, pfun: P) -> Result<(), CvodeError> {
        if self.cv_lmm != Lmm::Bdf {
            return Err(CvodeError::IllInput(
                "Projection is only supported with BDF methods.",
            ));
        }
        self.proj_create_if_needed();
        let pm = self.access_proj_mem_mut()?;
        pm.internal_proj = false;
        pm.pfun = Some(pfun);
        self.proj_enabled = true;
        Ok(())
    }

    /// Enable/disable projection error estimation.
    #[inline]
    pub fn set_proj_err_est(&mut self, onoff: bool) -> Result<(), CvodeError> {
        self.access_proj_mem_mut()?.err_proj = onoff;
        Ok(())
    }

    /// Set projection frequency.
    #[inline]
    pub fn set_proj_frequency(&mut self, freq: i64) -> Result<(), CvodeError> {
        let pm = self.access_proj_mem_mut()?;
        pm.freq = default_freq(freq);
        self.proj_enabled = pm.freq != 0;
        Ok(())
    }

    /// Set maximum number of projection failures per step attempt.
    #[inline]
    pub fn set_max_num_proj_fails(&mut self, max_fails: i32) -> Result<(), CvodeError> {
        self.access_proj_mem_mut()?.max_fails = default_max_fails(max_fails);
        Ok(())
    }

    /// Set projection tolerance.
    #[inline]
    pub fn set_eps_proj(&mut self, eps: SunReal) -> Result<(), CvodeError> {
        self.access_proj_mem_mut()?.eps_proj = default_eps(eps);
        Ok(())
    }

    /// Set step-size reduction factor after projection failure.
    #[inline]
    pub fn set_proj_fail_eta(&mut self, eta: SunReal) -> Result<(), CvodeError> {
        self.access_proj_mem_mut()?.eta_pfail = default_eta(eta);
        Ok(())
    }

    /// Get number of projection evaluations.
    #[inline]
    pub fn get_num_proj_evals(&mut self) -> Result<i64, CvodeError> {
        Ok(self.access_proj_mem_mut()?.nproj)
    }

    /// Get number of projection failures.
    #[inline]
    pub fn get_num_proj_fails(&mut self) -> Result<i64, CvodeError> {
        Ok(self.access_proj_mem_mut()?.npfails)
    }

    /// Initialize projection counters/flags.
    #[inline]
    pub fn proj_init(&mut self) -> Result<(), CvodeError> {
        let pm = self.access_proj_mem_mut()?;
        pm.first_proj = true;
        pm.nstlprj = 0;
        pm.nproj = 0;
        pm.npfails = 0;
        Ok(())
    }

    /// Free projection memory (RAII: drop by setting `None`).
    #[inline]
    pub fn proj_free(&mut self) {
        self.proj_mem = None;
    }

    /// Perform projection step.
    #[inline]
    pub fn do_projection(
        &mut self,
        nflag: &mut i32,
        saved_t: SunReal,
        npfail: &mut i32,
    ) -> Result<(), CvodeError> {
        let (err_proj, eps_proj, eta_pfail, max_fails) = {
            let pm = self.access_proj_mem_mut()?;
            (pm.err_proj, pm.eps_proj, pm.eta_pfail, pm.max_fails)
        };

        let mut errp_local = if err_proj {
            let mut e = self.cv_ftemp.clone();
            V::scale(ONE, &self.cv_acor, &mut e);
            Some(e)
        } else {
            None
        };

        let mut acorp = self.cv_tempv.clone();

        let ret = {
            let cv_tn = self.cv_tn;
            let cv_y = &self.cv_y;
            let user_data = &mut self.user_data;
            let pm = self.access_proj_mem_mut()?;
            let pfun = pm
                .pfun
                .as_mut()
                .ok_or(CvodeError::IllInput("The projection function is NULL."))?;
            pfun.project(cv_tn, cv_y, &mut acorp, eps_proj, errp_local.as_mut(), user_data)
        };

        {
            let pm = self.access_proj_mem_mut()?;
            pm.nproj += 1;
            pm.first_proj = false;
        }

        match ret {
            Ok(()) => {
                if let Some(errp) = errp_local.as_ref() {
                    self.cv_acnrm = V::wrms_norm(errp, &self.cv_ewt);
                }
                self.proj_applied = true;
                Ok(())
            }
            Err(code) => {
                self.access_proj_mem_mut()?.npfails += 1;
                self.restore(saved_t);

                if code < 0 {
                    return Err(CvodeError::ProjFuncFail);
                }

                *npfail += 1;
                self.cv_etamax = ONE;

                if self.cv_h.abs() <= self.cv_hmin * ONEPSM || *npfail == max_fails {
                    return Err(CvodeError::RepeatedProjFuncErr);
                }

                self.cv_eta = eta_pfail.max(self.cv_hmin / self.cv_h.abs());
                *nflag = -100; // PREV_PROJ_FAIL placeholder
                self.rescale();
                Err(CvodeError::PredictAgain)
            }
        }
    }

    #[inline]
    fn restore(&mut self, _saved_t: SunReal) {}

    #[inline]
    fn rescale(&mut self) {}
}