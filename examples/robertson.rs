//! Robertson chemical kinetics — classic stiff ODE test problem.
//!
//! dy1/dt = -0.04*y1 + 1e4*y2*y3
//! dy2/dt =  0.04*y1 - 1e4*y2*y3 - 3e7*y2^2
//! dy3/dt =  3e7*y2^2
//!
//! y(0) = [1, 0, 0], t ∈ [0, 4e10]
//!
//! Reference: SUNDIALS 7.4.0 cvRoberts_dns
//! C reference: steps=1070, rhs_evals=1537, conservation<1e-15

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::time::Instant;

fn main() -> Result<(), cvode::CvodeError> {
    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        ydot[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        ydot[2] = 3e7 * y[1] * y[1];
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0, 0.0, 0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-4)
        .atol(1e-8)
        .max_order(5) // BDF-5 — same as LLNL cvRoberts_dns
        .init_step(1e-4) // LLNL h0 default for Robertson
        .max_steps(50000)
        // Analytical Jacobian — matches LLNL cvRoberts_dns dense Jacobian.
        // Eliminates 3 extra RHS evals per Jacobian compute (one per column of
        // the 3×3 FD Jacobian), giving exact Newton directions.
        // Layout: cols[j][i] = ∂f_i/∂y_j (column-major, same as SUNDIALS SUNDenseMatrix).
        //
        //   f0 = -0.04*y0 + 1e4*y1*y2
        //   f1 =  0.04*y0 - 1e4*y1*y2 - 3e7*y1²
        //   f2 =  3e7*y1²
        .jacobian(|_t, y, j| {
            // Column 0: ∂f/∂y0
            j.cols[0][0] = -0.04;
            j.cols[0][1] = 0.04;
            j.cols[0][2] = 0.0;
            // Column 1: ∂f/∂y1
            j.cols[1][0] = 1e4 * y[2];
            j.cols[1][1] = -1e4 * y[2] - 6e7 * y[1];
            j.cols[1][2] = 6e7 * y[1];
            // Column 2: ∂f/∂y2
            j.cols[2][0] = 1e4 * y[1];
            j.cols[2][1] = -1e4 * y[1];
            j.cols[2][2] = 0.0;
            Ok(())
        })
        .build(rhs, 0.0, y0)?;

    println!("Robertson Chemical Kinetics (stiff system)");
    println!("BDF method with adaptive step size + analytical Jacobian");
    println!("Reference: SUNDIALS 7.4.0 cvRoberts_dns (C)");
    println!("{:>12} {:>14} {:>14} {:>14}", "t", "y1", "y2", "y3");
    println!("{}", "-".repeat(58));

    let times = [
        0.4, 4.0, 40.0, 400.0, 4000.0, 40000.0, 4e5, 4e6, 4e7, 4e8, 4e9, 4e10,
    ];

    let start = Instant::now();
    for &tout in &times {
        let (t, y) = solver.solve(tout, Task::Normal)?;
        println!(
            "{t:12.4e} {y1:14.6e} {y2:14.6e} {y3:14.6e}",
            y1 = y[0],
            y2 = y[1],
            y3 = y[2]
        );
    }
    let wall_ms = start.elapsed().as_millis();

    let steps = solver.num_steps();
    let rhs_evals = solver.num_rhs_evals();

    println!("\nSolver statistics:");
    println!(
        "  Steps:      {steps}  (C ref: 1070, ratio: {:.1}x)",
        steps as f64 / 1070.0
    );
    println!(
        "  RHS evals:  {rhs_evals}  (C ref: 1537, ratio: {:.1}x)",
        rhs_evals as f64 / 1537.0
    );
    println!("  Final order: {}", solver.order());
    println!("  Wall time:  {wall_ms}ms");

    // Verify conservation: y1 + y2 + y3 = 1
    let y = solver.y();
    let sum = y[0] + y[1] + y[2];
    let conservation_error = (sum - 1.0).abs();
    println!("  Conservation (y1+y2+y3): {sum:.15e} (should be 1.0)");
    println!("  Conservation error: {conservation_error:.2e}  (threshold 1e-12)");

    if conservation_error < 1e-12 {
        println!("  Conservation: PASS ✓");
    } else {
        println!("  Conservation: FAIL ✗");
        std::process::exit(1);
    }

    Ok(())
}
