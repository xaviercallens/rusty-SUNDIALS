//! Vortex-phonon scattering by a Bogoliubov wave (`exploration/pgpe/vortex_wave_scattering.py` and
//! `analyze_wave_scan.py` of SocrateAI-Scientific-QuantumFluids; `docs/designs/PGPE_VORTEX_SCATTERING_PREREG.md`).
//!
//! A neutral vortex-antivortex pair at separation `d0` along `x` in the uniform `T = 0` condensate (`psi = 1`) is dressed
//! with a travelling Bogoliubov wave of wavenumber `k = 2 pi m / L` along `+-y`,
//!
//! ```text
//! psi = psi_pair * (1 + eps [u e^{i dir k y} - v e^{-i dir k y}]),   u, v = sqrt((e0 + 1)/(2 w) +- 1/2),
//! ```
//!
//! `e0 = k^2/2`, `w = sqrt(e0 (e0 + 2))`, and the norm is restored. A wave of momentum flux `j` (measured on the same
//! dressed field WITHOUT vortices) exerts the Magnus-balanced force that makes the pair separation `d_x(t)` and the
//! centroid `y_c(t)` drift; with the no-wave control subtracted,
//!
//! ```text
//! sigma_par = |d slope(d_x)| 2 pi / (2 j),    sigma_perp = |d slope(y_c)| 2 pi / j.
//! ```
//!
//! Indexing is the Python one: `psi[i (x), j (y)]`, row-major, wave phase `dir k y_j` with `y_j = j dx`.
use crate::ComplexField2D;
use crate::vortex::{Ended, Tracker, detect, momentum_band, uniform_condensate};
use num_complex::Complex64;
use std::f64::consts::PI;

/// `2 pi` kappa of the unit-density, `c = 1` condensate.
pub const KAPPA: f64 = 2.0 * PI;

/// Bogoliubov amplitudes `(u, v, omega)` at wavenumber `k` (`g = n = 1`).
pub fn bogoliubov_uv(k: f64) -> (f64, f64, f64) {
    let e0 = 0.5 * k * k;
    let w = (e0 * (e0 + 2.0)).sqrt();
    let u = ((e0 + 1.0) / (2.0 * w) + 0.5).sqrt();
    let v = ((e0 + 1.0) / (2.0 * w) - 0.5).max(0.0).sqrt();
    (u, v, w)
}

/// Wavenumber of the `m`-th box mode.
pub fn wavenumber(l: f64, m: usize) -> f64 {
    2.0 * PI * m as f64 / l
}

/// `eps` such that the peak velocity amplitude `k eps (u + v)` equals `av`.
pub fn eps_from_av(k: f64, av: f64) -> f64 {
    let (u, v, _) = bogoliubov_uv(k);
    av / (k * (u + v))
}

/// Dress the amplitudes `c` with the wave of mode number `m` along `dir = +-1`; returns `(c', k, omega)`.
pub fn dress(
    f: &ComplexField2D,
    c: &[Complex64],
    m: usize,
    eps: f64,
    dir: i32,
) -> (Vec<Complex64>, f64, f64) {
    let n = f.n;
    let k = wavenumber(f.l, m);
    let (u, v, w) = bogoliubov_uv(k);
    let mut psi = f.psi(c);
    for j in 0..n {
        let ph = dir as f64 * k * (j as f64 * f.dx);
        let wave =
            1.0 + eps * (u * Complex64::new(0.0, ph).exp() - v * Complex64::new(0.0, -ph).exp());
        for i in 0..n {
            psi[i * n + j] *= wave;
        }
    }
    let mut c2 = f.modes(&psi);
    let s = (f.norm(c) / f.norm(&c2)).sqrt();
    c2.iter_mut().for_each(|z| *z *= s);
    (c2, k, w)
}

/// The wave alone on the vortex-free condensate: `(j, P_wave, amp_rho)` with `j = |P|/L^2` and
/// `amp_rho = (max n - min n)/2` of the dressed density.
pub fn wave_flux(
    f: &ComplexField2D,
    m: usize,
    eps: f64,
    dir: i32,
) -> (f64, (f64, f64), f64, Vec<Complex64>) {
    let c0 = uniform_condensate(f);
    let (cw, _, _) = dress(f, &c0, m, eps, dir);
    let p = momentum_band(f, &cw, 0.0);
    let j = p.0.hypot(p.1) / (f.l * f.l);
    let dens: Vec<f64> = f.psi(&cw).iter().map(|z| z.norm_sqr()).collect();
    let (mx, mn) = dens
        .iter()
        .fold((f64::MIN, f64::MAX), |(a, b), &d| (a.max(d), b.min(d)));
    (j, p, (mx - mn) / 2.0, cw)
}

/// Parameters of one scattering run.
#[derive(Debug, Clone)]
pub struct WaveParams {
    pub m: usize,
    pub eps: f64,
    pub dir: i32,
    pub t_max: f64,
    pub d0: f64,
    pub dt_sample: f64,
    pub r_track: f64,
    pub no_pair: bool,
}

impl WaveParams {
    pub fn new(m: usize, eps: f64, dir: i32, t_max: f64, d0: f64) -> Self {
        WaveParams {
            m,
            eps,
            dir,
            t_max,
            d0,
            dt_sample: 1.0,
            r_track: 3.0,
            no_pair: false,
        }
    }
}

/// Samples and metadata of a run (the Python `.npz`).
#[derive(Debug, Clone)]
pub struct WaveRun {
    pub t: Vec<f64>,
    /// Tracked positions per sample: `[vortex, antivortex]` (empty with `no_pair`).
    pub r: Vec<Vec<(f64, f64)>>,
    pub p: Vec<(f64, f64)>,
    pub e: Vec<f64>,
    pub ended: Ended,
    pub t_end: f64,
    pub k: f64,
    pub omega: f64,
    pub eps: f64,
    pub dir: i32,
    pub l: f64,
    pub n: usize,
    pub d0: f64,
    pub j: f64,
    pub p_wave: (f64, f64),
    pub amp_rho: f64,
    pub drift_e: f64,
}

/// Run the pair (or, with `no_pair`, the wave alone): dress, evolve and track with the Python loop (track, record,
/// advance by `dt_sample`; a lost vortex ends the run before the sample is recorded; no annihilation stop).
pub fn run_wave_pair(f: &ComplexField2D, a: &WaveParams) -> WaveRun {
    let (j, p_wave, amp_rho, cw) = wave_flux(f, a.m, a.eps, a.dir);
    let l = f.l;
    let (pos, q, mut c, k, omega) = if a.no_pair {
        let k = wavenumber(l, a.m);
        (Vec::new(), Vec::new(), cw, k, bogoliubov_uv(k).2)
    } else {
        let yc = l / 2.0;
        let pos = vec![
            ((l / 2.0 - a.d0 / 2.0).rem_euclid(l), yc),
            ((l / 2.0 + a.d0 / 2.0).rem_euclid(l), yc),
        ];
        let q = vec![1, -1];
        let cp = crate::vortex::imprint_v2(f, &uniform_condensate(f), &pos, &q);
        let (c, k, w) = dress(f, &cp, a.m, a.eps, a.dir);
        (pos, q, c, k, w)
    };
    let mut tr = Tracker::new(&pos, &q, l, a.r_track);
    let e0 = f.energy(&c);
    let (mut t_v, mut r_v, mut p_v, mut e_v) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut t = 0.0;
    let mut ended = Ended::TMax;
    loop {
        if !q.is_empty() {
            let det = detect(f, &c);
            match tr.update(&det, t_v.is_empty()) {
                Some(cur) => r_v.push(cur),
                None => {
                    ended = Ended::TrackLost;
                    break;
                }
            }
        } else {
            r_v.push(Vec::new());
        }
        t_v.push(t);
        p_v.push(momentum_band(f, &c, 0.0));
        e_v.push(f.energy(&c));
        if t >= a.t_max - 1e-9 {
            break;
        }
        c = f.run(&c, a.dt_sample);
        t += a.dt_sample;
    }
    let drift_e = (f.energy(&c) - e0).abs() / e0.abs();
    WaveRun {
        t: t_v,
        r: r_v,
        p: p_v,
        e: e_v,
        ended,
        t_end: t,
        k,
        omega,
        eps: a.eps,
        dir: a.dir,
        l,
        n: f.n,
        d0: a.d0,
        j,
        p_wave,
        amp_rho,
        drift_e,
    }
}

/// `numpy.unwrap` with `period = 2 pi` (`discont = pi`).
pub fn unwrap(p: &[f64]) -> Vec<f64> {
    let mut out = p.to_vec();
    let mut corr = 0.0;
    for i in 1..p.len() {
        let dd = p[i] - p[i - 1];
        let mut ddmod = (dd + PI).rem_euclid(2.0 * PI) - PI;
        if ddmod == -PI && dd > 0.0 {
            ddmod = PI;
        }
        if dd.abs() >= PI {
            corr += ddmod - dd;
        }
        out[i] = p[i] + corr;
    }
    out
}

/// `np.unwrap(a 2 pi / L) L / (2 pi)`: positions unwrapped on the torus of side `l`.
pub fn unwrap_torus(a: &[f64], l: f64) -> Vec<f64> {
    let s: Vec<f64> = a.iter().map(|x| x * 2.0 * PI / l).collect();
    unwrap(&s).into_iter().map(|x| x * l / (2.0 * PI)).collect()
}

/// Least-squares slope of `y(t)` (`np.polyfit(t, y, 1)[0]`).
pub fn slope(t: &[f64], y: &[f64]) -> f64 {
    let n = t.len() as f64;
    let (tm, ym) = (t.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let num: f64 = t.iter().zip(y).map(|(a, b)| (a - tm) * (b - ym)).sum();
    let den: f64 = t.iter().map(|a| (a - tm) * (a - tm)).sum();
    num / den
}

/// Drift slopes of one run (`analyze_wave_scan.slopes`).
#[derive(Debug, Clone, Copy)]
pub struct Slopes {
    /// slope of the separation `d_x = x_0 - x_1` (unwrapped).
    pub sdx: f64,
    /// slope of the centroid `y_c`.
    pub syc: f64,
    /// `d_x(end) - d_x(0)`.
    pub d_change: f64,
    pub j: f64,
    pub k: f64,
    pub eps: f64,
    pub dir: i32,
}

/// Slopes of `d_x(t)` and `y_c(t)` over `t >= t0`; `None` if the run did not end at `t_max` or is shorter than 100
/// samples (the Python rule).
pub fn drift_slopes(run: &WaveRun, l: f64, t0: f64) -> Option<Slopes> {
    drift_slopes_min(run, l, t0, 100)
}

/// [`drift_slopes`] with an explicit minimum number of samples (short runs of the cross-check fixtures).
pub fn drift_slopes_min(run: &WaveRun, l: f64, t0: f64, min_samples: usize) -> Option<Slopes> {
    if run.ended != Ended::TMax || run.r.len() < min_samples || run.r[0].len() < 2 {
        return None;
    }
    let col = |v: usize, ax: usize| -> Vec<f64> {
        let a: Vec<f64> = run
            .r
            .iter()
            .map(|r| if ax == 0 { r[v].0 } else { r[v].1 })
            .collect();
        unwrap_torus(&a, l)
    };
    let (x0, x1, y0, y1) = (col(0, 0), col(1, 0), col(0, 1), col(1, 1));
    let dx: Vec<f64> = x0.iter().zip(&x1).map(|(a, b)| a - b).collect();
    let yc: Vec<f64> = y0.iter().zip(&y1).map(|(a, b)| 0.5 * (a + b)).collect();
    let idx: Vec<usize> = (0..run.t.len()).filter(|&i| run.t[i] >= t0).collect();
    let tt: Vec<f64> = idx.iter().map(|&i| run.t[i]).collect();
    let pick = |v: &[f64]| -> Vec<f64> { idx.iter().map(|&i| v[i]).collect() };
    Some(Slopes {
        sdx: slope(&tt, &pick(&dx)),
        syc: slope(&tt, &pick(&yc)),
        d_change: dx[dx.len() - 1] - dx[0],
        j: run.j,
        k: run.k,
        eps: run.eps,
        dir: run.dir,
    })
}

/// `sigma_par`, `sigma_perp` of one pair of directions against a control (`analyze_wave_scan`, `analyze_wave_scan_d0`).
#[derive(Debug, Clone, Copy)]
pub struct Sigma {
    pub k: f64,
    pub j: f64,
    pub sigma_par: f64,
    pub sp: f64,
    pub sm: f64,
    pub sigma_perp: f64,
    pub tp: f64,
    pub tm: f64,
    /// `d_x` drifts are opposite for the two directions (sign test only, as in the d0 analysis).
    pub odd: bool,
    /// Odd AND `|sp - sm| / max <= 0.25` (the stricter test of the first scan).
    pub odd_strict: bool,
    pub d_dx_p: f64,
    pub d_dx_m: f64,
}

/// Combine the `+y` (`p`) and `-y` (`q`) runs of one wavenumber with the control slopes `(sdx, syc)`.
pub fn sigma_from_pair(p: &Slopes, q: &Slopes, control: (f64, f64)) -> Sigma {
    let (dpx, dqx) = (p.sdx - control.0, q.sdx - control.0);
    let (dpy, dqy) = (p.syc - control.1, q.syc - control.1);
    let sp = dpx.abs() * KAPPA / (2.0 * p.j);
    let sm = dqx.abs() * KAPPA / (2.0 * q.j);
    let tp = dpy.abs() * KAPPA / p.j;
    let tm = dqy.abs() * KAPPA / q.j;
    let odd = dpx.signum() == -dqx.signum();
    Sigma {
        k: p.k,
        j: 0.5 * (p.j + q.j),
        sigma_par: 0.5 * (sp + sm),
        sp,
        sm,
        sigma_perp: 0.5 * (tp + tm),
        tp,
        tm,
        odd,
        odd_strict: odd && (sp - sm).abs() / sp.max(sm) <= 0.25,
        d_dx_p: dpx,
        d_dx_m: dqx,
    }
}

impl WaveRun {
    /// `t, x0, y0, x1, y1, Px, Py, E` at full precision.
    pub fn csv(&self) -> String {
        let mut s = String::from("t,x0,y0,x1,y1,Px,Py,E\n");
        for (i, t) in self.t.iter().enumerate() {
            let r = &self.r[i];
            let (a, b) = if r.len() == 2 {
                (r[0], r[1])
            } else {
                ((f64::NAN, f64::NAN), (f64::NAN, f64::NAN))
            };
            s += &format!(
                "{t},{},{},{},{},{},{},{}\n",
                a.0, a.1, b.0, b.1, self.p[i].0, self.p[i].1, self.e[i]
            );
        }
        s
    }

    /// Flat JSON of the metadata (the Python `meta`).
    pub fn meta_json(&self, seconds: f64) -> String {
        format!(
            "{{\n \"k\": {},\n \"omega\": {},\n \"eps\": {},\n \"dir\": {},\n \"L\": {},\n \"N\": {},\n \"d0\": {},\n \"ended\": \"{}\",\n \"t_end\": {},\n \"j\": {},\n \"P_wave_x\": {},\n \"P_wave_y\": {},\n \"amp_rho\": {},\n \"drift_E\": {},\n \"seconds\": {}\n}}\n",
            self.k,
            self.omega,
            self.eps,
            self.dir,
            self.l,
            self.n,
            self.d0,
            self.ended.name(),
            self.t_end,
            self.j,
            self.p_wave.0,
            self.p_wave.1,
            self.amp_rho,
            self.drift_e,
            seconds
        )
    }

    /// Rebuild a run from the files written by the `wave_scattering` example (for resuming a scan).
    pub fn from_files(csv: &str, meta: &str) -> Option<WaveRun> {
        let num = |key: &str| -> Option<f64> { json_field(meta, key)?.parse().ok() };
        let ended = match json_field(meta, "ended")?.trim_matches('"') {
            "t_max" => Ended::TMax,
            "track_lost" => Ended::TrackLost,
            _ => Ended::Annihilated,
        };
        let (mut t, mut r, mut p, mut e) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for line in csv.lines().skip(1) {
            let v: Vec<f64> = line
                .split(',')
                .map(|x| x.parse().unwrap_or(f64::NAN))
                .collect();
            if v.len() != 8 {
                return None;
            }
            t.push(v[0]);
            r.push(if v[1].is_nan() {
                Vec::new()
            } else {
                vec![(v[1], v[2]), (v[3], v[4])]
            });
            p.push((v[5], v[6]));
            e.push(v[7]);
        }
        Some(WaveRun {
            t,
            r,
            p,
            e,
            ended,
            t_end: num("t_end")?,
            k: num("k")?,
            omega: num("omega")?,
            eps: num("eps")?,
            dir: num("dir")? as i32,
            l: num("L")?,
            n: num("N")? as usize,
            d0: num("d0")?,
            j: num("j")?,
            p_wave: (num("P_wave_x")?, num("P_wave_y")?),
            amp_rho: num("amp_rho")?,
            drift_e: num("drift_E")?,
        })
    }
}

/// Value of `"key": value` in the flat one-line-per-field JSON written by [`WaveRun::meta_json`].
pub fn json_field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let i = s.find(&pat)? + pat.len();
    let rest = &s[i..];
    let end = rest.find([',', '\n', '}']).unwrap_or(rest.len());
    Some(rest[..end].trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_satisfy_the_bogoliubov_normalisation() {
        for k in [0.2, 0.7, 2.0] {
            let (u, v, _) = bogoliubov_uv(k);
            assert!((u * u - v * v - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn unwrap_matches_numpy_on_a_ramp() {
        let a: Vec<f64> = (0..30)
            .map(|i| (0.9 * i as f64 + 1.0).rem_euclid(2.0 * PI))
            .collect();
        let u = unwrap(&a);
        for (i, ui) in u.iter().enumerate() {
            assert!((ui - (a[0] + 0.9 * i as f64)).abs() < 1e-12, "i = {i}");
        }
    }

    #[test]
    fn slope_of_a_line() {
        let t: Vec<f64> = (0..20).map(f64::from).collect();
        let y: Vec<f64> = t.iter().map(|x| 3.0 - 0.25 * x).collect();
        assert!((slope(&t, &y) + 0.25).abs() < 1e-14);
    }

    #[test]
    fn wave_without_vortices_has_the_expected_flux_scale() {
        // peak velocity amplitude av = k eps (u + v): the momentum density of a Bogoliubov wave is O(eps^2).
        let f = ComplexField2D::new(32, 16.0, 1.0, 0.01);
        let k = wavenumber(f.l, 2);
        let eps = eps_from_av(k, 0.04);
        let (j, p, amp, _) = wave_flux(&f, 2, eps, 1);
        assert!(j > 0.0 && amp > 0.0);
        assert!(p.0.abs() < 1e-12 * p.1.abs().max(1.0), "wave along +y only");
        let (j2, _, _, _) = wave_flux(&f, 2, eps, -1);
        assert!((j - j2).abs() < 1e-14 * j);
    }
}
