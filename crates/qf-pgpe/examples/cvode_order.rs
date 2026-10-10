//! Order of the IF-RK4 step against rusty-SUNDIALS CVODE as an independent reference (data of the K2 figure):
//! `cargo run --release -p qf-pgpe --example cvode_order` prints one JSON object with the global error of the
//! `qf-pgpe` step at `dt = 0.04 ... 0.0025` against a CVODE (Adams, rtol 1e-12/atol 1e-14) solution of the same
//! right-hand side (`ComplexField2D::rhs`), the fitted order, and the CVODE cost (needs the Adams fix of PR #63).
use cvode::{Cvode, Method, Task};
use num_complex::Complex64;
use nvector::SerialVector;
use qf_pgpe::ComplexField2D;

fn main() {
    let t_end = 1.0;
    let f0 = ComplexField2D::new(16, 16.0, 1.0, 0.01);
    let k1 = 2.0 * std::f64::consts::PI / f0.l;
    let psi: Vec<Complex64> = (0..16 * 16)
        .map(|idx| {
            let (x, y) = ((idx / 16) as f64 * f0.dx, (idx % 16) as f64 * f0.dx);
            Complex64::new(1.0, 0.0)
                + 0.15 * Complex64::new(0.0, k1 * x + 2.0 * k1 * y).exp()
                + 0.08 * Complex64::new(0.0, -2.0 * k1 * x + k1 * y).exp()
        })
        .collect();
    let c0 = f0.modes(&psi);
    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot.copy_from_slice(&f0.pack(&f0.rhs(&f0.unpack(y))));
        Ok(())
    };
    let mut solver = Cvode::builder(Method::Adams)
        .rtol(1e-12)
        .atol(1e-14)
        .max_steps(2_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&f0.pack(&c0)))
        .expect("cvode");
    let (_, y) = solver
        .solve(t_end, Task::Normal)
        .expect("cvode solve (needs the Adams-order fix of PR #63)");
    let yref = y.to_vec();
    let (nfe, nst) = (solver.num_rhs_evals(), solver.num_steps());
    let mut rows = Vec::new();
    for dt in [0.04, 0.02, 0.01, 0.005, 0.0025] {
        let f = ComplexField2D::new(16, 16.0, 1.0, dt);
        let err = f
            .pack(&f.run(&c0, t_end))
            .iter()
            .zip(&yref)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        rows.push((dt, err));
    }
    let n = rows.len();
    let slope = ((rows[0].1 / rows[n - 1].1).ln() / (rows[0].0 / rows[n - 1].0).ln()).abs();
    let pts: Vec<String> = rows.iter().map(|(dt, e)| format!("[{dt},{e:e}]")).collect();
    println!(
        "{{\"n\":16,\"modes\":{},\"t_end\":{t_end},\"cvode_rhs_evals\":{nfe},\"cvode_steps\":{nst},\"fitted_order\":{slope:.3},\"dt_error\":[{}]}}",
        f0.n_modes(),
        pts.join(",")
    );
}
