//! HIRES — High Irradiance RESponse (Hairer & Wanner test set).
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let rhs = |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        f[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
        f[1] = 1.71 * y[0] - 8.75 * y[1];
        f[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
        f[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
        f[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
        f[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
        f[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
        f[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-3)
        .atol(1e-6)
        .max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("HIRES Benchmark (8 species)");
    println!(
        "{:>10} {:>12} {:>12} {:>12} {:>12}",
        "t", "y1", "y2", "y7", "y8"
    );
    println!("{}", "-".repeat(62));
    for &tout in &[0.5, 1.0, 5.0, 10.0, 50.0, 100.0] {
        let (t, y) = solver.solve(tout, Task::Normal)?;
        println!(
            "{t:10.2} {:12.6e} {:12.6e} {:12.6e} {:12.6e}",
            y[0], y[1], y[6], y[7]
        );
    }
    let y = solver.y();
    let total: f64 = y.iter().sum();
    println!("\nConservation: Σy = {total:.10} (expect 1.0057)");
    println!(
        "Steps: {}, RHS evals: {}",
        solver.num_steps(),
        solver.num_rhs_evals()
    );
    Ok(())
}
