//! Iterative linear-algebra kernels used by Krylov/Anderson-style solvers.
//!
//! This module is an idiomatic Rust translation of the SUNDIALS iterative
//! helper routines (Gram-Schmidt, QR factorization/solve, and QR-add updates).
//!
//! The implementation preserves IEEE-754 numerical behavior of the original C
//! algorithms while exposing safe, trait-based abstractions.

#![allow(clippy::needless_range_loop)]

use core::fmt;

/// Floating-point scalar type used by SUNDIALS.
pub type SunReal = f64;

/// Index type used by SUNDIALS.
pub type SunIndex = usize;

const FACTOR: SunReal = 1000.0;
const ZERO: SunReal = 0.0;
const ONE: SunReal = 1.0;

#[inline]
const fn two(i: usize) -> usize {
    i * 2
}

/// Error type for iterative kernels.
#[derive(Debug, Clone, PartialEq)]
pub enum CvodeError {
    /// Input dimensions are inconsistent.
    DimensionMismatch(&'static str),
    /// Matrix/vector index out of bounds.
    IndexOutOfBounds(&'static str),
    /// A required diagonal entry was zero (singular triangular system).
    Singular { at: usize },
    /// Generic linear algebra backend failure.
    LinearAlgebra(&'static str),
}

impl fmt::Display for CvodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch(s) => write!(f, "dimension mismatch: {s}"),
            Self::IndexOutOfBounds(s) => write!(f, "index out of bounds: {s}"),
            Self::Singular { at } => write!(f, "singular system at 1-based index {}", at + 1),
            Self::LinearAlgebra(s) => write!(f, "linear algebra error: {s}"),
        }
    }
}

impl std::error::Error for CvodeError {}

/// Trait abstraction for SUNDIALS-like vectors.
///
/// Implementations should be zero-cost and avoid dynamic dispatch in hot paths.
pub trait NVector: Clone {
    fn dot(&self, other: &Self) -> Result<SunReal, CvodeError>;
    fn linear_sum(
        a: SunReal,
        x: &Self,
        b: SunReal,
        y: &Self,
        z: &mut Self,
    ) -> Result<(), CvodeError>;
    fn scale(c: SunReal, x: &Self, z: &mut Self) -> Result<(), CvodeError>;

    #[inline]
    fn dot_prod_multi(x: &Self, ys: &[Self], out: &mut [SunReal]) -> Result<(), CvodeError> {
        if ys.len() != out.len() {
            return Err(CvodeError::DimensionMismatch("dot_prod_multi output length"));
        }
        for (o, y) in out.iter_mut().zip(ys.iter()) {
            *o = x.dot(y)?;
        }
        Ok(())
    }

    #[inline]
    fn linear_combination(
        coeffs: &[SunReal],
        xs: &[Self],
        z: &mut Self,
    ) -> Result<(), CvodeError> {
        if coeffs.len() != xs.len() || xs.is_empty() {
            return Err(CvodeError::DimensionMismatch("linear_combination"));
        }
        let mut tmp = xs[0].clone();
        Self::scale(coeffs[0], &xs[0], &mut tmp)?;
        for (c, x) in coeffs.iter().copied().zip(xs.iter()).skip(1) {
            let mut next = tmp.clone();
            Self::linear_sum(ONE, &tmp, c, x, &mut next)?;
            tmp = next;
        }
        *z = tmp;
        Ok(())
    }
}

/// Workspace for QR-add routines.
#[derive(Clone)]
pub struct SunQrData<V: NVector> {
    pub vtemp: V,
    pub vtemp2: V,
    pub temp_array: Vec<SunReal>,
}

/// Modified Gram-Schmidt orthogonalization.
///
/// Computes orthogonalization of `v[k]` against `v[i0..k)` where `i0=max(k-p,0)`,
/// updates Hessenberg entries `h[i][k-1]`, and returns `||v_k^{new}||_2`.
#[inline]
pub fn sun_modified_gs<V: NVector>(
    v: &mut [V],
    h: &mut [Vec<SunReal>],
    k: usize,
    p: usize,
) -> Result<SunReal, CvodeError> {
    if k == 0 || k >= v.len() {
        return Err(CvodeError::IndexOutOfBounds("k in sun_modified_gs"));
    }

    let k_minus_1 = k - 1;
    let i0 = k.saturating_sub(p);

    let vk_norm = v[k].dot(&v[k])?.sqrt();

    for i in i0..k {
        h[i][k_minus_1] = v[i].dot(&v[k])?;
        let mut out = v[k].clone();
        V::linear_sum(ONE, &v[k], -h[i][k_minus_1], &v[i], &mut out)?;
        v[k] = out;
    }

    let mut new_vk_norm = v[k].dot(&v[k])?.sqrt();
    let temp = FACTOR * vk_norm;
    if (temp + new_vk_norm) != temp {
        return Ok(new_vk_norm);
    }

    let mut new_norm_2 = ZERO;
    for i in i0..k {
        let new_product = v[i].dot(&v[k])?;
        let temp2 = FACTOR * h[i][k_minus_1];
        if (temp2 + new_product) == temp2 {
            continue;
        }
        h[i][k_minus_1] += new_product;
        let mut out = v[k].clone();
        V::linear_sum(ONE, &v[k], -new_product, &v[i], &mut out)?;
        v[k] = out;
        new_norm_2 += new_product * new_product;
    }

    if new_norm_2 != ZERO {
        let np = new_vk_norm * new_vk_norm - new_norm_2;
        new_vk_norm = if np > ZERO { np.sqrt() } else { ZERO };
    }

    Ok(new_vk_norm)
}

/// Classical Gram-Schmidt orthogonalization with optional reorthogonalization.
#[inline]
pub fn sun_classical_gs<V: NVector>(
    v: &mut [V],
    h: &mut [Vec<SunReal>],
    k: usize,
    p: usize,
    stemp: &mut [SunReal],
    vtemp: &mut [V],
) -> Result<SunReal, CvodeError> {
    if k == 0 || k >= v.len() {
        return Err(CvodeError::IndexOutOfBounds("k in sun_classical_gs"));
    }

    let k_minus_1 = k - 1;
    let i0 = k.saturating_sub(p);
    let n = k - i0 + 1;

    if stemp.len() < n || vtemp.len() < n {
        return Err(CvodeError::DimensionMismatch("stemp/vtemp too small"));
    }

    V::dot_prod_multi(&v[k], &v[i0..=k], &mut stemp[..n])?;
    let vk_norm = stemp[n - 1].sqrt();

    for i in (0..(n - 1)).rev() {
        h[i][k_minus_1] = stemp[i];
        stemp[i + 1] = -stemp[i];
        vtemp[i + 1] = v[i].clone();
    }

    stemp[0] = ONE;
    vtemp[0] = v[k].clone();
    V::linear_combination(&stemp[..n], &vtemp[..n], &mut v[k])?;

    let mut new_vk_norm = v[k].dot(&v[k])?.sqrt();

    if (FACTOR * new_vk_norm) < vk_norm {
        V::dot_prod_multi(&v[k], &v[i0..k], &mut stemp[1..(k - i0 + 1)])?;
        stemp[0] = ONE;
        vtemp[0] = v[k].clone();

        for i in i0..k {
            let idx = i - i0 + 1;
            h[i][k_minus_1] += stemp[idx];
            stemp[idx] = -stemp[idx];
            vtemp[idx] = v[i - i0].clone();
        }

        V::linear_combination(&stemp[..(k + 1)], &vtemp[..(k + 1)], &mut v[k])?;
        new_vk_norm = v[k].dot(&v[k])?.sqrt();
    }

    Ok(new_vk_norm)
}

/// QR factorization/update using Givens rotations.
///
/// Returns `Ok(0)` on success, or `Ok(code)` with 1-based singular pivot index.
#[inline]
pub fn sun_qr_fact(
    n: usize,
    h: &mut [Vec<SunReal>],
    q: &mut [SunReal],
    job: i32,
) -> Result<usize, CvodeError> {
    let mut code = 0usize;

    match job {
        0 => {
            for k in 0..n {
                for j in 0..k.saturating_sub(1) {
                    let qi = two(j);
                    let (temp1, temp2) = (h[j][k], h[j + 1][k]);
                    let (c, s) = (q[qi], q[qi + 1]);
                    h[j][k] = c * temp1 - s * temp2;
                    h[j + 1][k] = s * temp1 + c * temp2;
                }

                let (temp1, temp2) = (h[k][k], h[k + 1][k]);
                let (c, s) = givens(temp1, temp2);
                let qk = two(k);
                q[qk] = c;
                q[qk + 1] = s;
                h[k][k] = c * temp1 - s * temp2;

                if h[k][k] == ZERO {
                    code = k + 1;
                }
            }
        }
        _ => {
            let nm1 = n - 1;
            for k in 0..nm1 {
                let qk = two(k);
                let (temp1, temp2) = (h[k][nm1], h[k + 1][nm1]);
                let (c, s) = (q[qk], q[qk + 1]);
                h[k][nm1] = c * temp1 - s * temp2;
                h[k + 1][nm1] = s * temp1 + c * temp2;
            }

            let (temp1, temp2) = (h[nm1][nm1], h[n][nm1]);
            let (c, s) = givens(temp1, temp2);
            let qn = two(nm1);
            q[qn] = c;
            q[qn + 1] = s;
            h[nm1][nm1] = c * temp1 - s * temp2;

            if h[nm1][nm1] == ZERO {
                code = n;
            }
        }
    }

    Ok(code)
}

#[inline]
const fn sign_neg_one() -> SunReal {
    -ONE
}

#[inline]
fn givens(temp1: SunReal, temp2: SunReal) -> (SunReal, SunReal) {
    if temp2 == ZERO {
        (ONE, ZERO)
    } else if temp2.abs() >= temp1.abs() {
        let t = temp1 / temp2;
        let s = sign_neg_one() / (ONE + t * t).sqrt();
        let c = -s * t;
        (c, s)
    } else {
        let t = temp2 / temp1;
        let c = ONE / (ONE + t * t).sqrt();
        let s = -c * t;
        (c, s)
    }
}