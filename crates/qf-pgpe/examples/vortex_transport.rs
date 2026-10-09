//! Vortex-transport instrument of SocrateAI-Scientific-QuantumFluids (`exploration/pgpe/vortex_transport.py`,
//! amendment A1/A1.1 of `docs/designs/PGPE_FRICTION_PREREG.md`), ported to the pure-Rust `ComplexField2D`:
//!
//! 1. **Periodic vortex imprint** (`imprint_v2`): the Jacobi theta-function phase of each vortex (torus,
//!    `tau = i`), with the phase jump the product acquires across each box boundary measured and removed by a
//!    uniform phase gradient (so a configuration of ANY dipole moment is single-valued on the torus), and the
//!    quasi-static Bernoulli amplitude `[1/(1 + |v|^2/2)]^{1/2}` (zero at the cores, `1 - v^2/2` far away).
//!    The earlier imprint (`round2.imprint`) was not periodic for a single pair and had a `1/r^2` amplitude tail
//!    whose sound moved vortices at `T = 0`; both defects were found by the `T = 0` control of the campaign.
//! 2. **Raw phase-winding detection with sub-grid refinement**: winding plaquettes (`VortexWinding.lean`) and the
//!    zero of the least-squares plane through `psi` at the plaquette's four corners.
//! 3. **Tracking** of the imprinted vortices by continuity (same sign, nearest detection within `r_track`), and
//!    the field momentum (total, and in the band `|k| > 1` -- the phonon wind of hypothesis W).
//!
//! Output: a CSV `t, x_i, y_i (i = 1..n_v), n_detected, Px, Py, Px_hi, Py_hi` that the Python estimators
//! (`transport_estimators.py`: energy estimator of the friction `alpha`, two-coefficient regression for
//! `1 - alpha'` and `alpha`, residual MSD for the diffusion `eta`) consume unchanged.
//!
//!     cargo run --release -p qf-pgpe --example vortex_transport -- validate
//!     cargo run --release -p qf-pgpe --example vortex_transport -- BASE.raw|T0 OUT.csv [--geom antiparallel|dipole]
//!         [--d0 10] [--t-max 400] [--n 128] [--l 64] [--seed 1] [--r-track 3.0] [--d-stop 1.5]
//!
//! `validate` reproduces the gate G1 known answers of the campaign at `T = 0` (uniform condensate, single pair
//! `d0 = 10`, 100 time units): separation constant, field momentum constant and within 5 % of `2 pi n d`, vortex
//! count exactly 2. The Python cross-check (`PYTHON_CROSSCHECK.md`) compares the imprinted field and the first
//! detected positions bit-for-bit in intent (same formulas), to `1e-10` on the positions.
use num_complex::Complex64;
use qf_pgpe::ComplexField2D;
use qf_pgpe::vortex::{
    Ended, Lcg, TrackConfig, antiparallel, csv, detect, dipole, imprint_v2, momentum_band,
    torus_distance, track_run, uniform_condensate,
};
use std::f64::consts::PI;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

fn read_complex_raw(path: &Path, n2: usize) -> std::io::Result<Vec<Complex64>> {
    let mut buf = Vec::new();
    fs::File::open(path)?.read_to_end(&mut buf)?;
    assert_eq!(
        buf.len(),
        16 * n2,
        "raw field size does not match n*n complex128"
    );
    Ok((0..n2)
        .map(|k| {
            let re = f64::from_le_bytes(buf[16 * k..16 * k + 8].try_into().unwrap());
            let im = f64::from_le_bytes(buf[16 * k + 8..16 * k + 16].try_into().unwrap());
            Complex64::new(re, im)
        })
        .collect())
}

struct Args {
    base: String,
    out: String,
    geom: String,
    d0: f64,
    t_max: f64,
    n: usize,
    l: f64,
    seed: u64,
    r_track: f64,
    d_stop: f64,
}

fn parse() -> Args {
    let a: Vec<String> = std::env::args().collect();
    let mut r = Args {
        base: a.get(1).cloned().unwrap_or_default(),
        out: a.get(2).cloned().unwrap_or_default(),
        geom: "antiparallel".into(),
        d0: 10.0,
        t_max: 400.0,
        n: 128,
        l: 64.0,
        seed: 1,
        r_track: 3.0,
        d_stop: 1.5,
    };
    let mut i = 3;
    while i + 1 < a.len() {
        match a[i].as_str() {
            "--geom" => r.geom = a[i + 1].clone(),
            "--d0" => r.d0 = a[i + 1].parse().unwrap(),
            "--t-max" => r.t_max = a[i + 1].parse().unwrap(),
            "--n" => r.n = a[i + 1].parse().unwrap(),
            "--l" => r.l = a[i + 1].parse().unwrap(),
            "--seed" => r.seed = a[i + 1].parse().unwrap(),
            "--r-track" => r.r_track = a[i + 1].parse().unwrap(),
            "--d-stop" => r.d_stop = a[i + 1].parse().unwrap(),
            other => panic!("unknown option {other}"),
        }
        i += 2;
    }
    r
}

fn validate() {
    // Gate G1 known answers (campaign, corrected imprint): single pair d0 = 10 at T = 0, 100 time units.
    let f = ComplexField2D::new(128, 64.0, 1.0, 0.01);
    let c0 = uniform_condensate(&f);
    let pos = vec![(30.13 + 5.0, 20.31), (30.13 - 5.0, 20.31)];
    let q = vec![1, -1];
    let c = imprint_v2(&f, &c0, &pos, &q);
    let det = detect(&f, &c);
    let (px, py) = momentum_band(&f, &c, 0.0);
    let ratio = (px * px + py * py).sqrt() / (2.0 * PI * 10.0);
    println!(
        "detected after imprint: {} (expected 2); |P| / (2 pi n d) = {ratio:.4} (campaign: 0.96-0.99)",
        det.len()
    );
    let run = track_run(&f, c, &pos, &q, &TrackConfig::new(100.0));
    let sep = |i: usize| torus_distance(run.samples[i].pos[0], run.samples[i].pos[1], 64.0);
    let pm = |i: usize| run.samples[i].p.0.hypot(run.samples[i].p.1);
    let last = run.samples.len() - 1;
    let (d_first, d_last, p_first, p_last) = (sep(10), sep(last), pm(0), pm(last));
    let ok = det.len() == 2
        && (0.95..=1.0).contains(&ratio)
        && (d_last - d_first).abs() < 0.1
        && ((p_last - p_first) / p_first).abs() < 1e-6
        && run.ended == Ended::TMax;
    println!(
        "run: {}; separation t=10 -> t=100: {d_first:.3} -> {d_last:.3} (G1: 9.815 -> 9.821 over 400); |P| {p_first:.4} -> {p_last:.4}",
        run.ended.name()
    );
    println!("VALIDATE: {}", if ok { "PASS" } else { "FAIL" });
    std::process::exit(if ok { 0 } else { 1 });
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("validate") {
        validate();
        return;
    }
    let a = parse();
    let f = ComplexField2D::new(a.n, a.l, 1.0, 0.01);
    let c0 = if a.base == "T0" {
        uniform_condensate(&f)
    } else {
        read_complex_raw(Path::new(&a.base), a.n * a.n).expect("read base field")
    };
    let mut rng = Lcg::new(a.seed);
    let (pos, q) = if a.geom == "dipole" {
        dipole(a.l, a.d0, &mut rng)
    } else {
        antiparallel(a.l, a.d0, &mut rng)
    };
    let run = track_run(
        &f,
        imprint_v2(&f, &c0, &pos, &q),
        &pos,
        &q,
        &TrackConfig {
            t_max: a.t_max,
            dt_sample: 1.0,
            r_track: a.r_track,
            d_stop: a.d_stop,
        },
    );
    fs::File::create(&a.out)
        .expect("create output")
        .write_all(csv(&run, q.len()).as_bytes())
        .expect("write output");
    // sidecar: the imprint positions and charges, for exact cross-checks against the Python instrument
    let meta: String = pos
        .iter()
        .zip(&q)
        .map(|(p, qi)| format!("{:.12} {:.12} {qi}\n", p.0, p.1))
        .collect();
    fs::write(format!("{}.pos0", a.out), meta).expect("write pos0");
    println!(
        "{}: {} t_end={}",
        a.out,
        run.ended.name(),
        run.samples.last().map_or(0.0, |s| s.t)
    );
}
