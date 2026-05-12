//! SIR Epidemic — Kermack-McKendrick (1927).
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let n_pop = 1000.0;
    let beta = 0.3;
    let gamma = 0.1;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        let inf = beta * y[0] * y[1] / n_pop;
        f[0] = -inf;
        f[1] = inf - gamma * y[1];
        f[2] = gamma * y[1];
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[999.0, 1.0, 0.0]);
    let mut solver = Cvode::builder(Method::Adams)
        .rtol(1e-3)
        .atol(1e-5)
        .max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!(
        "SIR Epidemic (β={beta}, γ={gamma}, R₀={:.1}, N={n_pop})",
        beta / gamma
    );
    println!(
        "{:>8} {:>12} {:>12} {:>12} {:>12}",
        "t(days)", "S", "I", "R", "S+I+R"
    );
    println!("{}", "-".repeat(60));
    for &tout in &[5.0, 10.0, 20.0, 30.0, 50.0, 75.0, 100.0, 150.0, 200.0] {
        let (t, y) = solver.solve(tout, Task::Normal)?;
        println!(
            "{t:8.1} {:12.2} {:12.2} {:12.2} {:12.4}",
            y[0],
            y[1],
            y[2],
            y[0] + y[1] + y[2]
        );
    }
    println!(
        "\nSteps: {}, RHS evals: {}",
        solver.num_steps(),
        solver.num_rhs_evals()
    );
    Ok(())
}
