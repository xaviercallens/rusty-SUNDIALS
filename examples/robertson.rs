//! Robertson chemical kinetics — classic stiff ODE test problem.
//!
//! dy1/dt = -0.04*y1 + 1e4*y2*y3
//! dy2/dt =  0.04*y1 - 1e4*y2*y3 - 3e7*y2^2
//! dy3/dt =  3e7*y2^2
//!
//! y(0) = [1, 0, 0], t ∈ [0, 4e10]
//!
//! This is the standard SUNDIALS CVODE example (cvRoberts_dns).

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

fn main() -> Result<(), cvode::CvodeError> {
    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        ydot[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        ydot[2] = 3e7 * y[1] * y[1];
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0, 0.0, 0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-4)
        .atol(1e-8)
        .max_steps(50000)
        .build(rhs, 0.0, y0)?;

    println!("Robertson Chemical Kinetics (stiff system)");
    println!("BDF method with adaptive step size");
    println!("{:>12} {:>14} {:>14} {:>14}", "t", "y1", "y2", "y3");
    println!("{}", "-".repeat(58));

    let times = [0.4, 4.0, 40.0, 400.0, 4000.0, 40000.0, 4e5, 4e6, 4e7, 4e8, 4e9, 4e10];

    for &tout in &times {
        let (t, y) = solver.solve(tout, Task::Normal)?;
        println!("{t:12.4e} {y1:14.6e} {y2:14.6e} {y3:14.6e}",
            y1 = y[0], y2 = y[1], y3 = y[2]);
    }

    println!("\nSolver statistics:");
    println!("  Steps: {}", solver.num_steps());
    println!("  RHS evals: {}", solver.num_rhs_evals());
    println!("  Final order: {}", solver.order());

    // Verify conservation: y1 + y2 + y3 = 1
    let y = solver.y();
    let sum = y[0] + y[1] + y[2];
    println!("  Conservation (y1+y2+y3): {sum:.15e} (should be 1.0)");

    Ok(())
}
