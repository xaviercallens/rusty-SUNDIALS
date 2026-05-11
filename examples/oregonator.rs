//! Oregonator — A benchmark for stiff ODE solvers.
//! Models the Belousov-Zhabotinsky reaction (Field & Noyes 1974).
//!
//! dy1/dt = 77.27 * (y2 + y1*(1 - 8.375e-6*y1 - y2))
//! dy2/dt = (1/77.27) * (y3 - (1 + y1)*y2)
//! dy3/dt = 0.161 * (y1 - y3)
//!
//! y(0) = [1.0, 2.0, 3.0], t ∈ [0, 360]
//!
//! This problem is highly stiff and serves as a standard test case
//! for stiff ODE solvers, popularized by Hairer & Wanner.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

fn main() -> Result<(), cvode::CvodeError> {
    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        let y1 = y[0];
        let y2 = y[1];
        let y3 = y[2];

        ydot[0] = 77.27 * (y2 + y1 * (1.0 - 8.375e-6 * y1 - y2));
        ydot[1] = (1.0 / 77.27) * (y3 - (1.0 + y1) * y2);
        ydot[2] = 0.161 * (y1 - y3);

        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0, 2.0, 3.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-4)
        .atol(1e-11)
        .max_steps(50000)
        .build(rhs, 0.0, y0)?;

    println!("Oregonator Benchmark (Belousov-Zhabotinsky reaction)");
    println!("BDF method with adaptive step size");
    println!("{:>10} {:>14} {:>14} {:>14}", "t", "y1", "y2", "y3");
    println!("{}", "-".repeat(56));

    let t_end = 360.0;
    let num_output_steps = 10;
    let dt_out = t_end / (num_output_steps as f64);

    for i in 1..=num_output_steps {
        let tout = dt_out * (i as f64);
        let (t, y) = solver.solve(tout, Task::Normal)?;
        println!("{t:10.2e} {y1:14.6e} {y2:14.6e} {y3:14.6e}",
            y1 = y[0], y2 = y[1], y3 = y[2]);
    }

    println!("\nSolver statistics:");
    println!("  Steps: {}", solver.num_steps());
    println!("  RHS evals: {}", solver.num_rhs_evals());
    println!("  Final order: {}", solver.order());

    Ok(())
}
