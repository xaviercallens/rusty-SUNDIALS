//! Lorenz Attractor — Deterministic chaos in atmospheric convection.
//! Reference: Lorenz, E.N. (1963). J. Atmos. Sci. 20(2), 130–141.
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let sigma = 10.0;
    let rho = 28.0;
    let beta = 8.0 / 3.0;
    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = sigma * (y[1] - y[0]);
        ydot[1] = y[0] * (rho - y[2]) - y[1];
        ydot[2] = y[0] * y[1] - beta * y[2];
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[1.0, 1.0, 1.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-3)
        .atol(1e-5)
        .max_order(1)
        .max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("Lorenz Attractor (σ={sigma}, ρ={rho}, β={beta:.4})");
    println!("{:>10} {:>14} {:>14} {:>14}", "t", "x", "y", "z");
    println!("{}", "-".repeat(56));
    for i in 1..=20 {
        let tout = 0.25 * i as f64;
        let (t, y) = solver.solve(tout, Task::Normal)?;
        println!(
            "{t:10.3} {x:14.6e} {y:14.6e} {z:14.6e}",
            x = y[0],
            y = y[1],
            z = y[2]
        );
    }
    println!(
        "\nSteps: {}, RHS evals: {}",
        solver.num_steps(),
        solver.num_rhs_evals()
    );
    Ok(())
}
