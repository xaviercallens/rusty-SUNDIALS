//! Rössler Attractor — Rössler (1976), Phys. Lett. A.
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let a = 0.2;
    let b = 0.2;
    let c = 5.7;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        f[0] = -y[1] - y[2];
        f[1] = y[0] + a * y[1];
        f[2] = b + y[2] * (y[0] - c);
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[1.0, 1.0, 0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-3)
        .atol(1e-5)
        .max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("Rössler Attractor (a={a}, b={b}, c={c})");
    println!("{:>8} {:>14} {:>14} {:>14}", "t", "x", "y", "z");
    println!("{}", "-".repeat(54));
    for i in 1..=20 {
        let (t, y) = solver.solve(1.0 * i as f64, Task::Normal)?;
        println!("{t:8.1} {:14.6e} {:14.6e} {:14.6e}", y[0], y[1], y[2]);
    }
    println!(
        "\nSteps: {}, RHS evals: {}",
        solver.num_steps(),
        solver.num_rhs_evals()
    );
    Ok(())
}
