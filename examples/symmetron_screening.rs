//! 1D Symmetron Screening Equation solved with BDF + Banded Jacobian.
//!
//! Solves:
//! u_t = u_rr + (2/r) u_r - ( (rho(r)/M^2 - mu^2) * u + lambda * u^3 )
//!
//! Where rho(r) = rho_in * 0.5 * (1 - tanh((r - r_core) / eps)) + rho_out

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::fs::File;
use std::io::Write;

const N: usize = 1000;
const R_MAX: f64 = 10.0;

// Symmetron parameters (from workshopcosmo.py)
const RHO_IN: f64 = 1000.0;
const RHO_OUT: f64 = 0.01;
const M_SCALE: f64 = 1.0;
const MU_SYM: f64 = 1.0;
const LAMBDA_SYM: f64 = 1.0;
const R_CORE: f64 = 1.0;
const EPS: f64 = 1e-4;

fn main() -> Result<(), cvode::CvodeError> {
    let dr = R_MAX / (N as f64 - 1.0);
    
    // RHS: Symmetron PDE
    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        for i in 0..N {
            let r = i as f64 * dr;
            
            // Smoothed density
            let rho_r = if i == 0 {
                RHO_IN
            } else {
                let tanh_val = ((r - R_CORE) / EPS).tanh();
                RHO_IN * 0.5 * (1.0 - tanh_val) + RHO_OUT
            };
            
            // Effective potential derivative: V'(phi)
            let v_prime = (rho_r / (M_SCALE * M_SCALE) - MU_SYM * MU_SYM) * y[i] + LAMBDA_SYM * y[i].powi(3);
            
            // Laplacian
            let laplacian = if i == 0 {
                // At r=0: d2/dr2 + 2/r d/dr -> 3 * d2/dr2
                let u_1 = y[1];
                let u_0 = y[0];
                3.0 * 2.0 * (u_1 - u_0) / (dr * dr)
            } else {
                let u_l = y[i - 1];
                let u_r = if i == N - 1 {
                    // Right boundary condition: VEV
                    MU_SYM / LAMBDA_SYM.sqrt()
                } else {
                    y[i + 1]
                };
                let d2u_dr2 = (u_l - 2.0 * y[i] + u_r) / (dr * dr);
                let du_dr = (u_r - u_l) / (2.0 * dr);
                d2u_dr2 + (2.0 / r) * du_dr
            };
            
            ydot[i] = laplacian - v_prime;
        }
        Ok(())
    };

    // Initial condition: start at VEV everywhere (or 0)
    // Actually, starting at VEV is fine, it will evolve to the true solution.
    let mut y0_data = vec![MU_SYM / LAMBDA_SYM.sqrt(); N];
    let y0 = SerialVector::from_slice(&y0_data);

    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-6)
        .atol(1e-8)
        .init_step(1e-4)
        .max_order(5)
        .max_steps(100_000)
        .build(rhs, 0.0, y0)?;

    println!("Evolving Symmetron field to steady state...");
    
    // Evolve to large time t to reach steady state
    let t_end = 100.0;
    let (t, y) = solver.solve(t_end, Task::Normal)?;
    
    println!("Reached steady state at t = {:.2}", t);
    
    // Output to CSV for workshopcosmo.py to ingest
    let mut file = File::create("symmetron_screening_rs.csv").expect("Unable to create file");
    writeln!(file, "r,phi").expect("Unable to write");
    for i in 0..N {
        let r = i as f64 * dr;
        writeln!(file, "{},{}", r, y[i]).expect("Unable to write");
    }
    
    let phi_0 = y[0];
    let vev = MU_SYM / LAMBDA_SYM.sqrt();
    println!("Field value at core r=0: {:.6}", phi_0);
    println!("Screening suppression factor: {:.6e}", phi_0 / vev);
    
    Ok(())
}
