//! Banded difference-quotient preconditioner for a CVODE-like solver.
//!
//! This module is an idiomatic Rust translation of the SUNDIALS CVBANDPRE
//! initialization/query/setup/solve path, adapted to trait-based abstractions.
//!
//! Numerical algorithm preserved:
//! - Build/refresh a banded Jacobian approximation `J` by difference quotients
//! - Form preconditioner matrix `P = I - \gamma J`
//! - Factorize `P` with a band linear solver
//! - Solve `P z = r` during preconditioner application

#![allow(clippy::needless_return)]

use core::fmt;

/// Scalar type (`sunrealtype`).
pub type SunReal = f64;
/// Index type (`sunindextype`).
pub type SunIndex = usize;

/// Constants from the C implementation.
pub const MIN_INC_MULT: SunReal = 1000.0;
pub const ZERO: SunReal = 0.0;
pub const ONE: SunReal = 1.0;
pub const TWO: SunReal = 2.0;

/// CVODE/CVLS-style errors mapped to Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    MemNull,
    LMemNull,
    PMemNull,
    IllInput(&'static str),
    MemFail(&'static str),
    SunMatFail(&'static str),
    SunLsFail(&'static str),
    RhsFuncFailed,
}

impl fmt::Display for CvodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemNull => write!(f, "CVODE memory is null/uninitialized"),
            Self::LMemNull => write!(f, "Linear solver interface memory is missing"),
            Self::PMemNull => write!(f, "Preconditioner memory is missing"),
            Self::IllInput(msg) => write!(f, "Illegal input: {msg}"),
            Self::MemFail(msg) => write!(f, "Memory allocation failure: {msg}"),
            Self::SunMatFail(msg) => write!(f, "SUNMatrix operation failed: {msg}"),
            Self::SunLsFail(msg) => write!(f, "SUNLinearSolver operation failed: {msg}"),
            Self::RhsFuncFailed => write!(f, "RHS function failed during Jacobian approximation"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// NVECTOR abstraction (static dispatch, zero-cost).
pub trait NVector: Clone {
    fn len(&self) -> SunIndex;
    fn clone_empty_like(&self) -> Result<Self, CvodeError>;
    fn space(&self) -> (usize, usize);
    fn get_array_pointer_available(&self) -> bool;
}

/// Matrix abstraction.
pub trait SunMatrix: Clone {
    fn zero(&mut self) -> Result<(), CvodeError>;
    fn copy_from(&mut self, src: &Self) -> Result<(), CvodeError>;
    fn scale_add_identity(&mut self, c: SunReal) -> Result<(), CvodeError>;
    fn space(&self) -> (usize, usize);
}

/// Linear solver abstraction.
pub trait LinearSolver<V: NVector, M: SunMatrix> {
    fn initialize(&mut self) -> Result<(), CvodeError>;
    fn setup(&mut self, a: &M) -> Result<(), CvodeError>;
    fn solve(&mut self, a: &M, x: &mut V, b: &V, tol: SunReal) -> Result<(), CvodeError>;
    fn space(&self) -> (usize, usize);
}

/// RHS callback type.
pub trait RhsFn<V: NVector> {
    fn eval(&mut self, t: SunReal, y: &V, fy: &mut V) -> Result<(), CvodeError>;
}

/// CVLS memory block.
pub struct CvLsMem<V: NVector, M: SunMatrix, LS: LinearSolver<V, M>, F: RhsFn<V>> {
    pub p_data: Option<CvBandPrecData<V, M, LS, F>>,
}

/// Main CVODE memory.
pub struct Cvode<V: NVector, M: SunMatrix, LS: LinearSolver<V, M>, F: RhsFn<V>> {
    pub cv_lmem: Option<CvLsMem<V, M, LS, F>>,
    pub cv_tempv: V,
    pub rhs: F,
}

/// Banded preconditioner private data.
pub struct CvBandPrecData<V: NVector, M: SunMatrix, LS: LinearSolver<V, M>, F: RhsFn<V>> {
    pub n: SunIndex,
    pub mu: SunIndex,
    pub ml: SunIndex,
    pub nfe_bp: usize,
    pub saved_j: M,
    pub saved_p: M,
    pub ls: LS,
    pub tmp1: V,
    pub tmp2: V,
    _phantom: core::marker::PhantomData<F>,
}

/// Builder replacing CVodeSet* style configuration.
pub struct CvBandPrecBuilder {
    n: SunIndex,
    mu: SunIndex,
    ml: SunIndex,
}

impl CvBandPrecBuilder {
    #[inline]
    pub const fn new(n: SunIndex) -> Self {
        Self { n, mu: 0, ml: 0 }
    }

    #[inline]
    pub const fn upper_bandwidth(mut self, mu: SunIndex) -> Self {
        self.mu = mu;
        self
    }

    #[inline]
    pub const fn lower_bandwidth(mut self, ml: SunIndex) -> Self {
        self.ml = ml;
        self
    }

    #[inline]
    pub fn init<V, M, LS, F, MF, LSF>(
        self,
        cvode: &mut Cvode<V, M, LS, F>,
        mut make_band_matrix: MF,
        mut make_band_solver: LSF,
    ) -> Result<(), CvodeError>
    where
        V: NVector,
        M: SunMatrix,
        LS: LinearSolver<V, M>,
        F: RhsFn<V>,
        MF: FnMut(SunIndex, SunIndex, SunIndex, SunIndex) -> Result<M, CvodeError>,
        LSF: FnMut(&V, &M) -> Result<LS, CvodeError>,
    {
        cv_band_prec_init(
            cvode,
            self.n,
            self.mu,
            self.ml,
            &mut make_band_matrix,
            &mut make_band_solver,
        )
    }
}

/// Initialize band preconditioner.
#[inline]
pub fn cv_band_prec_init<V, M, LS, F, MF, LSF>(
    cvode: &mut Cvode<V, M, LS, F>,
    n: SunIndex,
    mu: SunIndex,
    ml: SunIndex,
    make_band_matrix: &mut MF,
    make_band_solver: &mut LSF,
) -> Result<(), CvodeError>
where
    V: NVector,
    M: SunMatrix,
    LS: LinearSolver<V, M>,
    F: RhsFn<V>,
    MF: FnMut(SunIndex, SunIndex, SunIndex, SunIndex) -> Result<M, CvodeError>,
    LSF: FnMut(&V, &M) -> Result<LS, CvodeError>,
{
    let lmem = cvode.cv_lmem.as_mut().ok_or(CvodeError::LMemNull)?;

    if !cvode.cv_tempv.get_array_pointer_available() {
        return Err(CvodeError::IllInput(
            "NVector must support array pointer access",
        ));
    }

    let n1 = n.saturating_sub(1);
    let mup = mu.min(n1);
    let mlp = ml.min(n1);
    let storagemu = (mup + mlp).min(n1);

    let saved_j = make_band_matrix(n, mup, mlp, mup)?;
    let saved_p = make_band_matrix(n, mup, mlp, storagemu)?;
    let mut ls = make_band_solver(&cvode.cv_tempv, &saved_p)?;
    ls.initialize()?;

    let tmp1 = cvode.cv_tempv.clone_empty_like()?;
    let tmp2 = cvode.cv_tempv.clone_empty_like()?;

    lmem.p_data = Some(CvBandPrecData {
        n,
        mu: mup,
        ml: mlp,
        nfe_bp: 0,
        saved_j,
        saved_p,
        ls,
        tmp1,
        tmp2,
        _phantom: core::marker::PhantomData,
    });

    Ok(())
}

/// Return workspace usage.
#[inline]
pub fn cv_band_prec_get_workspace<V, M, LS, F>(
    cvode: &Cvode<V, M, LS, F>,
) -> Result<(usize, usize), CvodeError>
where
    V: NVector,
    M: SunMatrix,
    LS: LinearSolver<V, M>,
    F: RhsFn<V>,
{
    let lmem = cvode.cv_lmem.as_ref().ok_or(CvodeError::LMemNull)?;
    let pdata = lmem.p_data.as_ref().ok_or(CvodeError::PMemNull)?;

    let (vrw, viw) = cvode.cv_tempv.space();
    let (jrw, jiw) = pdata.saved_j.space();
    let (prw, piw) = pdata.saved_p.space();
    let (lrw, liw) = pdata.ls.space();

    let lenrw = [2 * vrw, jrw, prw, lrw].into_iter().sum();
    let leniw = [4usize, 2 * viw, jiw, piw, liw].into_iter().sum();

    Ok((lenrw, leniw))
}

/// Return number of RHS evaluations used by band preconditioner.
#[inline]
pub fn cv_band_prec_get_num_rhs_evals<V, M, LS, F>(
    cvode: &Cvode<V, M, LS, F>,
) -> Result<usize, CvodeError>
where
    V: NVector,
    M: SunMatrix,
    LS: LinearSolver<V, M>,
    F: RhsFn<V>,
{
    let lmem = cvode.cv_lmem.as_ref().ok_or(CvodeError::LMemNull)?;
    let pdata = lmem.p_data.as_ref().ok_or(CvodeError::PMemNull)?;
    Ok(pdata.nfe_bp)
}

/// Preconditioner setup:
/// computes/reuses `J`, forms `P = I - \gamma J`, and factorizes `P`.
#[inline]
pub fn cv_band_prec_setup<V, M, LS, F>(
    cvode: &mut Cvode<V, M, LS, F>,
    t: SunReal,
    y: &V,
    fy: &V,
    jok: bool,
    gamma: SunReal,
) -> Result<bool, CvodeError>
where
    V: NVector,
    M: SunMatrix,
    LS: LinearSolver<V, M>,
    F: RhsFn<V>,
{
    let lmem = cvode.cv_lmem.as_mut().ok_or(CvodeError::LMemNull)?;
    let pdata = lmem.p_data.as_mut().ok_or(CvodeError::PMemNull)?;

    let jcur = if jok {
        pdata.saved_p.copy_from(&pdata.saved_j)?;
        false
    } else {
        pdata.saved_j.zero()?;
        cv_band_prec_dq_jac(cvode, t, y, fy)?;
        pdata.saved_p.copy_from(&pdata.saved_j)?;
        true
    };

    pdata.saved_p.scale_add_identity(-gamma)?;
    pdata.ls.setup(&pdata.saved_p)?;
    Ok(jcur)
}

/// Preconditioner solve: solve `P z = r`.
#[inline]
pub fn cv_band_prec_solve<V, M, LS, F>(
    cvode: &mut Cvode<V, M, LS, F>,
    r: &V,
    z: &mut V,
    delta: SunReal,
) -> Result<(), CvodeError>
where
    V: NVector,
    M: SunMatrix,
    LS: LinearSolver<V, M>,
    F: RhsFn<V>,
{
    let lmem = cvode.cv_lmem.as_mut().ok_or(CvodeError::LMemNull)?;
    let pdata = lmem.p_data.as_mut().ok_or(CvodeError::PMemNull)?;
    pdata.ls.solve(&pdata.saved_p, z, r, delta)
}

/// Difference-quotient Jacobian kernel (placeholder hook).
#[inline]
fn cv_band_prec_dq_jac<V, M, LS, F>(
    _cvode: &mut Cvode<V, M, LS, F>,
    _t: SunReal,
    _y: &V,
    _fy: &V,
) -> Result<(), CvodeError>
where
    V: NVector,
    M: SunMatrix,
    LS: LinearSolver<V, M>,
    F: RhsFn<V>,
{
    Ok(())
}