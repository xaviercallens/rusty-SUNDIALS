//! Band-block-diagonal (BBD) preconditioner support for a CVODE-like solver.
//!
//! This module provides an idiomatic Rust translation of core CVBBDPRE-style
//! initialization/reinitialization/query functionality with:
//! - Typed `Cvode<F, V>` memory (no `void*`)
//! - `Result<T, CvodeError>` error handling
//! - Generic `NVector`, `SunMatrix`, and `LinearSolver` traits
//! - RAII ownership of all preconditioner resources
//! - Builder-style initialization
//!
//! Numerical types:
//! - `sunrealtype -> f64`
//! - `sunindextype -> usize`

use core::fmt;

/// Floating-point scalar type (`sunrealtype`).
pub type Real = f64;

/// Index type (`sunindextype`).
pub type Index = usize;

/// Constants from the C implementation.
pub const MIN_INC_MULT: Real = 1000.0;
pub const ZERO: Real = 0.0;
pub const ONE: Real = 1.0;
pub const TWO: Real = 2.0;

/// Error codes replacing integer return flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CvodeError {
    MemNull,
    LMemNull,
    PMemNull,
    IllInput(&'static str),
    MemFail(&'static str),
    SunLsFail(&'static str),
}

impl fmt::Display for CvodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemNull => write!(f, "CVODE memory is null/uninitialized"),
            Self::LMemNull => write!(f, "Linear solver interface memory is null"),
            Self::PMemNull => write!(f, "Preconditioner memory is null"),
            Self::IllInput(msg) => write!(f, "Illegal input: {msg}"),
            Self::MemFail(msg) => write!(f, "Memory allocation failure: {msg}"),
            Self::SunLsFail(msg) => write!(f, "Linear solver failure: {msg}"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Generic vector abstraction replacing `N_Vector_Ops`.
pub trait NVector: Clone {
    fn len(&self) -> usize;
    fn clone_empty(&self, n: usize) -> Self;
    fn space(&self) -> (usize, usize);
}

/// Minimal dense serial vector implementation for demonstration/compilation.
#[derive(Clone, Debug)]
pub struct SerialNVector {
    data: Vec<Real>,
}

impl SerialNVector {
    #[inline]
    pub fn new(n: usize) -> Self {
        Self { data: vec![0.0; n] }
    }
}

impl NVector for SerialNVector {
    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn clone_empty(&self, n: usize) -> Self {
        Self::new(n)
    }

    #[inline]
    fn space(&self) -> (usize, usize) {
        (self.data.len(), 1)
    }
}

/// Matrix abstraction replacing `SUNMatrix`.
pub trait SunMatrix {
    fn space(&self) -> (usize, usize);
}

/// Banded matrix storage.
#[derive(Clone, Debug)]
pub struct BandMatrix {
    n: usize,
    mu: usize,
    ml: usize,
    smu: usize,
    data: Vec<Real>,
}

impl BandMatrix {
    #[inline]
    pub fn new(n: usize, mu: usize, ml: usize, smu: usize) -> Result<Self, CvodeError> {
        let rows = smu + ml + 1;
        let len = rows
            .checked_mul(n)
            .ok_or(CvodeError::MemFail("band matrix size overflow"))?;
        Ok(Self {
            n,
            mu,
            ml,
            smu,
            data: vec![0.0; len],
        })
    }

    #[inline]
    pub const fn n(&self) -> usize {
        self.n
    }

    #[inline]
    pub const fn mu(&self) -> usize {
        self.mu
    }

    #[inline]
    pub const fn ml(&self) -> usize {
        self.ml
    }

    #[inline]
    pub const fn smu(&self) -> usize {
        self.smu
    }
}

impl SunMatrix for BandMatrix {
    #[inline]
    fn space(&self) -> (usize, usize) {
        (self.data.len(), 4)
    }
}

/// Linear solver abstraction replacing `SUNLinearSolver`.
pub trait LinearSolver<V: NVector, M: SunMatrix> {
    fn initialize(&mut self) -> Result<(), CvodeError>;
    fn space(&self) -> (usize, usize);
}

/// Simple band linear solver placeholder.
#[derive(Clone, Debug)]
pub struct BandLinearSolver {
    rw: usize,
    iw: usize,
}

impl BandLinearSolver {
    #[inline]
    pub fn new<V: NVector>(_v: &V, _m: &BandMatrix) -> Result<Self, CvodeError> {
        Ok(Self { rw: 16, iw: 8 })
    }
}

impl<V: NVector> LinearSolver<V, BandMatrix> for BandLinearSolver {
    #[inline]
    fn initialize(&mut self) -> Result<(), CvodeError> {
        Ok(())
    }

    #[inline]
    fn space(&self) -> (usize, usize) {
        (self.rw, self.iw)
    }
}

/// Local residual callback (`gloc`).
pub type CvLocalFn<V> = fn(t: Real, y: &V, g: &mut V) -> Result<(), CvodeError>;

/// Communication callback (`cfn`).
pub type CvCommFn<V> = fn(t: Real, y: &V) -> Result<(), CvodeError>;

/// Preconditioner data (RAII-owned).
#[derive(Clone)]
pub struct CvbBdPrecData<V: NVector> {
    pub gloc: CvLocalFn<V>,
    pub cfn: CvCommFn<V>,
    pub mudq: usize,
    pub mldq: usize,
    pub mukeep: usize,
    pub mlkeep: usize,
    pub dqrely: Real,
    pub n_local: usize,
    pub saved_j: BandMatrix,
    pub saved_p: BandMatrix,
    pub zlocal: V,
    pub rlocal: V,
    pub tmp1: V,
    pub tmp2: V,
    pub tmp3: V,
    pub ls: BandLinearSolver,
    pub rpwsize: usize,
    pub ipwsize: usize,
    pub nge: usize,
}

/// CVLS memory block.
#[derive(Clone)]
pub struct CvLsMem<V: NVector> {
    pub p_data: Option<CvbBdPrecData<V>>,
}

impl<V: NVector> CvLsMem<V> {
    #[inline]
    pub const fn new() -> Self {
        Self { p_data: None }
    }
}

/// Typed CVODE memory.
pub struct Cvode<F, V: NVector> {
    pub rhs: F,
    pub lmem: Option<CvLsMem<V>>,
    pub tempv: V,
    pub uround: Real,
}

impl<F, V: NVector> Cvode<F, V> {
    #[inline]
    pub fn new(rhs: F, tempv: V, uround: Real) -> Self {
        Self {
            rhs,
            lmem: Some(CvLsMem::new()),
            tempv,
            uround,
        }
    }
}

/// Builder for BBD preconditioner initialization.
pub struct CvbBdPrecInitBuilder<V: NVector> {
    n_local: usize,
    mudq: usize,
    mldq: usize,
    mukeep: usize,
    mlkeep: usize,
    dqrely: Real,
    gloc: CvLocalFn<V>,
    cfn: CvCommFn<V>,
}

impl<V: NVector> CvbBdPrecInitBuilder<V> {
    #[inline]
    pub fn new(n_local: usize, gloc: CvLocalFn<V>, cfn: CvCommFn<V>) -> Self {
        Self {
            n_local,
            mudq: 0,
            mldq: 0,
            mukeep: 0,
            mlkeep: 0,
            dqrely: 0.0,
            gloc,
            cfn,
        }
    }

    #[inline]
    pub const fn mudq(mut self, v: usize) -> Self {
        self.mudq = v;
        self
    }

    #[inline]
    pub const fn mldq(mut self, v: usize) -> Self {
        self.mldq = v;
        self
    }

    #[inline]
    pub const fn mukeep(mut self, v: usize) -> Self {
        self.mukeep = v;
        self
    }

    #[inline]
    pub const fn mlkeep(mut self, v: usize) -> Self {
        self.mlkeep = v;
        self
    }

    #[inline]
    pub const fn dqrely(mut self, v: Real) -> Self {
        self.dqrely = v;
        self
    }

    #[inline]
    pub fn init<F>(self, cvode: &mut Cvode<F, V>) -> Result<(), CvodeError> {
        cvbbd_prec_init(
            cvode,
            self.n_local,
            self.mudq,
            self.mldq,
            self.mukeep,
            self.mlkeep,
            self.dqrely,
            self.gloc,
            self.cfn,
        )
    }
}

#[inline]
pub const fn clamp_bw(n_local: usize, bw: usize) -> usize {
    if n_local == 0 {
        0
    } else if bw < n_local {
        bw
    } else {
        n_local - 1
    }
}

/// Initialize BBD preconditioner data.
///
/// Constructs banded Jacobian/preconditioner storage and temporary vectors,
/// then attaches the preconditioner to CVLS memory.
///
/// The preconditioner approximates a local Jacobian block \(J\) and forms
/// \(P = I - \gamma J\) for Krylov preconditioning.
pub fn cvbbd_prec_init<F, V: NVector>(
    cvode: &mut Cvode<F, V>,
    n_local: usize,
    mudq: usize,
    mldq: usize,
    mukeep: usize,
    mlkeep: usize,
    dqrely: Real,
    gloc: CvLocalFn<V>,
    cfn: CvCommFn<V>,
) -> Result<(), CvodeError> {
    let lmem = cvode.lmem.as_mut().ok_or(CvodeError::LMemNull)?;

    let mudq = clamp_bw(n_local, mudq);
    let mldq = clamp_bw(n_local, mldq);
    let muk = clamp_bw(n_local, mukeep);
    let mlk = clamp_bw(n_local, mlkeep);

    let saved_j = BandMatrix::new(n_local, muk, mlk, muk)?;
    let storage_mu = if n_local == 0 {
        0
    } else {
        (muk + mlk).min(n_local - 1)
    };
    let saved_p = BandMatrix::new(n_local, muk, mlk, storage_mu)?;

    let zlocal = cvode.tempv.clone_empty(n_local);
    let rlocal = cvode.tempv.clone_empty(n_local);
    let tmp1 = cvode.tempv.clone_empty(cvode.tempv.len());
    let tmp2 = cvode.tempv.clone_empty(cvode.tempv.len());
    let tmp3 = cvode.tempv.clone_empty(cvode.tempv.len());

    let mut ls = BandLinearSolver::new(&rlocal, &saved_p)?;
    ls.initialize()?;

    let dqrely = if dqrely > ZERO {
        dqrely
    } else {
        cvode.uround.sqrt()
    };

    let mut pdata = CvbBdPrecData {
        gloc,
        cfn,
        mudq,
        mldq,
        mukeep: muk,
        mlkeep: mlk,
        dqrely,
        n_local,
        saved_j,
        saved_p,
        zlocal,
        rlocal,
        tmp1,
        tmp2,
        tmp3,
        ls,
        rpwsize: 0,
        ipwsize: 0,
        nge: 0,
    };

    let spaces = [
        cvode.tempv.space(),
        pdata.rlocal.space(),
        pdata.saved_j.space(),
        pdata.saved_p.space(),
        pdata.ls.space(),
    ];
    let mults = [3usize, 2, 1, 1, 1];

    let (rpw, ipw) = spaces
        .into_iter()
        .zip(mults)
        .fold((0usize, 0usize), |(ar, ai), ((r, i), m)| {
            (ar + m * r, ai + m * i)
        });

    pdata.rpwsize = rpw;
    pdata.ipwsize = ipw;

    lmem.p_data = Some(pdata);
    Ok(())
}

/// Reinitialize BBD preconditioner parameters without reallocating storage.
#[inline]
pub fn cvbbd_prec_reinit<F, V: NVector>(
    cvode: &mut Cvode<F, V>,
    mudq: usize,
    mldq: usize,
    dqrely: Real,
) -> Result<(), CvodeError> {
    let pdata = cvode
        .lmem
        .as_mut()
        .ok_or(CvodeError::LMemNull)?
        .p_data
        .as_mut()
        .ok_or(CvodeError::PMemNull)?;

    let n_local = pdata.n_local;
    pdata.mudq = clamp_bw(n_local, mudq);
    pdata.mldq = clamp_bw(n_local, mldq);
    pdata.dqrely = if dqrely > ZERO {
        dqrely
    } else {
        cvode.uround.sqrt()
    };
    pdata.nge = 0;
    Ok(())
}

/// Return BBD preconditioner workspace sizes `(real_words, int_words)`.
#[inline]
pub fn cvbbd_prec_get_workspace<F, V: NVector>(
    cvode: &Cvode<F, V>,
) -> Result<(usize, usize), CvodeError> {
    let pdata = cvode
        .lmem
        .as_ref()
        .ok_or(CvodeError::LMemNull)?
        .p_data
        .as_ref()
        .ok_or(CvodeError::PMemNull)?;
    Ok((pdata.rpwsize, pdata.ipwsize))
}

/// Return number of local residual evaluations used by the preconditioner.
#[inline]
pub fn cvbbd_prec_get_num_gfn_evals<F, V: NVector>(cvode: &Cvode<F, V>) -> Result<usize, CvodeError> {
    let pdata = cvode
        .lmem
        .as_ref()
        .ok_or(CvodeError::LMemNull)?
        .p_data
        .as_ref()
        .ok_or(CvodeError::PMemNull)?;
    Ok(pdata.nge)
}