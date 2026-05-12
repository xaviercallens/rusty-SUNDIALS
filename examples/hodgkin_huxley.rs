//! Hodgkin-Huxley Neuron Model — Nobel Prize 1963.
//! Reference: Hodgkin & Huxley (1952). J. Physiol. 117(4), 500–544.
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
const G_NA: f64 = 120.0;
const G_K: f64 = 36.0;
const G_L: f64 = 0.3;
const E_NA: f64 = 50.0;
const E_K: f64 = -77.0;
const E_L: f64 = -54.387;
fn alpha_m(v: f64) -> f64 {
    let d = v + 40.0;
    if d.abs() < 1e-7 {
        1.0
    } else {
        0.1 * d / (1.0 - (-0.1 * d).exp())
    }
}
fn beta_m(v: f64) -> f64 {
    4.0 * (-(v + 65.0) / 18.0).exp()
}
fn alpha_h(v: f64) -> f64 {
    0.07 * (-(v + 65.0) / 20.0).exp()
}
fn beta_h(v: f64) -> f64 {
    1.0 / (1.0 + (-(v + 35.0) / 10.0).exp())
}
fn alpha_n(v: f64) -> f64 {
    let d = v + 55.0;
    if d.abs() < 1e-7 {
        0.1
    } else {
        0.01 * d / (1.0 - (-0.1 * d).exp())
    }
}
fn beta_n(v: f64) -> f64 {
    0.125 * (-(v + 65.0) / 80.0).exp()
}
fn main() -> Result<(), cvode::CvodeError> {
    let i_ext = 10.0;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        let (v, m, h, n) = (y[0], y[1], y[2], y[3]);
        f[0] = (i_ext
            - G_NA * m.powi(3) * h * (v - E_NA)
            - G_K * n.powi(4) * (v - E_K)
            - G_L * (v - E_L))
            / 1.0;
        f[1] = alpha_m(v) * (1.0 - m) - beta_m(v) * m;
        f[2] = alpha_h(v) * (1.0 - h) - beta_h(v) * h;
        f[3] = alpha_n(v) * (1.0 - n) - beta_n(v) * n;
        Ok(())
    };
    let v0 = -65.0;
    let y0 = SerialVector::from_slice(&[
        v0,
        alpha_m(v0) / (alpha_m(v0) + beta_m(v0)),
        alpha_h(v0) / (alpha_h(v0) + beta_h(v0)),
        alpha_n(v0) / (alpha_n(v0) + beta_n(v0)),
    ]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-3)
        .atol(1e-5)
        .max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("Hodgkin-Huxley Neuron (I_ext={i_ext} µA/cm²)");
    println!(
        "{:>8} {:>12} {:>10} {:>10} {:>10}",
        "t(ms)", "V(mV)", "m", "h", "n"
    );
    println!("{}", "-".repeat(54));
    for i in 1..=20 {
        let (t, y) = solver.solve(0.5 * i as f64, Task::Normal)?;
        println!(
            "{t:8.2} {v:12.4} {m:10.6} {h:10.6} {n:10.6}",
            v = y[0],
            m = y[1],
            h = y[2],
            n = y[3]
        );
    }
    println!(
        "\nSteps: {}, RHS evals: {}",
        solver.num_steps(),
        solver.num_rhs_evals()
    );
    Ok(())
}
