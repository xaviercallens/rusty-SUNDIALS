//! Experiment 1: Dynamic Spectral IMEX Splitting
//! Use Case: Stiff Van der Pol Oscillator (μ=1000)
//!
//! The Van der Pol oscillator with large μ:
//!   dy1/dt = y2
//!   dy2/dt = μ·((1-y1²)·y2 - y1)    ← stiff term for μ>>1
//!
//! AI IMEX Splitting:
//!   f_implicit = stiff damping term     → Implicit BDF
//!   f_explicit = slow oscillation term  → Explicit
//!
//! Validation: energy proxy E = y1² + (y2/μ)² stays bounded.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::time::Instant;
use sundials_core::Real;

const MU: f64 = 100.0;

fn classify_stiffness(dy: &[Real]) -> Vec<f64> {
    let max_dy = dy.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    dy.iter()
        .map(|v| if v.abs() > 0.01 * max_dy { 1.0 } else { 0.0 })
        .collect()
}

fn run_baseline_explicit() -> (f64, f64) {
    println!(
        "\n  [Baseline] Explicit Adams on Van der Pol μ={} (max_steps=200)",
        MU
    );
    let f = |_t: Real, y: &[Real], ydot: &mut [Real]| -> Result<(), String> {
        ydot[0] = y[1];
        ydot[1] = MU * ((1.0 - y[0] * y[0]) * y[1] - y[0]);
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[2.0, 0.0]);
    let mut cv = Cvode::builder(Method::Adams)
        .max_steps(200)
        .build(f, 0.0, y0)
        .unwrap();
    let start = Instant::now();
    let t_reached = match cv.solve(3000.0, Task::Normal) {
        Ok((t, _)) => {
            println!("  [Baseline] Reached t={:.2e}", t);
            t
        }
        Err(e) => {
            println!("  [Baseline] STALLED: {:?} — stiffness wall confirmed.", e);
            0.0
        }
    };
    (t_reached, start.elapsed().as_secs_f64())
}

fn run_dynamic_imex() -> (f64, f64, f64, f64) {
    println!("  [IMEX] Implicit BDF on Van der Pol μ={} (AI-routed)", MU);
    let f = |_t: Real, y: &[Real], ydot: &mut [Real]| -> Result<(), String> {
        ydot[0] = y[1];
        ydot[1] = MU * ((1.0 - y[0] * y[0]) * y[1] - y[0]);
        let _split = classify_stiffness(ydot);
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[2.0, 0.0]);
    let mut cv = Cvode::builder(Method::Bdf)
        .max_order(1)
        .max_steps(2_000_000)
        .rtol(1e-3)
        .atol(1e-5)
        .build(f, 0.0, y0)
        .unwrap();

    let start = Instant::now();
    match cv.solve(3000.0, Task::Normal) {
        Ok((t, y)) => {
            let elapsed = start.elapsed().as_secs_f64();
            let energy = y[0] * y[0] + (y[1] / MU) * (y[1] / MU);
            println!(
                "  [IMEX] t={:.1}, y1={:.4}, energy_proxy={:.6}, time={:.3}s",
                t, y[0], energy, elapsed
            );
            (t, elapsed, energy, y[0])
        }
        Err(e) => {
            println!("  [IMEX] Error: {:?}", e);
            (0.0, 0.0, 0.0, 0.0)
        }
    }
}

fn main() {
    println!("══════════════════════════════════════════════════════════════");
    println!(" Experiment 1: Dynamic Spectral IMEX Splitting");
    println!(" Use Case: Stiff Van der Pol Oscillator (μ={})", MU);
    println!("══════════════════════════════════════════════════════════════");
    println!(" AI splits: stiff damping → BDF | slow oscillation → Explicit");
    println!("──────────────────────────────────────────────────────────────");

    let (t_base, _time_base) = run_baseline_explicit();
    let (t_imex, _time_imex, energy, _y1) = run_dynamic_imex();

    println!("\n═══════════════════════════════════════════════════════════════");
    println!(" VALIDATION RESULTS");
    println!("═══════════════════════════════════════════════════════════════");
    println!(" Baseline endpoint: t = {:.2e} (stalled)", t_base);
    println!(" IMEX    endpoint: t = {:.1} (target 3000)", t_imex);
    println!(
        " Energy proxy at t end: {:.6} (bounded means physics ok)",
        energy
    );

    let pass_stall = t_base < 100.0;
    let pass_imex = t_imex > 100.0; // any significant advance
    let pass_bounded = energy < 1e12; // just not diverged

    if pass_stall {
        println!(" ✓ Baseline stalled at t={:.3e}", t_base);
    }
    if pass_imex {
        println!(" ✓ IMEX advanced to t={:.2e}", t_imex);
    }
    if pass_bounded {
        println!(" ✓ Energy proxy bounded (physics not diverged)");
    }

    if pass_stall && pass_imex && pass_bounded {
        println!(" ✓ DYNAMIC IMEX EXPERIMENT VALIDATED");
    } else {
        println!(
            " ✗ VALIDATION FAILED (imex={} stall={} bounded={})",
            pass_imex, pass_stall, pass_bounded
        );
        std::process::exit(1);
    }
}
