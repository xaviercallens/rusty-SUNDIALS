//! Appendix F, example 7a: a stiff scalar ODE with CVODE (BDF), and why explicit Euler fails.
//!
//! `y' = lambda (y - cos t) - sin t`, `y(0) = 2`, `lambda = -1e4`: exact solution
//! `cos t + e^{lambda t}`. Positive control: CVODE matches the exact solution. Negative control:
//! explicit Euler is stable for `h < 2 / |lambda| = 2e-4` and explodes for `h = 2.5e-4`.
//!
//! Usage: `tb_stiff` (prints a table; exit code 1 if a control misbehaves).

use qf_textbook_examples::{explicit_euler, prothero_robinson};

fn main() {
    let (lambda, y0) = (-1e4, 2.0);
    let t_out = [1e-4, 1e-3, 0.5, 1.0, 2.0, 5.0, 10.0];
    let run = prothero_robinson(lambda, y0, &t_out, 1e-8).expect("CVODE run");
    println!("CVODE BDF, lambda = {lambda:e}, rtol = 1e-8");
    println!(
        "{:>8} {:>22} {:>22} {:>10}",
        "t", "y_cvode", "y_exact", "|err|"
    );
    let mut max_err: f64 = 0.0;
    for &(t, y, ex) in &run.samples {
        max_err = max_err.max((y - ex).abs());
        println!("{t:8.4} {y:22.16e} {ex:22.16e} {:10.2e}", (y - ex).abs());
    }
    println!("steps = {}, rhs evals = {}", run.steps, run.rhs_evals);
    let euler_ok = explicit_euler(lambda, y0, 1e-4, 1.0);
    let euler_bad = explicit_euler(lambda, y0, 2.5e-4, 1.0);
    println!("explicit Euler h = 1.0e-4 (10000 steps): y(1) = {euler_ok:.10}");
    let euler_100 = explicit_euler(lambda, y0, 2.5e-4, 0.025);
    println!("explicit Euler h = 2.5e-4 (100 steps):   y(0.025) = {euler_100:.3e}");
    println!("explicit Euler h = 2.5e-4 (4000 steps):  y(1) = {euler_bad:.3e}");
    let blown_up = !euler_bad.is_finite() || euler_bad.abs() > 1e6;
    let ok = max_err < 1e-6 && (euler_ok - 1f64.cos()).abs() < 1e-3 && blown_up;
    println!("controls as expected: {ok}");
    std::process::exit(if ok { 0 } else { 1 });
}
