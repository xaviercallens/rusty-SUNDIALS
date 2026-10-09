//! Vortex instrument of the vortex-transport campaign (`exploration/pgpe/vortex_transport.py` of
//! SocrateAI-Scientific-QuantumFluids; amendments A1/A1.1 of `PGPE_FRICTION_PREREG.md`):
//!
//! 1. [`imprint_v2`]: periodic Jacobi-theta phase for any dipole moment + Bernoulli amplitude;
//! 2. [`detect`]: raw plaquette phase-winding detection with sub-grid refinement (`VortexWinding.lean`);
//! 3. [`Tracker`] / [`track_run`]: tracking by continuity, field momentum (total and band `|k| > 1`);
//! 4. placement generators ([`antiparallel`], [`dipole`]) and torus geometry helpers.
//!
//! Validated against the Python instrument by `VORTEX_TRANSPORT_CROSSCHECK.md` (positions to 5e-7, momentum to 4e-7
//! on the same imprint) and by the `T = 0` known answers of the campaign (see `tests` below).
use crate::ComplexField2D;
use num_complex::Complex64;
use std::f64::consts::PI;

const Q_NOME: f64 = 0.043_213_918_263_772_25; // e^{-pi}, nome of the square torus (tau = i)

/// Jacobi theta_1(u | tau = i) = 2 sum_{n>=0} (-1)^n q^{(n+1/2)^2} sin((2n+1) u), 10 terms as in `round2.py`.
pub fn theta1(u: Complex64) -> Complex64 {
    let mut out = Complex64::new(0.0, 0.0);
    for n in 0..10 {
        let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
        let q = Q_NOME.powf((n as f64 + 0.5).powi(2));
        out += sign * q * (u * (2 * n + 1) as f64).sin();
    }
    2.0 * out
}

/// `z / (|z| + 1e-300)`: exactly zero at a vortex core that sits on a grid node (as in the Python instrument; the
/// field is then exactly zero at that node, the physical core value, rather than a unit-modulus phase).
fn unit(z: Complex64) -> Complex64 {
    z / (z.norm() + 1e-300)
}

/// The product of theta-function phase factors over a configuration, evaluated at the grid shifted by `(sx, sy)`.
fn phase_factor(
    f: &ComplexField2D,
    pos: &[(f64, f64)],
    q: &[i32],
    sx: f64,
    sy: f64,
) -> Vec<Complex64> {
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

/// `imprint_v2`: periodic phase for any dipole moment + Bernoulli amplitude `[1/(1 + |v|^2/2)]^{1/2}`; norm restored.
///
/// The phase jump the theta product acquires across each box boundary is measured on the phase factor itself and
/// removed by a uniform phase gradient (the torus counterflow), so a configuration of any dipole moment is
/// single-valued; the amplitude is zero at the cores and `1 - v^2/2` far away (no `1/r^2` tail).
pub fn imprint_v2(
    f: &ComplexField2D,
    c: &[Complex64],
    pos: &[(f64, f64)],
    q: &[i32],
) -> Vec<Complex64> {
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
    let idx = |i: isize, j: isize| {
        (i.rem_euclid(n as isize) as usize) * n + j.rem_euclid(n as isize) as usize
    };
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

/// One detected vortex: sub-grid position (box coordinates) and integer winding.
pub type Vortex = (f64, f64, i32);

/// Raw plaquette vortices with sub-grid refinement: `(x, y, charge)`.
pub fn detect(f: &ComplexField2D, c: &[Complex64]) -> Vec<Vortex> {
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
            out.push((
                ((i as f64 + u) * f.dx).rem_euclid(f.l),
                ((j as f64 + v) * f.dx).rem_euclid(f.l),
                qg,
            ));
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

/// Minimum-image distance on the torus of side `l`.
pub fn torus_distance(a: (f64, f64), b: (f64, f64), l: f64) -> f64 {
    let (mut dx, mut dy) = (a.0 - b.0, a.1 - b.1);
    dx -= l * (dx / l).round();
    dy -= l * (dy / l).round();
    (dx * dx + dy * dy).sqrt()
}

/// Smallest distance between any vortex (`q > 0`) and any antivortex (`q < 0`).
pub fn min_opposite_distance(pos: &[(f64, f64)], q: &[i32], l: f64) -> f64 {
    let mut m = f64::INFINITY;
    for (i, &qi) in q.iter().enumerate() {
        for (j, &qj) in q.iter().enumerate() {
            if qi > 0 && qj < 0 {
                m = m.min(torus_distance(pos[i], pos[j], l));
            }
        }
    }
    m
}

/// Small deterministic generator for placements (documented as not numpy-identical: the campaign's claims are
/// ensemble statements, not single trajectories).
pub struct Lcg(pub u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Lcg(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    pub fn uniform(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

/// Two antiparallel dipoles: zero net charge and zero net dipole moment (no net impulse shed).
pub fn antiparallel(l: f64, d0: f64, rng: &mut Lcg) -> (Vec<(f64, f64)>, Vec<i32>) {
    let y0 = rng.uniform() * l;
    let (ox, oy) = (rng.uniform() * 0.5, rng.uniform() * 0.5);
    let p = vec![
        (
            (l / 4.0 + d0 / 2.0 + ox).rem_euclid(l),
            (y0 + oy).rem_euclid(l),
        ),
        (
            (l / 4.0 - d0 / 2.0 + ox).rem_euclid(l),
            (y0 + oy).rem_euclid(l),
        ),
        (
            (3.0 * l / 4.0 - d0 / 2.0 + ox).rem_euclid(l),
            (y0 + oy).rem_euclid(l),
        ),
        (
            (3.0 * l / 4.0 + d0 / 2.0 + ox).rem_euclid(l),
            (y0 + oy).rem_euclid(l),
        ),
    ];
    (p, vec![1, -1, 1, -1])
}

/// A single dipole (net impulse `2 pi n d`): vortex at `x0 + d0/2`, antivortex at `x0 - d0/2`, same `y0`.
pub fn dipole(l: f64, d0: f64, rng: &mut Lcg) -> (Vec<(f64, f64)>, Vec<i32>) {
    let (x0, y0) = (rng.uniform() * l, rng.uniform() * l);
    (
        vec![
            ((x0 + d0 / 2.0).rem_euclid(l), y0),
            ((x0 - d0 / 2.0).rem_euclid(l), y0),
        ],
        vec![1, -1],
    )
}

/// The uniform condensate `psi = 1` (density 1) as projected amplitudes.
pub fn uniform_condensate(f: &ComplexField2D) -> Vec<Complex64> {
    let mut c = vec![Complex64::new(0.0, 0.0); f.n * f.n];
    c[0] = Complex64::new((f.n * f.n) as f64, 0.0);
    c
}

/// Tracking by continuity: the same-sign detection nearest to the previous position, within `r_track`
/// (`r_first` at the first sample).
pub struct Tracker {
    pub q: Vec<i32>,
    pub last: Vec<(f64, f64)>,
    pub l: f64,
    pub r_track: f64,
    pub r_first: f64,
}

impl Tracker {
    pub fn new(pos0: &[(f64, f64)], q: &[i32], l: f64, r_track: f64) -> Self {
        Tracker {
            q: q.to_vec(),
            last: pos0.to_vec(),
            l,
            r_track,
            r_first: 3.0,
        }
    }

    /// Match the detections to the tracked vortices; `None` if any is lost.
    pub fn update(&mut self, det: &[Vortex], first: bool) -> Option<Vec<(f64, f64)>> {
        let mut cur = Vec::with_capacity(self.q.len());
        for (i, &qi) in self.q.iter().enumerate() {
            let best = det
                .iter()
                .filter(|d| d.2 == qi)
                .map(|d| (torus_distance((d.0, d.1), self.last[i], self.l), (d.0, d.1)))
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            match best {
                Some((r, p)) if r <= self.r_track || (first && r <= self.r_first) => cur.push(p),
                _ => return None,
            }
        }
        self.last = cur.clone();
        Some(cur)
    }
}

/// How a tracked run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    TMax,
    TrackLost,
    Annihilated,
}

impl Ended {
    pub fn name(self) -> &'static str {
        match self {
            Ended::TMax => "t_max",
            Ended::TrackLost => "track_lost",
            Ended::Annihilated => "annihilated",
        }
    }
}

/// One sample of a tracked run (every `dt_sample` time units).
#[derive(Debug, Clone)]
pub struct Sample {
    pub t: f64,
    pub pos: Vec<(f64, f64)>,
    pub n_det: usize,
    pub p: (f64, f64),
    pub p_hi: (f64, f64),
}

/// A tracked run: samples, how it ended, the final field and the relative energy drift.
pub struct TrackedRun {
    pub samples: Vec<Sample>,
    pub ended: Ended,
    pub c_final: Vec<Complex64>,
    pub drift_e: f64,
}

/// Parameters of a tracked run (campaign defaults: sample every time unit, `r_track = 3`, `d_stop = 1.5`).
#[derive(Debug, Clone, Copy)]
pub struct TrackConfig {
    pub t_max: f64,
    pub dt_sample: f64,
    pub r_track: f64,
    pub d_stop: f64,
}

impl TrackConfig {
    pub fn new(t_max: f64) -> Self {
        TrackConfig {
            t_max,
            dt_sample: 1.0,
            r_track: 3.0,
            d_stop: 1.5,
        }
    }
}

/// Evolve `c` (already imprinted), tracking the vortices at `pos0`/`q` every `dt_sample`; stops at `t_max`, when
/// a vortex is lost, or when any vortex and antivortex come within `d_stop`.
pub fn track_run(
    f: &ComplexField2D,
    c: Vec<Complex64>,
    pos0: &[(f64, f64)],
    q: &[i32],
    cfg: &TrackConfig,
) -> TrackedRun {
    let TrackConfig {
        t_max,
        dt_sample,
        r_track,
        d_stop,
    } = *cfg;
    let mut c = c;
    let e0 = f.energy(&c);
    let mut tr = Tracker::new(pos0, q, f.l, r_track);
    let mut samples = Vec::new();
    let mut t = 0.0;
    let mut ended = Ended::TMax;
    loop {
        let det = detect(f, &c);
        let Some(cur) = tr.update(&det, samples.is_empty()) else {
            ended = Ended::TrackLost;
            break;
        };
        samples.push(Sample {
            t,
            pos: cur.clone(),
            n_det: det.len(),
            p: momentum_band(f, &c, 0.0),
            p_hi: momentum_band(f, &c, 1.0),
        });
        if min_opposite_distance(&cur, q, f.l) < d_stop {
            ended = Ended::Annihilated;
            break;
        }
        if t >= t_max - 1e-9 {
            break;
        }
        c = f.run(&c, dt_sample);
        t += dt_sample;
    }
    let drift_e = (f.energy(&c) - e0).abs() / e0.abs();
    TrackedRun {
        samples,
        ended,
        c_final: c,
        drift_e,
    }
}

/// Header and rows of the CSV the Python estimators read: `t, x_i, y_i, n_det, Px, Py, Px_hi, Py_hi`.
pub fn csv(run: &TrackedRun, n_v: usize) -> String {
    let mut s = String::from("t");
    for i in 0..n_v {
        s += &format!(",x{i},y{i}");
    }
    s += ",n_det,Px,Py,Px_hi,Py_hi\n";
    for r in &run.samples {
        s += &format!("{}", r.t);
        for p in &r.pos {
            s += &format!(",{:.6},{:.6}", p.0, p.1);
        }
        s += &format!(
            ",{},{:.6},{:.6},{:.6},{:.6}\n",
            r.n_det, r.p.0, r.p.1, r.p_hi.0, r.p_hi.1
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gate G1 known answers of the campaign at `T = 0` (corrected imprint): a single pair `d0 = 10` neither
    /// shrinks nor changes momentum, with momentum `2 pi n d x (0.96..0.99)` and exactly two detections.
    #[test]
    fn g1_known_answer_single_pair() {
        let f = ComplexField2D::new(64, 32.0, 1.0, 0.01);
        let c0 = uniform_condensate(&f);
        let pos = vec![(15.0 + 4.0, 11.0), (15.0 - 4.0, 11.0)];
        let q = vec![1, -1];
        let c = imprint_v2(&f, &c0, &pos, &q);
        assert_eq!(detect(&f, &c).len(), 2);
        let (px, py) = momentum_band(&f, &c, 0.0);
        let ratio = (px * px + py * py).sqrt() / (2.0 * PI * 8.0);
        assert!((0.90..=1.0).contains(&ratio), "momentum ratio {ratio}");
        let run = track_run(&f, c, &pos, &q, &TrackConfig::new(40.0));
        assert_eq!(run.ended, Ended::TMax);
        let d = |s: &Sample| torus_distance(s.pos[0], s.pos[1], f.l);
        let (d1, d2) = (d(&run.samples[5]), d(run.samples.last().unwrap()));
        // Python instrument, same configuration (N = 64, L = 32, d = 8): separation 7.626664 at t = 5, 7.814511 at t = 40
        // (the pair oscillates by 0.19 in this small box because of its periodic images)
        assert!(
            (d1 - 7.626_664_287).abs() < 1e-4 && (d2 - 7.814_510_712).abs() < 1e-4,
            "separation {d1} -> {d2}"
        );
        let (p1, p2) = (
            run.samples[0].p.0.hypot(run.samples[0].p.1),
            run.samples
                .last()
                .unwrap()
                .p
                .0
                .hypot(run.samples.last().unwrap().p.1),
        );
        assert!(((p2 - p1) / p1).abs() < 1e-6, "momentum {p1} -> {p2}");
        assert!(run.drift_e < 1e-6, "energy drift {}", run.drift_e);
    }

    /// A vortex centred exactly on a grid node has zero field there (Python-identical convention), and the imprint
    /// agrees with the Python instrument on such a configuration (momentum 47.3351 for N = 64, L = 32, d = 8).
    #[test]
    fn imprint_on_a_grid_node_matches_python() {
        let f = ComplexField2D::new(64, 32.0, 1.0, 0.01);
        let c = imprint_v2(
            &f,
            &uniform_condensate(&f),
            &[(19.0, 11.0), (11.0, 11.0)],
            &[1, -1],
        );
        let (px, py) = momentum_band(&f, &c, 0.0);
        assert!(
            (px.hypot(py) - 47.335_149_138_257_76).abs() < 1e-6,
            "|P| = {}",
            px.hypot(py)
        );
    }

    /// The imprint of a neutral pair is single-valued on the torus: detection finds exactly the imprinted charges.
    #[test]
    fn imprint_is_periodic_for_any_dipole_moment() {
        let f = ComplexField2D::new(64, 32.0, 1.0, 0.01);
        let c0 = uniform_condensate(&f);
        for d0 in [3.0, 7.3, 15.9] {
            let pos = vec![(10.3 + d0 / 2.0, 14.1), (10.3 - d0 / 2.0, 14.1)];
            let q = vec![1, -1];
            let det = detect(&f, &imprint_v2(&f, &c0, &pos, &q));
            assert_eq!(det.len(), 2, "d0 = {d0}");
            assert_eq!(det.iter().map(|d| d.2).sum::<i32>(), 0);
        }
    }

    #[test]
    fn tracker_loses_a_vanished_vortex_and_geometry_helpers() {
        assert!((torus_distance((0.5, 0.5), (31.5, 31.5), 32.0) - 2f64.sqrt()).abs() < 1e-12);
        let mut rng = Lcg::new(1);
        let (p, q) = antiparallel(64.0, 10.0, &mut rng);
        assert_eq!(p.len(), 4);
        assert!((min_opposite_distance(&p, &q, 64.0) - 10.0).abs() < 1e-9);
        let mut tr = Tracker::new(&p, &q, 64.0, 3.0);
        let det: Vec<Vortex> = p.iter().zip(&q).map(|(a, &qi)| (a.0, a.1, qi)).collect();
        assert!(tr.update(&det, true).is_some());
        assert!(tr.update(&det[..3], false).is_none());
    }
}
