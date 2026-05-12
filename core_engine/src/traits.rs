//! Formally Verified AI-Discovery Interfaces
//! 
//! These traits form the unbreakable boundary between the DeepProbLog/CodeBERT
//! synthesized AI hypotheses and the Exascale C-ABI of SUNDIALS.
//! The Aeneas toolchain explicitly monitors these trait boundaries to enforce
//! the LLBC to Lean 4 formal extraction.

use nalgebra::DMatrix;
use sundials_sys::realtype;

/// A Memory-Safe abstraction for a SUNDIALS Preconditioner.
/// Any AI-generated preconditioner (like FoGNO, FLAGNO, AMG) must implement
/// this trait. CodeBERT will synthesize structs conforming to this.
pub trait SUNPreconditioner: Send + Sync {
    /// Dimension of the state space vector
    fn dim(&self) -> usize;

    /// Setup phase (evaluates matrix factors, spectral radii, graph networks)
    /// Expected to be bounded O(N) or O(N log N) by Lean 4 specifications.
    fn setup(&mut self, j_matrix: &DMatrix<realtype>) -> Result<(), String>;

    /// Solve phase (P z = r). Applies the inverse preconditioner.
    fn solve(&self, r: &[realtype], z: &mut [realtype], tol: realtype) -> Result<(), String>;
}

/// A Memory-Safe abstraction for an Iterative Linear Solver.
/// E.g., Dynamic Spectral IMEX, Krylov methods.
pub trait SUNLinearSolver: Send + Sync {
    /// Dimension of the linear system
    fn dim(&self) -> usize;

    /// Setup the linear solver parameters based on the Jacobian
    fn setup(&mut self, j_matrix: &DMatrix<realtype>) -> Result<(), String>;

    /// Solve the linear system A x = b to a specified tolerance
    fn solve(&self, a: &DMatrix<realtype>, x: &mut [realtype], b: &[realtype], tol: realtype) -> Result<(), String>;
}
