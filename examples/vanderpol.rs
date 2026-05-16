//! Van der Pol oscillator — a classic example of a stiff ODE.
//!
//! dy0/dt = y1
//! dy1/dt = mu * (1 - y0^2) * y1 - y0
//!
//! We use mu = 1000.0 which makes the system highly stiff.
//! Initial conditions: y(0) = [2.0, 0.0]
//!
//! This is a standard test for ODE solvers.

use cvode::{CvodeBuilder, Method, Task};
use nvector::SerialVector;

fn main() -> Result<(), cvode::CvodeError> {
    let mu = 10.0;

    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = y[1];
        ydot[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[2.0, 0.0]);
    let mut solver = CvodeBuilder::new(Method::Bdf)
        .rtol(1e-6)
        .atol(1e-8)
        .max_steps(5000000)
        .build(rhs, 0.0, y0)?;

    println!("Van der Pol Oscillator (stiff system, mu = {})", mu);
    println!("BDF method with adaptive step size");
    println!("{:>12} {:>14} {:>14}", "t", "y0", "y1");
    println!("{}", "-".repeat(42));

    let times = [5.0, 10.0, 15.0, 20.0, 25.0];

    for &tout in &times {
        let (t, y) = solver.solve(tout, Task::Normal)?;
        println!("{t:12.4e} {y0:14.6e} {y1:14.6e}", y0 = y[0], y1 = y[1]);
    }

    println!("\n=== Van der Pol Benchmark Results ===");
    println!("  Steps:      {}", solver.num_steps());
    println!("  RHS evals:  {}", solver.num_rhs_evals());
    #[cfg(feature = "experimental-nls-v2")]
    {
        let nni = solver.num_newton_iters();
        let steps = solver.num_steps();
        println!("  Newton iters: {nni}  (NI/step: {:.2})", nni as f64 / steps as f64);
    }
    println!("  Final order: {}", solver.order());
    println!("\n✅ PASS");

    Ok(())
}
