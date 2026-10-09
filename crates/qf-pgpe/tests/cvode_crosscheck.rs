//! Independent-integrator cross-check of the PGPE engine with rusty-SUNDIALS' own CVODE.
//!
//! The projected GP equation on the modes inside the projector is an ODE system `dy/dt = f(y)` of dimension
//! `2 * n_modes` (`ComplexField2D::{rhs, pack, unpack}`). `qf-pgpe` integrates it with its integrating-factor RK4
//! (`ComplexField2D::step`); here the same right-hand side is handed, unchanged, to `cvode::Cvode` (variable-order
//! Adams, tight tolerances) as an independent reference. Two checks:
//!
//! 1. **Agreement.** The two integrations of the same smooth initial state agree (IF-RK4 at `dt = 0.01`: 4.6e-7 over 1 time unit).
//! 2. **Order (the K2 known answer of `PGPE_BKT_PREREG.md`, previously "not ported").** The IF-RK4 global error
//!    measured against the CVODE reference falls as `dt^4` (fitted slope 4.0 over `dt = 0.04, 0.02, 0.01`: errors 1.2e-4, 7.3e-6, 4.6e-7).
//!
//! Requires the Adams-order fix of rusty-SUNDIALS PR #63 (`cvSetAdams`): on the unfixed `main`, Adams stays at order 1
//! on this Hamiltonian problem, the step size collapses and the solve fails with `MaxSteps` after ~10^6 RHS evaluations
//! (measured: 200 000 steps, t = 0.39). This test is therefore also a regression guard for that fix.
use cvode::{Cvode, Method, Task};
use num_complex::Complex64;
use nvector::SerialVector;
use qf_pgpe::ComplexField2D;

fn smooth_state(f: &ComplexField2D) -> Vec<Complex64> {
    let n = f.n;
    let k1 = 2.0 * std::f64::consts::PI / f.l;
    let mut psi = vec![Complex64::new(0.0, 0.0); n * n];
    for i in 0..n {
        for j in 0..n {
            let (x, y) = (i as f64 * f.dx, j as f64 * f.dx);
            psi[i * n + j] = Complex64::new(1.0, 0.0)
                + 0.15 * Complex64::new(0.0, k1 * x + 2.0 * k1 * y).exp()
                + 0.08 * Complex64::new(0.0, -2.0 * k1 * x + k1 * y).exp();
        }
    }
    f.modes(&psi)
}

fn cvode_reference(f: &ComplexField2D, c0: &[Complex64], t_end: f64) -> (Vec<f64>, usize) {
    let y0 = SerialVector::from_slice(&f.pack(c0));
    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        let c = f.unpack(y);
        ydot.copy_from_slice(&f.pack(&f.rhs(&c)));
        Ok(())
    };
    let mut solver = Cvode::builder(Method::Adams)
        .rtol(1e-10)
        .atol(1e-12)
        .max_steps(200_000)
        .build(rhs, 0.0, y0)
        .expect("cvode build");
    let (_, y) = match solver.solve(t_end, Task::Normal) {
        Ok(r) => r,
        Err(e) => panic!(
            "cvode solve failed: {e:?} after {} steps, {} RHS evaluations",
            solver.num_steps(),
            solver.num_rhs_evals()
        ),
    };
    let y = y.to_vec();
    (y, solver.num_rhs_evals())
}

fn max_err(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

#[test]
fn cvode_and_if_rk4_agree_and_rk4_is_fourth_order() {
    let t_end = 1.0;
    let f0 = ComplexField2D::new(16, 16.0, 1.0, 0.01);
    assert!(
        f0.n_modes() > 40,
        "tiny system expected, got {} modes",
        f0.n_modes()
    );
    let c0 = smooth_state(&f0);
    let (yref, nfe) = cvode_reference(&f0, &c0, t_end);
    let cref = f0.unpack(&yref);
    assert!((f0.norm(&cref) - f0.norm(&c0)).abs() / f0.norm(&c0) < 1e-9);
    let mut errs = Vec::new();
    for dt in [0.04, 0.02, 0.01] {
        let f = ComplexField2D::new(16, 16.0, 1.0, dt);
        let c = f.run(&c0, t_end);
        errs.push(max_err(&f.pack(&c), &yref));
    }
    println!("CVODE reference: {nfe} RHS evaluations; IF-RK4 errors dt=0.04/0.02/0.01: {errs:?}");
    assert!(errs[2] < 1e-6, "IF-RK4 (dt = 0.01) vs CVODE: {:e}", errs[2]);
    let slope = ((errs[0] / errs[2]).ln() / (4.0_f64).ln()).abs();
    assert!((3.8..=4.2).contains(&slope), "fitted order {slope}");
    // the reference itself: with the Adams-order fix (PR #63) CVODE needs a few hundred RHS evaluations; without it the
    // step collapses (> 10^6 evaluations, MaxSteps) because Adams is stuck at order 1 on this oscillatory problem.
    assert!(nfe < 5_000, "CVODE used {nfe} RHS evaluations");
}
