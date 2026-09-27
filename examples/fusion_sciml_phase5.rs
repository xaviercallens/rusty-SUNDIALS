//! "Experimental SciML Phase 5" — AUDITED 2026-09-27 (docs/audit/fusion-2026-09-27/README.md)
//!
//! AUDIT NOTE: the earlier version of this example printed metrics that were
//! string literals, not measurements (report A §2, report B §2):
//!   * "Speedup: 145x (Stiffness completely bypassed)"      — literal
//!   * "memory footprint reduced by 99.9%"                   — literal
//!   * FGMRES residuals 1.0e0 / 3.4e-4 / 1.2e-9, "3 iterations
//!     (Vanilla AMG takes 5,000+)"                           — literal printlns
//!   * "Ghost Sensitivities computed"                         — two tokio sleeps
//!     (400 ms / 350 ms); nothing was computed
//! plus `thread::sleep` calls of 150/200/50/300 ms that only simulated work.
//! All sleeps and literal metrics were removed. What remains is labelled for
//! what it is: the only real computation is a 10-variable ẏ = -y solve. No
//! IMEX splitting, autoencoder, FGMRES, GNN preconditioner, FP8 path, Tensor
//! Core offload or sensitivity analysis exists in this example.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::time::Instant;
use sundials_core::Real;

/// Number of independent decaying variables (not a latent space).
const LATENT_DIM: usize = 10;

const NOT_MEASURED: &str = "NOT MEASURED — demo placeholder (no implementation in this example)";

/// Solve ẏ = -y (LATENT_DIM copies) from 0 to 1 with the given method.
/// Returns (t reached, y0(t), wall time in seconds).
fn solve_decay(method: Method) -> (Real, Real, f64) {
    let f = |_t: Real, y: &[Real], ydot: &mut [Real]| {
        for i in 0..LATENT_DIM {
            ydot[i] = -y[i];
        }
        Ok(())
    };
    let initial_state = SerialVector::from_slice(&vec![1.0; LATENT_DIM]);
    let mut cvode = Cvode::builder(method)
        .max_steps(50000)
        .build(f, 0.0, initial_state)
        .unwrap();
    let start = Instant::now();
    let (t, y) = cvode.solve(1.0, Task::Normal).unwrap();
    (t, y[0], start.elapsed().as_secs_f64())
}

/// 1. "Dynamic IMEX splitting": only a plain Adams solve of ẏ = -y is run.
fn run_dynamic_imex_splitting() {
    println!("\n▶ Item 1: 'Dynamic IMEX splitting'");
    println!("  [Audit] No spectral analysis or IMEX split is implemented here.");
    let (t, y0, secs) = solve_decay(Method::Adams);
    println!(
        "  [Measured] Adams solve of ẏ = -y reached t={:.2}, y0={:.6} (exact {:.6}) in {:.6}s",
        t,
        y0,
        (-1.0_f64).exp(),
        secs
    );
    println!("  [Speedup vs any baseline] {}", NOT_MEASURED);
}

/// 2. "Latent-space implicit integration (LSI²)": not implemented.
fn run_latent_space_integration() {
    println!("\n▶ Item 2: 'Latent-Space Implicit Integration (LSI²)'");
    println!("  [Audit] No autoencoder, latent space or Enzyme AD is implemented here.");
    println!("  [Memory reduction] {}", NOT_MEASURED);
}

/// 3. "Field-aligned graph preconditioning (FLAGNO)": not implemented.
fn run_flagno_preconditioning() {
    println!("\n▶ Item 3: 'Field-Aligned Graph Preconditioning (FLAGNO)'");
    println!("  [Audit] No graph, GNN, FGMRES or Tensor Core code is run here.");
    println!("  [FGMRES iterations / residuals] {}", NOT_MEASURED);
}

/// 4. "Asynchronous ghost sensitivities": two identical decay solves are run
/// concurrently on tokio blocking threads. No sensitivities are computed.
async fn run_ghost_sensitivities() {
    println!("\n▶ Item 4: 'Asynchronous Ghost Sensitivities'");
    println!("  [Audit] No sensitivity equations and no GPU/FP8 path are implemented here.");
    let start = Instant::now();
    let a = tokio::task::spawn_blocking(|| solve_decay(Method::Bdf));
    let b = tokio::task::spawn_blocking(|| solve_decay(Method::Bdf));
    let (ra, rb) = tokio::join!(a, b);
    let (ta, ya, _) = ra.expect("task A panicked");
    let (tb, yb, _) = rb.expect("task B panicked");
    println!(
        "  [Measured] two concurrent BDF solves of ẏ = -y: t={:.2}/{:.2}, y0={:.6}/{:.6}, wall {:?}",
        ta,
        tb,
        ya,
        yb,
        start.elapsed()
    );
    println!("  [Sensitivities] {}", NOT_MEASURED);
}

#[tokio::main]
async fn main() {
    println!("============================================================");
    println!(" fusion_sciml_phase5 (audited): labelled demo, no SciML claims");
    println!("============================================================");

    run_dynamic_imex_splitting();
    run_latent_space_integration();
    run_flagno_preconditioning();
    run_ghost_sensitivities().await;

    println!("\n============================================================");
    println!(" Done. Nothing in this example validates a fusion/ITER claim.");
    println!("============================================================");
}
