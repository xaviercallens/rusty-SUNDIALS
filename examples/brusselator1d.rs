//! 1D Brusselator — Classic stiff PDE problem reduced to ODEs via Method of Lines.
//! Models an autocatalytic oscillating chemical reaction with diffusion.
//!
//! u_t = a + u^2 v - (b + 1) u + alpha * u_xx
//! v_t = b u - u^2 v + alpha * v_xx
//!
//! Spatial domain x ∈ [0, 1] with boundary conditions u(0,t)=u(1,t)=a, v(0,t)=v(1,t)=b/a.
//! Standard parameters: a = 1.0, b = 3.0, alpha = 0.02.
//! Initial conditions: u(x,0) = a + sin(pi * x), v(x,0) = b/a.
//!
//! Discretized with N spatial points using central finite differences.
//! This yields a system of 2N stiff ODEs.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

const N: usize = 100; // Spatial points
const A: f64 = 1.0;
const B: f64 = 3.0;
const ALPHA: f64 = 0.02;

fn main() -> Result<(), cvode::CvodeError> {
    let dx = 1.0 / ((N + 1) as f64);
    let dx_sq = dx * dx;
    let coeff = ALPHA / dx_sq;
    let u_bound = A;
    let v_bound = B / A;

    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        for i in 0..N {
            let u = y[2 * i];
            let v = y[2 * i + 1];

            // Left boundaries
            let u_left = if i == 0 { u_bound } else { y[2 * (i - 1)] };
            let v_left = if i == 0 { v_bound } else { y[2 * (i - 1) + 1] };

            // Right boundaries
            let u_right = if i == N - 1 { u_bound } else { y[2 * (i + 1)] };
            let v_right = if i == N - 1 {
                v_bound
            } else {
                y[2 * (i + 1) + 1]
            };

            // Diffusion terms
            let u_diff = coeff * (u_left - 2.0 * u + u_right);
            let v_diff = coeff * (v_left - 2.0 * v + v_right);

            // Reaction terms
            let u_react = A + u * u * v - (B + 1.0) * u;
            let v_react = B * u - u * u * v;

            // Full equations
            ydot[2 * i] = u_diff + u_react;
            ydot[2 * i + 1] = v_diff + v_react;
        }
        Ok(())
    };

    // Initial conditions
    let mut initial_state = vec![0.0; 2 * N];
    for i in 0..N {
        let x = ((i + 1) as f64) * dx;
        initial_state[2 * i] = A + (std::f64::consts::PI * x).sin();
        initial_state[2 * i + 1] = B / A;
    }

    let y0 = SerialVector::from_slice(&initial_state);

    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-4)
        .atol(1e-6)
        .max_steps(100000)
        .build(rhs, 0.0, y0)?;

    println!("1D Brusselator Benchmark (Method of Lines, {} points)", N);
    println!("BDF method with adaptive step size");
    println!(
        "{:>10} {:>14} {:>14} {:>14}",
        "t", "u(mid)", "v(mid)", "Steps"
    );
    println!("{}", "-".repeat(56));

    let t_end = 10.0;
    let num_output_steps = 10;
    let dt_out = t_end / (num_output_steps as f64);

    let mid_idx = N / 2;

    for i in 1..=num_output_steps {
        let tout = dt_out * (i as f64);
        let (t, y) = solver.solve(tout, Task::Normal)?;
        let u_mid = y[2 * mid_idx];
        let v_mid = y[2 * mid_idx + 1];
        let steps = solver.num_steps();
        println!("{t:10.2e} {u_mid:14.6e} {v_mid:14.6e} {steps:14}");
    }

    println!("\nSolver statistics:");
    println!("  Steps: {}", solver.num_steps());
    println!("  RHS evals: {}", solver.num_rhs_evals());

    Ok(())
}
