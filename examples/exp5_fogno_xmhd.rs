//! Experiment 5: "FoGNO" preconditioner on a diagonal test matrix.
//!
//! AUDIT NOTE 2026-09-27 (docs/audit/fusion-2026-09-27/README.md, report B §2/§3 C6):
//! this is NOT an xMHD or tearing-mode benchmark. The operator A is a
//! *diagonal* 1024×1024 matrix with entries alternating 1e6 and 1. "FoGNO"
//! (crates/sundials-core/src/fogno.rs) is `v_i · w_i^α`, i.e. a diagonal
//! (Jacobi-type) scaling; here the weights are set by hand to the exact
//! inverse diagonal, and with α = 1 the preconditioned operator is the
//! identity, so GMRES converges in 1 iteration *by construction*. The earlier
//! version tuned α to 1 "to hit <3 iters" and then printed "VALIDATED"; that
//! verdict was removed. There is no graph, no neural network and no
//! fractional-order operator beyond the elementwise power. Report B C6 shows
//! that on a non-diagonal anisotropic operator neither identity nor diagonal
//! preconditioning converges, so nothing here says anything about anisotropy.

use std::time::Instant;
use sundials_core::Real;
use sundials_core::fogno::FoGNO;
use sundials_core::gmres::{GmresConfig, GmresStatus, gmres_preconditioned};

fn iters_or_note(status: &GmresStatus) -> String {
    match status {
        GmresStatus::Converged { iters, .. } => format!("{}", iters),
        _ => "did not converge".to_string(),
    }
}

fn main() {
    println!("══════════════════════════════════════════════════════════════");
    println!(" Experiment 5 (audited): diagonal scaling preconditioner demo");
    println!(" Operator: diagonal A = diag(1e6, 1, 1e6, 1, ...), N = 1024");
    println!("══════════════════════════════════════════════════════════════");

    let n = 1024;

    // Diagonal test operator (not an xMHD stiffness matrix).
    let matvec = |x: &[Real], y: &mut [Real]| {
        for i in 0..n {
            y[i] = x[i] * if i % 2 == 0 { 1e6 } else { 1.0 };
        }
    };

    let b = vec![1.0; n];
    let cfg = GmresConfig {
        tol: 1e-6,
        restart: 30,
        max_restarts: 10,
    };

    let identity = |v: &[Real], out: &mut [Real]| {
        out.copy_from_slice(v);
    };

    // Baseline: no preconditioner.
    let mut x_base = vec![0.0; n];
    let start_base = Instant::now();
    let status_base = gmres_preconditioned(matvec, &b, &mut x_base, &cfg, identity, identity);
    let time_base = start_base.elapsed().as_secs_f64();

    // Weights = exact inverse of the diagonal of A.
    let mut weights = vec![0.0; n];
    for i in 0..n {
        weights[i] = if i % 2 == 0 { 1e-6 } else { 1.0 };
    }

    // α = 0.5: genuinely fractional scaling (w^0.5), reported for comparison.
    let mut fogno_half = FoGNO::new(0.5, n);
    fogno_half.set_weights(weights.clone());
    let mut x_half = vec![0.0; n];
    let status_half = gmres_preconditioned(matvec, &b, &mut x_half, &cfg, identity, |v, out| {
        fogno_half.apply(v, out)
    });

    // α = 1.0: exact inverse diagonal → preconditioned operator is the identity.
    let mut fogno_exact = FoGNO::new(1.0, n);
    fogno_exact.set_weights(weights);
    let mut x_exact = vec![0.0; n];
    let start_exact = Instant::now();
    let status_exact = gmres_preconditioned(matvec, &b, &mut x_exact, &cfg, identity, |v, out| {
        fogno_exact.apply(v, out)
    });
    let time_exact = start_exact.elapsed().as_secs_f64();

    println!(" GMRES iterations (measured):");
    println!(
        "   no preconditioner:                 {}  ({:.6}s)",
        iters_or_note(&status_base),
        time_base
    );
    println!(
        "   diagonal scaling, α = 0.5:         {}",
        iters_or_note(&status_half)
    );
    println!(
        "   exact inverse diagonal, α = 1.0:   {}  ({:.6}s)",
        iters_or_note(&status_exact),
        time_exact
    );
    println!();
    println!(" Interpretation: with α = 1 the preconditioner is the exact inverse of a");
    println!(" diagonal A, so a single iteration is expected by construction. This is");
    println!(" not evidence for any anisotropic-MHD or FLAGNO/FoGNO claim.");
}
