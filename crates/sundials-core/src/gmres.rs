//! GMRES (Generalised Minimal RESidual) iterative Krylov solver.
//!
//! Solves the linear system  M·x = b  without forming M explicitly.
//! Only matrix-vector products  v = M·w  are required, making this ideal for:
//!   * Very large state spaces (N > 100 000) where LU is prohibitive.
//!   * GPU-resident state vectors (the MVP can run in a compute shader).
//!   * Jacobian-Free Newton-Krylov (JFNK) where M is a finite-difference
//!     directional derivative.
//!
//! Algorithm: GMRES with Modified Gram-Schmidt (Arnoldi), restarted every
//! `restart` iterations (GMRES(m)).
//!
//! Reference: Saad & Schultz (1986), "GMRES: A generalized minimal residual
//!   algorithm for solving nonsymmetric linear systems", SIAM J. Sci. Stat.
//!   Comput. 7(3), 856–869.

use crate::Real;

/// Outcome of a GMRES solve.
#[derive(Debug, PartialEq)]
pub enum GmresStatus {
    Converged { iters: usize, res_norm: Real },
    MaxItersReached { res_norm: Real },
}

/// Configuration for the GMRES solver.
#[derive(Clone, Debug)]
pub struct GmresConfig {
    /// Maximum outer restarts.
    pub max_restarts: usize,
    /// Krylov subspace dimension (inner iterations before restart).
    pub restart: usize,
    /// Convergence tolerance on the residual norm.
    pub tol: Real,
}

impl Default for GmresConfig {
    fn default() -> Self {
        Self { max_restarts: 20, restart: 30, tol: 1e-8 }
    }
}

/// GMRES(m) solver.
///
/// # Arguments
/// * `matvec` — closure computing `y = A·x` (can be a finite-diff Jacobian-vector product)
/// * `b`      — right-hand side (length N)
/// * `x`      — initial guess, modified in-place to the solution
/// * `cfg`    — solver configuration
/// GMRES(m) solver without preconditioning.
///
/// Convenience wrapper over `gmres_preconditioned` using identity preconditioners.
pub fn gmres<F>(
    matvec: F,
    b: &[Real],
    x: &mut Vec<Real>,
    cfg: &GmresConfig,
) -> GmresStatus
where
    F: Fn(&[Real], &mut [Real]),
{
    let identity = |v: &[Real], out: &mut [Real]| {
        out.copy_from_slice(v);
    };
    gmres_preconditioned(matvec, b, x, cfg, identity, identity)
}

/// Preconditioned GMRES(m) solver.
///
/// Solves $(M_L^{-1} A M_R^{-1}) y = M_L^{-1} b$ where $x = M_R^{-1} y$.
///
/// # Arguments
/// * `matvec`     — closure computing `y = A·x`
/// * `b`          — right-hand side (length N)
/// * `x`          — initial guess, modified in-place to the solution
/// * `cfg`        — solver configuration
/// * `left_prec`  — left preconditioner action $y = M_L^{-1} x$
/// * `right_prec` — right preconditioner action $y = M_R^{-1} x$
pub fn gmres_preconditioned<F, PL, PR>(
    matvec: F,
    b: &[Real],
    x: &mut Vec<Real>,
    cfg: &GmresConfig,
    mut left_prec: PL,
    mut right_prec: PR,
) -> GmresStatus
where
    F: Fn(&[Real], &mut [Real]),
    PL: FnMut(&[Real], &mut [Real]),
    PR: FnMut(&[Real], &mut [Real]),
{
    let n = b.len();
    let m = cfg.restart;

    let mut total_iters = 0usize;

    for _restart in 0..=cfg.max_restarts {
        // Compute initial residual: r = b - A·x
        let mut ax = vec![0.0; n];
        matvec(x, &mut ax);
        let r_unprec: Vec<Real> = b.iter().zip(ax.iter()).map(|(bi, axi)| bi - axi).collect();

        // Apply left preconditioner: r_0 = M_L^{-1} (b - A·x)
        let mut r = vec![0.0; n];
        left_prec(&r_unprec, &mut r);

        let beta = norm2(&r);
        if beta < cfg.tol {
            return GmresStatus::Converged { iters: total_iters, res_norm: beta };
        }

        // Arnoldi basis V[0..m+1][n], Hessenberg H[m+1][m]
        let mut v: Vec<Vec<Real>> = Vec::with_capacity(m + 1);
        let mut h: Vec<Vec<Real>> = vec![vec![0.0; m]; m + 1];

        // v[0] = r / ||r||
        let r0: Vec<Real> = r.iter().map(|ri| ri / beta).collect();
        v.push(r0);

        let mut cs = vec![0.0; m];
        let mut sn = vec![0.0; m];
        let mut e1 = vec![0.0; m + 1];
        e1[0] = beta;

        let mut j_end = 0;

        for j in 0..m {
            j_end = j;
            total_iters += 1;

            // Apply right preconditioner: zj = M_R^{-1} v[j]
            let mut zj = vec![0.0; n];
            right_prec(&v[j], &mut zj);

            // Matrix-vector product: w_unprec = A · zj
            let mut w_unprec = vec![0.0; n];
            matvec(&zj, &mut w_unprec);

            // Apply left preconditioner: w = M_L^{-1} w_unprec
            let mut w = vec![0.0; n];
            left_prec(&w_unprec, &mut w);

            // Modified Gram-Schmidt orthogonalisation
            for i in 0..=j {
                h[i][j] = dot(&w, &v[i]);
                for k in 0..n {
                    w[k] -= h[i][j] * v[i][k];
                }
            }
            h[j + 1][j] = norm2(&w);

            if h[j + 1][j].abs() < 1e-15 {
                // lucky breakdown
                v.push(vec![0.0; n]);
                break;
            }
            let wn = w.iter().map(|wi| wi / h[j + 1][j]).collect();
            v.push(wn);

            // Apply previous Givens rotations
            for i in 0..j {
                let tmp =  cs[i] * h[i][j] + sn[i] * h[i + 1][j];
                h[i + 1][j] = -sn[i] * h[i][j] + cs[i] * h[i + 1][j];
                h[i][j] = tmp;
            }
            // Compute new Givens rotation
            let (c, s) = givens_rotation(h[j][j], h[j + 1][j]);
            cs[j] = c; sn[j] = s;

            h[j][j]     =  cs[j] * h[j][j] + sn[j] * h[j + 1][j];
            h[j + 1][j] = 0.0;

            e1[j + 1] = -sn[j] * e1[j];
            e1[j]     =  cs[j] * e1[j];

            let res: Real = e1[j + 1].abs();
            if res < cfg.tol {
                let y = back_solve(&h, &e1, j + 1);
                
                // Form solution: x_new = x_old + M_R^{-1} V y
                let mut vy = vec![0.0; n];
                for step_j in 0..=j {
                    for i in 0..n {
                        vy[i] += y[step_j] * v[step_j][i];
                    }
                }
                let mut prec_vy = vec![0.0; n];
                right_prec(&vy, &mut prec_vy);
                for i in 0..n {
                    x[i] += prec_vy[i];
                }

                return GmresStatus::Converged { iters: total_iters, res_norm: res };
            }
        }

        // Solve and update x with accumulated subspace
        let y = back_solve(&h, &e1, j_end + 1);
        let mut vy = vec![0.0; n];
        for step_j in 0..=j_end {
            for i in 0..n {
                vy[i] += y[step_j] * v[step_j][i];
            }
        }
        let mut prec_vy = vec![0.0; n];
        right_prec(&vy, &mut prec_vy);
        for i in 0..n {
            x[i] += prec_vy[i];
        }

        // Check un-preconditioned residual for restart
        let mut ax2 = vec![0.0; n];
        matvec(x, &mut ax2);
        let r_unprec: Vec<Real> = b.iter().zip(ax2.iter()).map(|(bi, axi)| bi - axi).collect();
        let res_norm = norm2(&r_unprec);
        if res_norm < cfg.tol {
            return GmresStatus::Converged { iters: total_iters, res_norm };
        }
    }

    let mut ax = vec![0.0; n];
    matvec(x, &mut ax);
    let res_norm = norm2(&b.iter().zip(ax.iter()).map(|(bi, axi)| bi - axi).collect::<Vec<_>>());
    GmresStatus::MaxItersReached { res_norm }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn norm2(v: &[Real]) -> Real {
    v.iter().map(|x| x * x).sum::<Real>().sqrt()
}

fn dot(a: &[Real], b: &[Real]) -> Real {
    a.iter().zip(b.iter()).map(|(ai, bi)| ai * bi).sum()
}

/// Compute Givens rotation  (c, s)  such that  |[c s; -s c] * [a; b]| = [r; 0].
#[cfg(test)]
pub(crate) fn givens_rotation(a: Real, b: Real) -> (Real, Real) {
    if b == 0.0 { (1.0, 0.0) } else { let r = a.hypot(b); (a / r, b / r) }
}
#[cfg(not(test))]
fn givens_rotation(a: Real, b: Real) -> (Real, Real) {
    if b == 0.0 { (1.0, 0.0) } else { let r = a.hypot(b); (a / r, b / r) }
}

/// Backward substitution for upper-triangular H[0..k][0..k] · y = e1[0..k].
fn back_solve(h: &[Vec<Real>], e1: &[Real], k: usize) -> Vec<Real> {
    let mut y = e1[..k].to_vec();
    for i in (0..k).rev() {
        y[i] /= h[i][i];
        for l in 0..i {
            y[l] -= h[l][i] * y[i];
        }
    }
    y
}

/// Update x += V[0..k] · y[0..k].
fn update_x(x: &mut [Real], v: &[Vec<Real>], y: &[Real], k: usize) {
    for j in 0..k {
        for i in 0..x.len() {
            x[i] += y[j] * v[j][i];
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// Solve a simple 3×3 SPD system with known solution.
    #[test]
    fn test_gmres_simple() {
        // A = [[4,1,0],[1,3,1],[0,1,2]], b = [1,2,3]
        let a = [[4.0, 1.0, 0.0], [1.0, 3.0, 1.0], [0.0, 1.0, 2.0]];
        let b = vec![1.0, 2.0, 3.0];

        let matvec = |x: &[Real], y: &mut [Real]| {
            for i in 0..3 {
                y[i] = (0..3).map(|j| a[i][j] * x[j]).sum();
            }
        };

        let mut x = vec![0.0; 3];
        let status = gmres(matvec, &b, &mut x, &GmresConfig::default());

        // Verify A·x ≈ b
        let mut ax = vec![0.0; 3];
        matvec(&x, &mut ax);
        for i in 0..3 {
            assert!((ax[i] - b[i]).abs() < 1e-6, "residual[{i}] = {}", (ax[i] - b[i]).abs());
        }

        let converged = matches!(status, GmresStatus::Converged { .. });
        assert!(converged, "GMRES did not converge on simple 3x3 SPD system");
    }
}
