//! Ground-state preparation of the reference run of Kwon & Shin (Zenodo 10.5281/zenodo.20068724), with the reference's own
//! parameters (dtau = 0.04, eps = Nx Ny 1e-9, at most 25000 steps, v_init = 0): see [`qf_pgpe::flow::FlowSolver::ground_state`].
//!
//!     cargo run --release -p qf-pgpe --example kwon_shin_ground -- --out psi0.npy [--eps-factor 1e-9] [--dtau 0.04] [--max-steps 25000]
//!
//! Writes the field WITHOUT the reference's seeded noise (numpy's `default_rng(2026)` stream is not reproduced in Rust); the
//! comparison with the stored `psi_time_0.0.npy` adds that noise in Python (`exploration/external/kwon_shin_ground_state.py --rust`).
use qf_pgpe::flow::{FlowParams, FlowSolver};
use qf_pgpe::npy::write_complex;
use std::path::PathBuf;
use std::time::Instant;

fn arg(a: &[String], key: &str) -> Option<String> {
    a.iter()
        .position(|s| s == key)
        .and_then(|i| a.get(i + 1).cloned())
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(arg(&a, "--out").expect("--out FILE.npy"));
    let dtau: f64 = arg(&a, "--dtau").map_or(0.04, |s| s.parse().unwrap());
    let factor: f64 = arg(&a, "--eps-factor").map_or(1e-9, |s| s.parse().unwrap());
    let max_steps: usize = arg(&a, "--max-steps").map_or(25000, |s| s.parse().unwrap());
    let (nx, ny) = (1000usize, 500usize);
    let solver = FlowSolver::new(nx, ny, 250.0, 125.0, FlowParams::default());
    let t0 = Instant::now();
    let gs = solver.ground_state(0.0, dtau, (nx * ny) as f64 * factor, max_steps);
    println!(
        "steps {} (tau = {:.2}), gamma = {:.3e}, converged = {}, {:.1} s",
        gs.steps,
        gs.steps as f64 * dtau,
        gs.gamma,
        gs.converged,
        t0.elapsed().as_secs_f64()
    );
    write_complex(&out, &[ny, nx], &gs.psi).unwrap();
}
