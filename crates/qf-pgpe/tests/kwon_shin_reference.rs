//! Reproduction of the reference run of Kwon & Shin (Phys. Rev. Research 2026; Zenodo 10.5281/zenodo.20068724, CC-BY-4.0).
//!
//! Needs the reference files (18.7 MB zip); the test is skipped unless `QF_KWON_SHIN_DIR` points to a directory holding
//! `psi_time_0.0.npy` and `force_dt=0.02.txt` (`tests/fetch_kwon_shin.sh DIR` downloads, verifies and extracts them).
//! Run in release mode: `QF_KWON_SHIN_DIR=DIR cargo test --release -p qf-pgpe --test kwon_shin_reference`.
//!
//! What is asserted (the numbers are the ones measured and documented in `PGPE_EXTERNAL_REPRODUCTION.md`):
//! 1. the ground-state preparation reproduces the stored initial field up to the reference's seeded noise: the difference
//!    is, entry by entry, an integer multiple of 1e-4 in [-5, 4] (real and imaginary part), to float32 rounding (the noise
//!    stream of numpy is not reproduced here; the 2.8e-8 agreement with the noise added is checked in Python);
//! 2. the force on the obstacle for t <= 0.3 (30 RK4 steps from the stored field, the reference's step-end ramp
//!    convention) agrees with the stored force to 5e-6 (measured 1.1e-6).
use num_complex::Complex64;
use qf_pgpe::flow::{FlowParams, FlowSolver, FlowWorkspace, Ramp};
use qf_pgpe::npy::read_complex;
use std::path::PathBuf;

fn reference_dir() -> Option<PathBuf> {
    match std::env::var("QF_KWON_SHIN_DIR") {
        Ok(d) => Some(PathBuf::from(d)),
        Err(_) => {
            eprintln!("QF_KWON_SHIN_DIR not set: reference reproduction skipped");
            None
        }
    }
}

fn solver() -> FlowSolver {
    FlowSolver::new(1000, 500, 250.0, 125.0, FlowParams::default())
}

#[test]
fn ground_state_reproduces_the_stored_initial_field_up_to_the_seeded_noise() {
    let Some(dir) = reference_dir() else { return };
    let s = solver();
    let gs = s.ground_state(0.0, 0.04, 1000.0 * 500.0 * 1e-9, 25000);
    assert!(
        gs.converged && gs.steps == 1,
        "the reference's tolerance is met at the first step; got {} steps (converged {})",
        gs.steps,
        gs.converged
    );
    let (shape, reference) = read_complex(&dir.join("psi_time_0.0.npy")).unwrap();
    assert_eq!(shape, vec![500, 1000]);
    let mut worst = 0.0f64;
    for (a, b) in reference.iter().zip(&gs.psi) {
        for d in [(a - b).re, (a - b).im] {
            let units = d / 1e-4;
            assert!(
                (-5.0 - 5e-3..=4.0 + 5e-3).contains(&units),
                "noise outside [-5, 4] x 1e-4: {d:e}"
            );
            worst = worst.max((units - units.round()).abs());
        }
    }
    assert!(worst < 5e-3, "noise is not on the 1e-4 lattice: {worst:e}");
}

#[test]
fn force_matches_the_stored_force_for_the_first_three_tenths_of_tau() {
    let Some(dir) = reference_dir() else { return };
    let s = solver();
    let (_, mut psi): (_, Vec<Complex64>) = read_complex(&dir.join("psi_time_0.0.npy")).unwrap();
    let table = std::fs::read_to_string(dir.join("force_dt=0.02.txt")).unwrap();
    let reference: Vec<(f64, f64)> = table
        .lines()
        .filter_map(|l| {
            let v: Vec<f64> = l
                .split_whitespace()
                .filter_map(|x| x.parse().ok())
                .collect();
            (v.len() >= 2).then(|| (v[0], v[1]))
        })
        .collect();
    let mut ws = FlowWorkspace::new(s.grid.len());
    let dt = 0.01;
    let mut worst = 0.0f64;
    for i in 0..=30 {
        let t = i as f64 * dt;
        if i % 10 == 0 {
            let f = s.force_x(&psi);
            let (_, fr) = reference
                .iter()
                .find(|(tt, _)| (tt - t).abs() < 1e-6)
                .expect("reference force at this time");
            worst = worst.max((f - fr).abs());
        }
        if i < 30 {
            s.step(&mut psi, t, dt, Ramp::StepEnd, &mut ws);
        }
    }
    assert!(worst < 5e-6, "max |F - F_ref| = {worst:e}");
}
