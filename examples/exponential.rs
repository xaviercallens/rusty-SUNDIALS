//! Simple exponential decay: dy/dt = -y, y(0) = 1
//! Exact solution: y(t) = exp(-t)

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

fn main() -> Result<(), cvode::CvodeError> {
    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = -y[0];
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-6)
        .atol(1e-10)
        .build(rhs, 0.0, y0)?;

    println!("Exponential decay: dy/dt = -y, y(0) = 1");
    println!("{:>10} {:>15} {:>15} {:>12}", "t", "y(t)", "exact", "error");
    println!("{}", "-".repeat(55));

    for &tout in &[0.1, 0.5, 1.0, 2.0, 5.0, 10.0] {
        let (t, y) = solver.solve(tout, Task::Normal)?;
        let exact = (-t).exp();
        let error = (y[0] - exact).abs();
        println!("{t:10.4} {y:15.10e} {exact:15.10e} {error:12.4e}", y = y[0]);
    }

    println!("\nSolver statistics:");
    println!("  Steps: {}", solver.num_steps());
    println!("  RHS evals: {}", solver.num_rhs_evals());
    println!("  Final step size: {:.4e}", solver.step_size());

    Ok(())
}
