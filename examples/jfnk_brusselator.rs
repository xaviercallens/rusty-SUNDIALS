//! Jacobian-Free Newton-Krylov (JFNK) demonstration using Dual Numbers.
//!
//! This example validates the performance improvements from `v0.2.0` by solving
//! the nonlinear reaction term of the Brusselator PDE using GMRES and exact
//! Automatic Differentiation (AutoDiff) for the Jacobian-vector products.
//!
//! The Brusselator equations are:
//! u' = A + u^2 * v - (B + 1) * u + D_u * u''
//! v' = B * u - u^2 * v + D_v * v''
//!
//! We will solve the nonlinear algebraic system F(u, v) = 0 for the steady state
//! (without diffusion for simplicity) using Newton's method. The linear solve
//! J * dx = -F inside Newton is performed by GMRES.
//! Instead of a finite-difference approximation for J * v, we use `Dual` numbers.

use sundials_core::dual::Dual;
use sundials_core::gmres::{gmres, GmresConfig, GmresStatus};
use std::time::Instant;

/// The nonlinear reaction function F(y), generic over T: Real or Dual
fn brusselator_reaction<T>(u: T, v: T, a: f64, b: f64) -> (T, T)
where
    T: Copy
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Add<f64, Output = T>
        + std::ops::Sub<f64, Output = T>
        + std::convert::From<f64>,
    f64: std::ops::Mul<T, Output = T>,
{
    // F_u = A + u^2 * v - (B + 1) * u
    let fu = T::from(a) + u * u * v - (b + 1.0) * u;
    // F_v = B * u - u^2 * v
    let fv = b * u - u * u * v;

    (fu, fv)
}

fn main() {
    println!("🚀 Validating JFNK AutoDiff with the Brusselator System");

    // Parameters
    let a = 1.0;
    let b = 3.0;

    // Initial guess
    let mut y = vec![0.5, 0.5]; // u, v
    
    // We want to find the roots of F(y) = 0 using Newton's method
    let mut newton_iters = 0;
    let max_newton_iters = 10;
    
    let start = Instant::now();

    while newton_iters < max_newton_iters {
        // 1. Evaluate RHS in standard f64 to get F(y)
        let (fu, fv) = brusselator_reaction(y[0], y[1], a, b);
        let f_val = vec![fu, fv];
        
        let norm_f = (fu*fu + fv*fv).sqrt();
        println!("Newton Iteration {}: ||F(y)|| = {:.2e}", newton_iters, norm_f);
        
        if norm_f < 1e-12 {
            println!("✅ Newton's method converged in {} iterations.", newton_iters);
            break;
        }

        // We need to solve J * dx = -F(y) using GMRES
        let rhs = vec![-fu, -fv];
        let mut dx = vec![0.0, 0.0];

        // 2. Define the exact Jacobian-vector product using Dual numbers!
        // matvec closure for GMRES
        let jv_exact = |v: &[f64], out: &mut [f64]| {
            // Seed the dual numbers: y_dual = y + v * ε
            let u_dual = Dual::new(y[0], v[0]);
            let v_dual = Dual::new(y[1], v[1]);
            
            // Evaluate RHS generically over Dual
            let (fu_dual, fv_dual) = brusselator_reaction(u_dual, v_dual, a, b);
            
            // The dual part is exactly J * v
            out[0] = fu_dual.dual;
            out[1] = fv_dual.dual;
        };

        // 3. Solve the linear system using GMRES
        let cfg = GmresConfig { tol: 1e-10, ..Default::default() };
        let status = gmres(jv_exact, &rhs, &mut dx, &cfg);
        
        match status {
            GmresStatus::Converged { iters, .. } => {
                println!("   GMRES converged in {} inner iterations", iters);
            }
            GmresStatus::MaxItersReached { .. } => {
                println!("   ⚠️ GMRES reached max iterations");
            }
        }

        // 4. Apply Newton step
        y[0] += dx[0];
        y[1] += dx[1];

        newton_iters += 1;
    }

    let duration = start.elapsed();
    println!("⏱️  Total Time: {:?}", duration);
    println!("🎯 Final Steady State: u = {:.5}, v = {:.5}", y[0], y[1]);

    // Theoretical steady state is u = A, v = B/A
    let expected_u = a;
    let expected_v = b / a;
    
    let err_u = (y[0] - expected_u).abs();
    let err_v = (y[1] - expected_v).abs();
    
    assert!(err_u < 1e-10, "u error too large: {}", err_u);
    assert!(err_v < 1e-10, "v error too large: {}", err_v);
    
    println!("✅ Validation Successful: Steady state matches theoretical values exactly.");
}
