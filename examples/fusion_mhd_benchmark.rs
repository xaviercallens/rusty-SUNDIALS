//! "xMHD Fusion Benchmark" — AUDITED 2026-09-27 (docs/audit/fusion-2026-09-27/README.md)
//!
//! AUDIT NOTE: this example does NOT model reduced MHD, a tearing mode, or any
//! anisotropic operator. The right-hand side is the 10-variable linear decay
//! ẏ = -y. The "Vanilla" and "SciML" paths below run the *identical* CVODE BDF
//! call. The earlier version manufactured a 10x-50x "speedup" by inserting
//! `thread::sleep(100 ms)` into one path and `sleep(2 ms)` into the other, and
//! printed "MATHEMATICALLY VERIFIED" (report B §3 C5: without the sleeps the
//! ratio is 1.00). The sleeps and the verdict lines were removed. The timing
//! ratio printed now is whatever the two identical runs actually measure
//! (expected ≈ 1). No Enzyme AD, FGMRES, AI preconditioner or FP8 path exists
//! here; the path labels are kept only so older logs can be compared.
//!
//! The one real check is accuracy: y(1) is compared with the exact e^{-1}.

use cvode::Cvode;
use cvode::{Method, Task};
use nvector::SerialVector;
use std::time::Instant;
use sundials_core::Real;

/// Number of independent decaying variables (not a 3D grid).
const GRID_SIZE: usize = 10;

/// RHS: ẏ_i = -y_i (linear, non-stiff, decoupled). Not an MHD model.
fn xmhd_rhs(_t: Real, y: &[Real], ydot: &mut [Real]) -> Result<(), String> {
    for i in 0..GRID_SIZE {
        ydot[i] = -y[i];
    }
    Ok(())
}

/// Integrate ẏ = -y from 0 to 1 in ten output steps; return (t, y0(t), wall time).
fn run_path(label: &str) -> (Real, Real, f64) {
    println!(
        "--- [{}] CVODE BDF on ẏ = -y (identical in both paths) ---",
        label
    );
    let initial_state = SerialVector::from_slice(&[1.0; GRID_SIZE]);
    let mut cvode = Cvode::builder(Method::Bdf)
        .build(xmhd_rhs, 0.0, initial_state)
        .unwrap();

    let start = Instant::now();
    let mut t_curr = 0.0;
    let mut y_first = 1.0;
    for _ in 0..10 {
        let (t, y) = cvode.solve(t_curr + 0.1, Task::Normal).unwrap();
        t_curr = t;
        y_first = y[0];
    }
    let duration = start.elapsed().as_secs_f64();
    println!(
        "Integration complete. Reached t={:.2}, y0={:.8}",
        t_curr, y_first
    );
    println!("Time: {:.6}s\n", duration);
    (t_curr, y_first, duration)
}

fn main() {
    println!("==================================================");
    println!(" fusion_mhd_benchmark (audited): ẏ = -y timing demo ");
    println!(" No MHD physics; no speedup claim is made.         ");
    println!("==================================================\n");

    let (_t_a, y_a, time_a) = run_path("path A (formerly 'Vanilla')");
    let (_t_b, y_b, time_b) = run_path("path B (formerly 'SciML')");

    let exact = (-1.0_f64).exp();
    let rel_err_a = ((y_a - exact) / exact).abs();
    let rel_err_b = ((y_b - exact) / exact).abs();

    println!("==================================================");
    println!(" RESULTS (measured)");
    println!("==================================================");
    println!("Path A time: {:.6}s", time_a);
    println!("Path B time: {:.6}s", time_b);
    if time_b > 0.0 {
        println!(
            "Timing ratio A/B: {:.2} (both paths run the same solver; ≈1 expected)",
            time_a / time_b
        );
    }
    println!(
        "y(1) path A = {:.8}, rel. error vs e^-1 = {:.3e}",
        y_a, rel_err_a
    );
    println!(
        "y(1) path B = {:.8}, rel. error vs e^-1 = {:.3e}",
        y_b, rel_err_b
    );

    // Default rtol is 1e-4; report B measured a relative error of 4.1e-4 at t=1.
    // A 1e-2 band is a loose sanity check, not a verification of any claim.
    if rel_err_a < 1e-2 && rel_err_b < 1e-2 {
        println!("Accuracy sanity check (rel. error < 1e-2): pass");
    } else {
        println!("Accuracy sanity check (rel. error < 1e-2): FAIL");
        std::process::exit(1);
    }
}
