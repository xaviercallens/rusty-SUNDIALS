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
use std::f64::consts::PI;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

const Q_NOME: f64 = 0.043213918263772250; // e^{-pi}, nome of the square torus (tau = i)

/// Jacobi theta_1(u | tau = i) = 2 sum_{n>=0} (-1)^n q^{(n+1/2)^2} sin((2n+1) u), 10 terms as in round2.py.
fn theta1(u: Complex64) -> Complex64 {
    let mut out = Complex64::new(0.0, 0.0);
    for n in 0..10 {
        let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
        let q = Q_NOME.powf((n as f64 + 0.5).powi(2));
        out += sign * q * (u * (2 * n + 1) as f64).sin();
    }
    2.0 * out
}

fn unit(z: Complex64) -> Complex64 {
    let r = z.norm();
    if r < 1e-300 { Complex64::new(1.0, 0.0) } else { z / r }
}

/// The product of theta-function phase factors over a configuration, evaluated at the grid shifted by `(sx, sy)`.
fn phase_factor(f: &ComplexField2D, pos: &[(f64, f64)], q: &[i32], sx: f64, sy: f64) -> Vec<Complex64> {
    let n = f.n;
    let mut ph = vec![Complex64::new(1.0, 0.0); n * n];
    for i in 0..n {
        for j in 0..n {
            let z = Complex64::new(i as f64 * f.dx + sx, j as f64 * f.dx + sy);
            let mut p = Complex64::new(1.0, 0.0);
            for ((xj, yj), &qj) in pos.iter().zip(q) {
                let u = unit(theta1(PI * (z - Complex64::new(*xj, *yj)) / f.l));
                p *= if qj > 0 { u } else { u.conj() };
            }
            ph[i * n + j] = p;
        }
    }
    ph
}

/// `imprint_v2`: periodic phase for any dipole moment + Bernoulli amplitude; norm restored.
pub fn imprint_v2(f: &ComplexField2D, c: &[Complex64], pos: &[(f64, f64)], q: &[i32]) -> Vec<Complex64> {
    let n = f.n;
    let ph0 = phase_factor(f, pos, q, 0.0, 0.0);
    let phx = phase_factor(f, pos, q, f.l, 0.0);
    let phy = phase_factor(f, pos, q, 0.0, f.l);
    let mean_arg = |a: &[Complex64], b: &[Complex64]| -> f64 {
        let s: Complex64 = a.iter().zip(b).map(|(u, v)| u * v.conj()).sum();
        s.arg()
    };
    let (jx, jy) = (mean_arg(&phx, &ph0), mean_arg(&phy, &ph0));
    let mut ph = ph0;
    for i in 0..n {
        for j in 0..n {
            let (x, y) = (i as f64 * f.dx, j as f64 * f.dx);
            ph[i * n + j] *= Complex64::from_polar(1.0, -(jx * x + jy * y) / f.l);
        }
    }
    // velocity of the imprinted phase by centred differences of the phase factor
    let idx = |i: isize, j: isize| (i.rem_euclid(n as isize) as usize) * n + j.rem_euclid(n as isize) as usize;
    let psi0 = f.psi(c);
    let mut psi = vec![Complex64::new(0.0, 0.0); n * n];
    for i in 0..n as isize {
        for j in 0..n as isize {
            let p = ph[idx(i, j)];
            let vx = (ph[idx(i + 1, j)] * ph[idx(i - 1, j)].conj()).arg() / (2.0 * f.dx);
            let vy = (ph[idx(i, j + 1)] * ph[idx(i, j - 1)].conj()).arg() / (2.0 * f.dx);
            let amp = (1.0 / (1.0 + 0.5 * (vx * vx + vy * vy))).sqrt();
            psi[idx(i, j)] = psi0[idx(i, j)] * p * amp;
        }
    }
    let mut c2 = f.modes(&psi);
    let s = (f.norm(c) / f.norm(&c2)).sqrt();
    c2.iter_mut().for_each(|v| *v *= s);
    c2
}

fn pv(d: f64) -> f64 {
    (d + PI).rem_euclid(2.0 * PI) - PI
}

/// Raw plaquette vortices with sub-grid refinement: `(x, y, charge)`.
pub fn detect(f: &ComplexField2D, c: &[Complex64]) -> Vec<(f64, f64, i32)> {
    let n = f.n;
    let psi = f.psi(c);
    let at = |i: usize, j: usize| psi[(i % n) * n + (j % n)];
    let th: Vec<f64> = psi.iter().map(|z| z.arg()).collect();
    let mut out = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let t00 = th[i * n + j];
            let t10 = th[((i + 1) % n) * n + j];
            let t01 = th[i * n + (j + 1) % n];
            let t11 = th[((i + 1) % n) * n + (j + 1) % n];
            let w = pv(t10 - t00) + pv(t11 - t10) - pv(t11 - t01) - pv(t01 - t00);
            let qg = (w / (2.0 * PI)).round() as i32;
            if qg == 0 {
                continue;
            }
            let (p00, p10, p01, p11) = (at(i, j), at(i + 1, j), at(i, j + 1), at(i + 1, j + 1));
            let b = 0.5 * ((p10 - p00) + (p11 - p01));
            let cc = 0.5 * ((p01 - p00) + (p11 - p10));
            let a = 0.25 * (p00 + p10 + p01 + p11) - 0.5 * (b + cc);
            let mut det = b.re * cc.im - b.im * cc.re;
            if det.abs() < 1e-14 {
                det = 1e-14;
            }
            let u = ((-a.re * cc.im + a.im * cc.re) / det).clamp(-0.5, 1.5);
            let v = ((-b.re * a.im + b.im * a.re) / det).clamp(-0.5, 1.5);
            out.push((((i as f64 + u) * f.dx).rem_euclid(f.l), ((j as f64 + v) * f.dx).rem_euclid(f.l), qg));
        }
    }
    out
}

/// Field momentum `(Px, Py)`; with `kmin > 0`, only modes with `|k| > kmin` (the phonon band).
pub fn momentum_band(f: &ComplexField2D, c: &[Complex64], kmin: f64) -> (f64, f64) {
    let w = f.dx * f.dx / (f.n * f.n) as f64;
    let (mut px, mut py) = (0.0, 0.0);
    for (idx, v) in c.iter().enumerate() {
        if kmin > 0.0 && f.k2[idx].sqrt() <= kmin {
            continue;
        }
        let m = v.norm_sqr() * w;
        px += f.kx[idx] * m;
        py += f.ky[idx] * m;
    }
    (px, py)
}

fn mindist(a: (f64, f64), b: (f64, f64), l: f64) -> f64 {
    let (mut dx, mut dy) = (a.0 - b.0, a.1 - b.1);
    dx -= l * (dx / l).round();
    dy -= l * (dy / l).round();
    (dx * dx + dy * dy).sqrt()
}

/// Two antiparallel dipoles: zero net charge and zero net dipole moment (no net impulse shed).
fn antiparallel(l: f64, d0: f64, rng: &mut Lcg) -> (Vec<(f64, f64)>, Vec<i32>) {
    let y0 = rng.uniform() * l;
    let (ox, oy) = (rng.uniform() * 0.5, rng.uniform() * 0.5);
    let p = vec![
        ((l / 4.0 + d0 / 2.0 + ox).rem_euclid(l), (y0 + oy).rem_euclid(l)),
        ((l / 4.0 - d0 / 2.0 + ox).rem_euclid(l), (y0 + oy).rem_euclid(l)),
        ((3.0 * l / 4.0 - d0 / 2.0 + ox).rem_euclid(l), (y0 + oy).rem_euclid(l)),
        ((3.0 * l / 4.0 + d0 / 2.0 + ox).rem_euclid(l), (y0 + oy).rem_euclid(l)),
    ];
    (p, vec![1, -1, 1, -1])
}

/// Small deterministic generator for placements (documented as not numpy-identical: the campaign's claims are
/// ensemble statements, not single trajectories).
struct Lcg(u64);
impl Lcg {
    fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

fn read_complex_raw(path: &Path, n2: usize) -> std::io::Result<Vec<Complex64>> {
    let mut buf = Vec::new();
    fs::File::open(path)?.read_to_end(&mut buf)?;
    assert_eq!(buf.len(), 16 * n2, "raw field size does not match n*n complex128");
    Ok((0..n2)
        .map(|k| {
            let re = f64::from_le_bytes(buf[16 * k..16 * k + 8].try_into().unwrap());
            let im = f64::from_le_bytes(buf[16 * k + 8..16 * k + 16].try_into().unwrap());
            Complex64::new(re, im)
        })
        .collect())
}

fn uniform_condensate(f: &ComplexField2D) -> Vec<Complex64> {
    let mut c = vec![Complex64::new(0.0, 0.0); f.n * f.n];
    c[0] = Complex64::new((f.n * f.n) as f64, 0.0); // psi = 1
    c
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
    let mut r = Args { base: a.get(1).cloned().unwrap_or_default(), out: a.get(2).cloned().unwrap_or_default(), geom: "antiparallel".into(), d0: 10.0, t_max: 400.0, n: 128, l: 64.0, seed: 1, r_track: 3.0, d_stop: 1.5 };
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

/// One tracked run; returns the CSV text and the final tracked positions.
fn run(f: &ComplexField2D, c0: &[Complex64], pos0: &[(f64, f64)], q: &[i32], t_max: f64, r_track: f64, d_stop: f64) -> (String, String) {
    let mut c = imprint_v2(f, c0, pos0, q);
    let mut last: Vec<(f64, f64)> = pos0.to_vec();
    let mut csv = String::from("t");
    for i in 0..q.len() {
        csv += &format!(",x{i},y{i}");
    }
    csv += ",n_det,Px,Py,Px_hi,Py_hi\n";
    let mut t = 0.0;
    let mut ended = "t_max";
    loop {
        let det = detect(f, &c);
        let mut cur = Vec::with_capacity(q.len());
        let mut lost = false;
        for (i, &qi) in q.iter().enumerate() {
            let best = det
                .iter()
                .filter(|d| d.2 == qi)
                .map(|d| (mindist((d.0, d.1), last[i], f.l), (d.0, d.1)))
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            match best {
                Some((r, p)) if r <= r_track || (t == 0.0 && r <= 3.0) => cur.push(p),
                _ => {
                    lost = true;
                    break;
                }
            }
        }
        if lost {
            ended = "track_lost";
            break;
        }
        let (px, py) = momentum_band(f, &c, 0.0);
        let (pxh, pyh) = momentum_band(f, &c, 1.0);
        csv += &format!("{t}");
        for p in &cur {
            csv += &format!(",{:.6},{:.6}", p.0, p.1);
        }
        csv += &format!(",{},{:.6},{:.6},{:.6},{:.6}\n", det.len(), px, py, pxh, pyh);
        last = cur;
        let dmin = (0..q.len())
            .flat_map(|i| (0..q.len()).map(move |j| (i, j)))
            .filter(|&(i, j)| q[i] > 0 && q[j] < 0)
            .map(|(i, j)| mindist(last[i], last[j], f.l))
            .fold(f64::INFINITY, f64::min);
        if dmin < d_stop {
            ended = "annihilated";
            break;
        }
        if t >= t_max - 1e-9 {
            break;
        }
        c = f.run(&c, 1.0);
        t += 1.0;
    }
    (csv, format!("{ended} t_end={t}"))
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
    let p = (px * px + py * py).sqrt();
    let ratio = p / (2.0 * PI * 10.0);
    println!("detected after imprint: {} (expected 2); |P| / (2 pi n d) = {ratio:.4} (campaign: 0.96-0.99)", det.len());
    let (csv, end) = run(&f, &c0, &pos, &q, 100.0, 3.0, 1.5);
    let rows: Vec<&str> = csv.lines().skip(1).collect();
    let sep = |row: &str| -> f64 {
        let v: Vec<f64> = row.split(',').map(|s| s.parse().unwrap()).collect();
        mindist((v[1], v[2]), (v[3], v[4]), 64.0)
    };
    let (d_first, d_last) = (sep(rows[10]), sep(rows[rows.len() - 1]));
    let pmom = |row: &str| -> f64 {
        let v: Vec<f64> = row.split(',').map(|s| s.parse().unwrap()).collect();
        (v[6] * v[6] + v[7] * v[7]).sqrt()
    };
    let (p_first, p_last) = (pmom(rows[0]), pmom(rows[rows.len() - 1]));
    let ok = det.len() == 2 && (0.95..=1.0).contains(&ratio) && (d_last - d_first).abs() < 0.1 && ((p_last - p_first) / p_first).abs() < 1e-6 && end.starts_with("t_max");
    println!("run: {end}; separation t=10 -> t=100: {d_first:.3} -> {d_last:.3} (G1: 9.815 -> 9.821 over 400); |P| {p_first:.4} -> {p_last:.4}");
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
    let c0 = if a.base == "T0" { uniform_condensate(&f) } else { read_complex_raw(Path::new(&a.base), a.n * a.n).expect("read base field") };
    let mut rng = Lcg(a.seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xD1B54A32D192ED03);
    let (pos, q) = if a.geom == "dipole" {
        let (x0, y0) = (rng.uniform() * a.l, rng.uniform() * a.l);
        (vec![((x0 + a.d0 / 2.0).rem_euclid(a.l), y0), ((x0 - a.d0 / 2.0).rem_euclid(a.l), y0)], vec![1, -1])
    } else {
        antiparallel(a.l, a.d0, &mut rng)
    };
    let (csv, end) = run(&f, &c0, &pos, &q, a.t_max, a.r_track, a.d_stop);
    fs::File::create(&a.out).expect("create output").write_all(csv.as_bytes()).expect("write output");
    // sidecar: the imprint positions and charges, for exact cross-checks against the Python instrument
    let meta: String = pos.iter().zip(&q).map(|(p, qi)| format!("{:.12} {:.12} {qi}\n", p.0, p.1)).collect();
    fs::write(format!("{}.pos0", a.out), meta).expect("write pos0");
    println!("{}: {end}", a.out);
}
