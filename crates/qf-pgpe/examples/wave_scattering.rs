//! One vortex-phonon scattering run (`exploration/pgpe/vortex_wave_scattering.py` ported to the pure-Rust PGPE):
//! a vortex-antivortex pair at separation `d0` along `x` in the uniform `T = 0` condensate, dressed with a travelling
//! Bogoliubov wave of mode `m` along `+-y`; writes `OUT.csv` (`t, x0, y0, x1, y1, Px, Py, E`) and `OUT.json`
//! (`k, omega, eps, dir, L, N, d0, ended, t_end, j, amp_rho, drift_E, seconds`).
//!
//!     cargo run --release -p qf-pgpe --example wave_scattering -- OUT --m 4 --av 0.04 --dir 1 --t-max 400
//!         [--eps 0.05] [--L 64] [--N 128] [--d0 32] [--dt-sample 1] [--r-track 3] [--no-pair]
//!
//! `OUT` is a path prefix (`OUT.csv`, `OUT.json`). With `--av > 0`, `eps` is set so that the peak velocity
//! amplitude `k eps (u + v)` equals `av`.
use qf_pgpe::ComplexField2D;
use qf_pgpe::scattering::{WaveParams, eps_from_av, run_wave_pair, wavenumber};
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = a
        .get(1)
        .cloned()
        .expect("usage: wave_scattering OUT [options]");
    let (mut m, mut eps, mut av, mut dir) = (4usize, 0.05, 0.0, 1i32);
    let (mut t_max, mut l, mut n, mut d0) = (600.0, 64.0, 128usize, 32.0);
    let (mut dt_sample, mut r_track, mut no_pair) = (1.0, 3.0, false);
    let mut i = 2;
    while i < a.len() {
        let val = |i: usize| {
            a.get(i + 1)
                .unwrap_or_else(|| panic!("missing value for {}", a[i]))
        };
        match a[i].as_str() {
            "--m" => m = val(i).parse().unwrap(),
            "--eps" => eps = val(i).parse().unwrap(),
            "--av" => av = val(i).parse().unwrap(),
            "--dir" => dir = val(i).parse().unwrap(),
            "--t-max" => t_max = val(i).parse().unwrap(),
            "--L" => l = val(i).parse().unwrap(),
            "--N" => n = val(i).parse().unwrap(),
            "--d0" => d0 = val(i).parse().unwrap(),
            "--dt-sample" => dt_sample = val(i).parse().unwrap(),
            "--r-track" => r_track = val(i).parse().unwrap(),
            "--no-pair" => {
                no_pair = true;
                i += 1;
                continue;
            }
            other => panic!("unknown option {other}"),
        }
        i += 2;
    }
    assert!(dir == 1 || dir == -1, "--dir must be +1 or -1");
    if av > 0.0 {
        eps = eps_from_av(wavenumber(l, m), av);
    }
    let t0 = Instant::now();
    let f = ComplexField2D::new(n, l, 1.0, 0.01);
    let mut p = WaveParams::new(m, eps, dir, t_max, d0);
    p.dt_sample = dt_sample;
    p.r_track = r_track;
    p.no_pair = no_pair;
    let run = run_wave_pair(&f, &p);
    let secs = (t0.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    std::fs::write(format!("{out}.csv"), run.csv()).expect("write csv");
    std::fs::write(format!("{out}.json"), run.meta_json(secs)).expect("write json");
    println!(
        "{out} k={:.6} eps={:.6} dir={} j={:.9e} amp_rho={:.6} ended={} t_end={} drift_E={:.3e} seconds={secs}",
        run.k,
        run.eps,
        run.dir,
        run.j,
        run.amp_rho,
        run.ended.name(),
        run.t_end,
        run.drift_e
    );
}
