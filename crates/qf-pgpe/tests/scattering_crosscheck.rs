//! Cross-check of the Rust scattering instrument against the Python original
//! (`exploration/pgpe/vortex_wave_scattering.py`, `analyze_wave_scan.slopes`).
//!
//! Fixtures `tests/fixtures/scattering_{pp,pm,p0}.{csv,meta}` were produced by the Python instrument for
//! `N = 32, L = 16, d0 = 7.3, m = 2, t_max = 60` with `eps = 0.05, +y` (`pp`), `eps = 0.05, -y` (`pm`) and
//! `eps = 0` (`p0`): tracked positions, field momentum, energy, the momentum flux `j` of the vortex-free wave and the
//! drift slopes (`np.unwrap` on the torus, `np.polyfit` degree 1 on `t >= 50`). The full-size registered probe is an
//! `#[ignore]` test (`cargo test --release -p qf-pgpe -- --ignored`), needing the Python probe files.
use qf_pgpe::ComplexField2D;
use qf_pgpe::scattering::{
    WaveParams, WaveRun, dress, drift_slopes, drift_slopes_min, run_wave_pair, sigma_from_pair,
    wave_flux,
};
use qf_pgpe::vortex::{Ended, momentum_band, uniform_condensate};
use std::collections::HashMap;

fn fixture(name: &str) -> (Vec<Vec<f64>>, HashMap<String, f64>) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
    let csv = std::fs::read_to_string(format!("{dir}scattering_{name}.csv")).unwrap();
    let rows = csv
        .lines()
        .skip(1)
        .map(|l| l.split(',').map(|x| x.parse().unwrap()).collect())
        .collect();
    let meta = std::fs::read_to_string(format!("{dir}scattering_{name}.meta")).unwrap();
    let kv = meta
        .lines()
        .map(|l| {
            let (k, v) = l.split_once('=').unwrap();
            (k.to_string(), v.parse().unwrap())
        })
        .collect();
    (rows, kv)
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs().max(1e-300)
}

fn check_case(name: &str) -> (WaveRun, HashMap<String, f64>) {
    let (rows, kv) = fixture(name);
    let n = kv["N"] as usize;
    let f = ComplexField2D::new(n, kv["L"], 1.0, 0.01);
    let (m, eps, dir) = (kv["m"] as usize, kv["eps"], kv["dir"] as i32);

    // the wave alone on the vortex-free condensate: dressing identical (momentum), j to 1e-12 relative
    let (j, pw, amp, cw) = wave_flux(&f, m, eps, dir);
    if eps > 0.0 {
        assert!(rel(j, kv["j"]) < 1e-12, "{name}: j {j} vs {}", kv["j"]);
        assert!(
            rel(pw.0.hypot(pw.1), kv["pwave_x"].hypot(kv["pwave_y"])) < 1e-12,
            "{name}: |P_wave|"
        );
        assert!(
            (pw.0 - kv["pwave_x"]).abs() < 1e-12 && (pw.1 - kv["pwave_y"]).abs() < 1e-12,
            "{name}: P_wave"
        );
        assert!(rel(amp, kv["amp_rho"]) < 1e-12, "{name}: amp_rho");
    } else {
        assert_eq!(j, 0.0);
    }
    let pc = momentum_band(&f, &cw, 0.0);
    assert!((pc.0 - pw.0).abs() < 1e-15 && (pc.1 - pw.1).abs() < 1e-15);
    // dress() norm restored
    let c0 = uniform_condensate(&f);
    let (c1, k, _) = dress(&f, &c0, m, eps, dir);
    assert!(rel(f.norm(&c1), f.norm(&c0)) < 1e-13);
    assert!(rel(k, kv["k"]) < 1e-14);

    let mut p = WaveParams::new(m, eps, dir, kv["t_max"], kv["d0"]);
    p.r_track = 3.0;
    let run = run_wave_pair(&f, &p);
    assert_eq!(run.ended, Ended::TMax, "{name}");
    assert_eq!(run.t.len(), rows.len(), "{name}: sample count");
    let (mut dpos, mut dp, mut de) = (0.0f64, 0.0f64, 0.0f64);
    for (i, row) in rows.iter().enumerate() {
        assert!((run.t[i] - row[0]).abs() < 1e-12);
        let r = &run.r[i];
        for (a, b) in [r[0].0, r[0].1, r[1].0, r[1].1].iter().zip(&row[1..5]) {
            // positions are periodic: compare on the torus
            let mut d = a - b;
            d -= f.l * (d / f.l).round();
            dpos = dpos.max(d.abs());
        }
        dp = dp
            .max((run.p[i].0 - row[5]).abs())
            .max((run.p[i].1 - row[6]).abs());
        de = de.max(rel(run.e[i], row[7]));
    }
    println!("{name}: max |dpos| {dpos:.2e}, max |dP| {dp:.2e}, max rel dE {de:.2e}");
    assert!(dpos < 1e-6, "{name}: positions differ by {dpos:e}");
    assert!(dp < 1e-6, "{name}: momentum differs by {dp:e}");
    assert!(de < 1e-9, "{name}: energy differs by {de:e}");
    (run, kv)
}

#[test]
fn rust_matches_python_on_the_short_case() {
    let (rp, kp) = check_case("pp");
    let (rm, km) = check_case("pm");
    let (r0, k0) = check_case("p0");
    let mut s = Vec::new();
    for (run, kv) in [(&rp, &kp), (&rm, &km), (&r0, &k0)] {
        let sl = drift_slopes_min(run, run.l, 50.0, 10).unwrap();
        println!(
            "slopes: sdx {:+.9e} (py {:+.9e}), syc {:+.9e} (py {:+.9e}), d_change {:+.9e} (py {:+.9e})",
            sl.sdx, kv["sdx"], sl.syc, kv["syc"], sl.d_change, kv["d_change"]
        );
        assert!(rel(sl.sdx, kv["sdx"]) < 1e-8, "sdx");
        assert!(rel(sl.syc, kv["syc"]) < 1e-8, "syc");
        assert!(rel(sl.d_change, kv["d_change"]) < 1e-8, "d_change");
        s.push(sl);
    }
    // the public rule needs >= 100 samples: the 61-sample fixture is refused
    assert!(drift_slopes(&rp, rp.l, 50.0).is_none());
    // sigma from the pair: reproduce the formula on the Python slopes
    let sg = sigma_from_pair(&s[0], &s[1], (s[2].sdx, s[2].syc));
    let want_sp = (kp["sdx"] - k0["sdx"]).abs() * 2.0 * std::f64::consts::PI / (2.0 * kp["j"]);
    assert!(
        rel(sg.sp, want_sp) < 1e-7,
        "sigma_par(+): {} vs {want_sp}",
        sg.sp
    );
    assert!(sg.sigma_par.is_finite() && sg.sigma_perp.is_finite());
}

/// Full-size registered probe (L = 64, N = 128, d0 = 32, m = 4, eps = 0.05, +y, 400 t.u.) against the Python probe.
/// The Python probe numbers (`probe_pp.npz` against `probe_p0.npz`: `sigma_par = 2.04`, `sigma_perp = 1.58`) are
/// hard-coded; run with `cargo test --release -p qf-pgpe -- --ignored`.
#[test]
#[ignore = "full-size run (several minutes); compares to the Python probe"]
fn registered_probe_sigma() {
    let f = ComplexField2D::new(128, 64.0, 1.0, 0.01);
    let p = WaveParams::new(4, 0.05, 1, 400.0, 32.0);
    let run = run_wave_pair(&f, &p);
    let mut p0 = p.clone();
    p0.eps = 0.0;
    let ctl = run_wave_pair(&f, &p0);
    let s = drift_slopes(&run, 64.0, 50.0).unwrap();
    let c = drift_slopes(&ctl, 64.0, 50.0).unwrap();
    let sp = (s.sdx - c.sdx).abs() * 2.0 * std::f64::consts::PI / (2.0 * s.j);
    let sq = (s.syc - c.syc).abs() * 2.0 * std::f64::consts::PI / s.j;
    println!("sigma_par {sp:.4}, sigma_perp {sq:.4} (Python probe: 2.04, 1.58)");
    assert!((sp - 2.04).abs() / 2.04 < 0.02, "sigma_par {sp}");
    assert!((sq - 1.58).abs() / 1.58 < 0.02, "sigma_perp {sq}");
}
