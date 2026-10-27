//! Appendix F, example 9: the two invariants of the sigma = 3 shell model, with CVODE (BDF), and
//! the kernel-checked enstrophy bound `2 D Omega(t) <= H_3(0) + S(0)^{3/2}` along a trajectory.
//!
//! Positive control: for the exact model, `S = sum |v_n|^2` and `H_3` drift only at the level of
//! the solver tolerance, and the bound ratio stays <= 1. Negative control: one coupling multiplied
//! by `1 + 1e-2` breaks the cancellation, and the drift becomes visible.
//!
//! Usage: `tb_shell_sigma3 [OUT_DIR]` (writes `shell_sigma3_trace.csv`).

use num_complex::Complex64;
use qf_shell_cascade::sigma::{CvodeSettings, SigmaModel, integrate_cvode, mass};
use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&out).expect("create OUT_DIR");
    let n_max = 5;
    // |v_n| = 2^{-1.5 n} with deterministic phases n * 0.7, normalised to S = 1.
    let v: Vec<Complex64> = (0..=n_max)
        .map(|n| Complex64::from_polar(2f64.powf(-1.5 * n as f64), 0.7 * n as f64))
        .collect();
    let s = mass(&v).sqrt();
    let v0: Vec<Complex64> = v.iter().map(|z| z / s).collect();
    let set = CvodeSettings {
        rtol: 1e-10,
        atol: 1e-13,
        max_steps: 20_000_000,
        analytic_jacobian: true,
        trace_every: 20,
    };
    let exact = SigmaModel {
        n_max,
        d: 0.05,
        sigma: 3.0,
        perturb: None,
    };
    let broken = SigmaModel {
        perturb: Some((1, 1e-2)),
        ..exact
    };
    let mut ok = true;
    for (name, model) in [("exact", exact), ("perturbed_coupling", broken)] {
        let run = integrate_cvode(model, &v0, 2.0, set).expect("CVODE run");
        println!(
            "{name:<19} T = 2: steps = {:>6}  max|dS|/S = {:.2e}  max|dH|/scale = {:.2e}  sup 2D Omega / (H(0)+S(0)^1.5) = {:.4}",
            run.steps, run.max_rel_ds, run.max_rel_dh, run.sup_ratio
        );
        if name == "exact" {
            ok &= run.max_rel_ds < 1e-7 && run.max_rel_dh < 1e-7 && run.sup_ratio <= 1.0;
            let mut csv = String::from("t,S,H,Omega\n");
            for (t, s, h, om) in &run.trace {
                let _ = writeln!(csv, "{t},{s:.17e},{h:.17e},{om:.17e}");
            }
            println!(
                "  H(0) = {:.12}, S(0) = {:.12}, bound H(0) + S(0)^1.5 = {:.12}",
                run.h0, run.s0, run.bound_rhs
            );
            std::fs::write(out.join("shell_sigma3_trace.csv"), csv).expect("write trace");
        } else {
            ok &= run.max_rel_ds > 1e-5;
        }
    }
    println!("controls as expected: {ok}");
    std::process::exit(if ok { 0 } else { 1 });
}
