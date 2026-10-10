//! Independent-scheme reproduction of the reference run of Kwon & Shin (Zenodo 10.5281/zenodo.20068724): flow past a
//! penetrable Gaussian obstacle with absorbing layers, started from the reference's own `psi_time_0.0.npy`.
//!
//!     cargo run --release -p qf-pgpe --example kwon_shin -- --ref-dir DIR [--t-end 10] [--dt 0.01] [--ramp step-end|stage-times]
//!         [--snap-dir OUT --snap-every 5]
//!
//! `DIR` holds the reference case files (`psi_time_{0.0,10.0,...}.npy`, `force_dt=0.02.txt`; extract them from the Zenodo zip).
//! Prints, every `0.1 tau`, the force on the obstacle against the reference, and at `t = 10, 20, ...` the relative L2
//! distance of the wave function to the reference snapshot (complex64 in the reference); writes `kwon_shin_force.csv`.
//! With `--snap-dir OUT` it also writes `psi_time_<t>.npy` (complex128) every `--snap-every` tau (default 5), so that the
//! vortex counts can be compared with the reference's `vortex.txt` (the counting is done in Python on the snapshots).
use num_complex::Complex64;
use qf_pgpe::flow::{FlowParams, FlowSolver, FlowWorkspace, Ramp};
use qf_pgpe::npy::read_complex;
use std::path::PathBuf;
use std::time::Instant;

fn arg(a: &[String], key: &str) -> Option<String> {
    a.iter()
        .position(|s| s == key)
        .and_then(|i| a.get(i + 1).cloned())
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = PathBuf::from(arg(&a, "--ref-dir").expect("--ref-dir DIR"));
    let t_end: f64 = arg(&a, "--t-end").map_or(10.0, |s| s.parse().unwrap());
    let dt: f64 = arg(&a, "--dt").map_or(0.01, |s| s.parse().unwrap());
    let ramp = match arg(&a, "--ramp").as_deref() {
        Some("stage-times") => Ramp::StageTimes,
        _ => Ramp::StepEnd,
    };
    let snap_dir = arg(&a, "--snap-dir").map(PathBuf::from);
    let snap_every: f64 = arg(&a, "--snap-every").map_or(5.0, |s| s.parse().unwrap());
    let (nx, ny, rx, ry) = (1000usize, 500usize, 250.0, 125.0);
    let solver = FlowSolver::new(nx, ny, rx, ry, FlowParams::default());
    let (shape, mut psi) = read_complex(&dir.join("psi_time_0.0.npy")).expect("psi_time_0.0.npy");
    assert_eq!(shape, vec![ny, nx]);
    let reference: Vec<(f64, f64)> = std::fs::read_to_string(dir.join("force_dt=0.02.txt"))
        .map(|t| {
            t.lines()
                .filter_map(|l| {
                    let v: Vec<f64> = l
                        .split_whitespace()
                        .filter_map(|x| x.parse().ok())
                        .collect();
                    (v.len() >= 2).then(|| (v[0], v[1]))
                })
                .collect()
        })
        .unwrap_or_default();
    let ref_at = |t: f64| {
        reference
            .iter()
            .find(|(tt, _)| (tt - t).abs() < 1e-6)
            .map(|x| x.1)
    };
    let mut ws = FlowWorkspace::new(solver.grid.len());
    let n = (t_end / dt).round() as usize;
    let every = (0.1 / dt).round() as usize;
    let t0 = Instant::now();
    let mut csv = String::from("t,force_x,force_ref,diff\n");
    let (mut max_diff, mut max_force) = (0.0f64, 0.0f64);
    for i in 0..=n {
        let t = i as f64 * dt;
        if i % every == 0 {
            let f = solver.force_x(&psi);
            let fr = ref_at((t * 1e3).round() / 1e3);
            if let Some(r) = fr {
                max_diff = max_diff.max((f - r).abs());
                max_force = max_force.max(r.abs());
            }
            csv += &format!(
                "{t:.3},{f:.12},{},{}\n",
                fr.map_or("".into(), |r| format!("{r:.12}")),
                fr.map_or("".into(), |r| format!("{:.3e}", f - r))
            );
        }
        let ts = (t / 10.0).round() * 10.0;
        if t > 0.0
            && (t - ts).abs() < 1e-9
            && let Ok((_, snap)) = read_complex(&dir.join(format!("psi_time_{ts:.1}.npy")))
        {
            let (num, den): (f64, f64) = psi.iter().zip(&snap).fold(
                (0.0, 0.0),
                |(n, d), (p, s): (&Complex64, &Complex64)| {
                    (n + (p - s).norm_sqr(), d + s.norm_sqr())
                },
            );
            println!(
                "t = {t:6.1}: relative L2 distance of psi to the reference snapshot {:.3e}",
                (num / den).sqrt()
            );
        }
        if let Some(d) = &snap_dir {
            let k = (t / snap_every).round();
            if t > 0.0 && (t - k * snap_every).abs() < 1e-9 {
                std::fs::create_dir_all(d).unwrap();
                qf_pgpe::npy::write_complex(
                    &d.join(format!("psi_time_{t:.1}.npy")),
                    &[ny, nx],
                    &psi,
                )
                .unwrap();
            }
        }
        if i < n {
            solver.step(&mut psi, t, dt, ramp, &mut ws);
        }
        if i % 100 == 0 {
            eprintln!(
                "step {i}/{n} t = {t:.2} ({:.0} s)",
                t0.elapsed().as_secs_f64()
            );
        }
    }
    println!(
        "force_x: max |ours - reference| = {max_diff:.3e}, relative to max |F| = {:.2e}; wall {:.0} s ({} steps)",
        max_diff / max_force.max(1e-300),
        t0.elapsed().as_secs_f64(),
        n
    );
    std::fs::write("kwon_shin_force.csv", csv).unwrap();
}
