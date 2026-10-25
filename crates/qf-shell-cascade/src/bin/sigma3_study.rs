//! H7 stage-2 numerics: the sigma = 3 regulated shell model integrated with CVODE (BDF).
//!
//! ```text
//! sigma3_study probe <n_max> <sigma> <D> <T> <rtol>        one timing/accuracy probe, printed
//! sigma3_study full <out.json> <well_amps.txt> <git_sha>   the whole study, written as JSON
//! ```
//!
//! `well_amps.txt`: one line per data snapshot, `time_index a_0 a_1 ... a_5` (shell moduli from
//! The Well, written by `results/sigma3/scripts/make_amps.py`).
//!
//! Every reported supremum is taken over CVODE's internal step points (one-step mode, no
//! interpolation); `t_reached` is the last internal time (>= the requested horizon).

use num_complex::Complex64;
use qf_shell_cascade::sigma::{CvodeSettings, SigmaModel, SigmaRun, integrate_cvode, mass};
use std::fmt::Write as _;
use std::time::Instant;

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed ^ 0x2545_F491_4F6C_DD1D)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn unif(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn phase(&mut self) -> Complex64 {
        Complex64::from_polar(1.0, 2.0 * std::f64::consts::PI * self.unif())
    }
}

/// `|v_n| = 2^{-1.5 n}` with random phases, normalised to `S = 1`: a family whose `Omega(0)` and
/// `E(0)` stay bounded uniformly in `n_max` (the class the cutoff-uniformity caveat requires).
fn decaying_state(n_max: usize, seed: u64) -> Vec<Complex64> {
    let mut rng = Rng::new(seed);
    let v: Vec<Complex64> = (0..=n_max)
        .map(|n| rng.phase() * 2f64.powf(-1.5 * n as f64))
        .collect();
    normalise(v)
}

/// Top-shell-dominated state: `v_{n_max} = 1`, all other shells `1e-3` with random phases, `S = 1`.
fn top_shell_state(n_max: usize, seed: u64) -> Vec<Complex64> {
    let mut rng = Rng::new(seed);
    let v: Vec<Complex64> = (0..=n_max)
        .map(|n| {
            if n == n_max {
                Complex64::new(1.0, 0.0)
            } else {
                rng.phase() * 1e-3
            }
        })
        .collect();
    normalise(v)
}

fn normalise(v: Vec<Complex64>) -> Vec<Complex64> {
    let s = mass(&v).sqrt();
    v.into_iter().map(|z| z / s).collect()
}

fn settings(rtol: f64) -> CvodeSettings {
    CvodeSettings {
        rtol,
        atol: rtol * 1e-3,
        max_steps: 20_000_000,
        analytic_jacobian: true,
        trace_every: 0,
    }
}

fn run_json(
    label: &str,
    model: &SigmaModel,
    rtol: f64,
    t_end: f64,
    r: &SigmaRun,
    secs: f64,
) -> String {
    let mut s = String::new();
    write!(
        s,
        "{{\"label\":\"{label}\",\"n_max\":{},\"lean_N\":{},\"sigma\":{},\"D\":{},\"perturb\":{},\
         \"rtol\":{rtol:e},\"atol\":{:e},\"t_end\":{t_end},\"t_reached\":{},\"steps\":{},\"rhs_evals\":{},\
         \"wall_s\":{secs:.3},\"S0\":{},\"H0\":{},\"H_scale0\":{},\"Omega0\":{},\"bound_rhs\":{},\
         \"max_rel_dS\":{:e},\"max_rel_dH\":{:e},\"sup_2DOmega\":{},\"sup_Omega\":{},\"sup_ratio\":{},\
         \"S_final\":{},\"H_final\":{},\"S_drift_signed\":{:e}",
        model.n_max,
        model.n_max as i64 - 1,
        model.sigma,
        model.d,
        match model.perturb {
            Some((j, e)) => format!("[{j},{e}]"),
            None => "null".into(),
        },
        rtol * 1e-3,
        r.t_reached,
        r.steps,
        r.rhs_evals,
        r.s0,
        r.h0,
        r.h_scale0,
        r.omega0,
        r.bound_rhs,
        r.max_rel_ds,
        r.max_rel_dh,
        r.sup_two_d_omega,
        r.sup_omega,
        r.sup_ratio,
        r.trace.last().map(|x| x.1).unwrap_or(f64::NAN),
        r.trace.last().map(|x| x.2).unwrap_or(f64::NAN),
        (r.trace.last().map(|x| x.1).unwrap_or(f64::NAN) - r.s0) / r.s0,
    )
    .unwrap();
    if r.trace.len() > 1 {
        s.push_str(",\"trace_t_S_H_Omega\":[");
        let stride = (r.trace.len() / 400).max(1);
        let mut first = true;
        for (i, &(t, sv, h, om)) in r.trace.iter().enumerate() {
            if i % stride == 0 || i + 1 == r.trace.len() {
                if !first {
                    s.push(',');
                }
                first = false;
                write!(s, "[{t},{sv},{h},{om}]").unwrap();
            }
        }
        s.push(']');
    }
    s.push('}');
    s
}

fn go(
    label: &str,
    model: SigmaModel,
    v0: &[Complex64],
    t_end: f64,
    rtol: f64,
    trace_every: usize,
) -> String {
    let mut set = settings(rtol);
    set.trace_every = trace_every;
    let t0 = Instant::now();
    match integrate_cvode(model, v0, t_end, set) {
        Ok(r) => {
            let secs = t0.elapsed().as_secs_f64();
            eprintln!(
                "{label}: n_max={} sigma={} D={} rtol={rtol:e} steps={} dS={:.2e} dH={:.2e} ratio={:.4} t={:.3} {:.1}s",
                model.n_max,
                model.sigma,
                model.d,
                r.steps,
                r.max_rel_ds,
                r.max_rel_dh,
                r.sup_ratio,
                r.t_reached,
                secs
            );
            run_json(label, &model, rtol, t_end, &r, secs)
        }
        Err(e) => {
            eprintln!("{label}: FAILED {e}");
            format!(
                "{{\"label\":\"{label}\",\"n_max\":{},\"sigma\":{},\"D\":{},\"rtol\":{rtol:e},\"error\":{:?}}}",
                model.n_max, model.sigma, model.d, e
            )
        }
    }
}

fn model(n_max: usize, sigma: f64, d: f64) -> SigmaModel {
    SigmaModel {
        n_max,
        d,
        sigma,
        perturb: None,
    }
}

fn read_amps(path: &str) -> Vec<(usize, Vec<f64>)> {
    let text = std::fs::read_to_string(path).expect("cannot read amplitude file");
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut it = l.split_whitespace();
            let ti: usize = it.next().unwrap().parse().unwrap();
            let a: Vec<f64> = it.map(|x| x.parse().unwrap()).collect();
            (ti, a)
        })
        .collect()
}

const D3: f64 = 0.05;

fn full(out: &str, amps_path: &str, sha: &str) {
    let mut sections: Vec<String> = Vec::new();
    let t_start = Instant::now();

    // P1. Positive control + convergence: sigma = 3, decaying data, rtol sweep at n_max = 4, 6.
    let mut v = Vec::new();
    for &n in &[4usize, 6] {
        let v0 = decaying_state(n, 100 + n as u64);
        for &rtol in &[1e-6, 1e-8, 1e-10] {
            v.push(go("P1_convergence", model(n, 3.0, D3), &v0, 2.0, rtol, 0));
        }
    }
    sections.push(format!("\"P1_positive_convergence\":[{}]", v.join(",")));

    // N1. Negative control: perturbed outgoing coupling at shell 1 (eps = 1e-2), same data and rtol
    // sweep. The structural drift must not shrink with rtol.
    let mut v = Vec::new();
    for &n in &[4usize, 6] {
        let v0 = decaying_state(n, 100 + n as u64);
        for &rtol in &[1e-6, 1e-8, 1e-10] {
            let m = SigmaModel {
                perturb: Some((1, 1e-2)),
                ..model(n, 3.0, D3)
            };
            v.push(go("N1_perturbed_coupling", m, &v0, 2.0, rtol, 0));
        }
    }
    sections.push(format!("\"N1_negative_perturbed\":[{}]", v.join(",")));

    // S3. sigma = 3 sweep over truncations, decaying data, fixed D, T = 2, rtol 1e-10, 2 phase seeds.
    let mut v = Vec::new();
    for &n in &[2usize, 3, 4, 5, 6, 7] {
        for seed in [1u64, 2] {
            let v0 = decaying_state(n, 1000 * seed + n as u64);
            v.push(go("S3_sweep", model(n, 3.0, D3), &v0, 2.0, 1e-10, 50));
        }
    }
    sections.push(format!("\"S3_sigma3_sweep\":[{}]", v.join(",")));

    // C2. sigma = 2 versus sigma = 3 on top-shell-dominated data: sup_t 2 D Omega / (H(0) + S^{3/2}).
    let mut v = Vec::new();
    for &n in &[2usize, 3, 4, 5, 6] {
        let v0 = top_shell_state(n, 7 + n as u64);
        v.push(go("C2_sigma2", model(n, 2.0, D3), &v0, 0.5, 1e-10, 0));
        v.push(go("C2_sigma3", model(n, 3.0, D3), &v0, 0.5, 1e-10, 0));
    }
    sections.push(format!("\"C2_sigma2_vs_sigma3\":[{}]", v.join(",")));

    // W. Data-driven: shell moduli from The Well (S normalised to 1), random phases (2 seeds),
    // shells above n = 5 set to zero; n_max in {5, 6, 7}.
    let amps = read_amps(amps_path);
    let mut v = Vec::new();
    for (ti, a) in &amps {
        for &n in &[5usize, 6, 7] {
            for seed in [11u64, 12] {
                let mut rng = Rng::new(seed * 1000 + *ti as u64);
                let v0: Vec<Complex64> = (0..=n)
                    .map(|k| {
                        let ph = rng.phase();
                        if k < a.len() {
                            ph * a[k]
                        } else {
                            Complex64::new(0.0, 0.0)
                        }
                    })
                    .collect();
                let v0 = normalise(v0);
                let label = format!("W_t{ti}_seed{seed}");
                v.push(go(&label, model(n, 3.0, D3), &v0, 2.0, 1e-10, 50));
            }
        }
    }
    sections.push(format!("\"W_well_data\":[{}]", v.join(",")));

    let json = format!(
        "{{\"crate\":\"qf-shell-cascade\",\"binary\":\"sigma3_study\",\"git_sha\":\"{sha}\",\"D\":{D3},\
         \"note\":\"suprema over CVODE internal steps (BDF, one-step mode); S normalised to 1; H scale = |Q|+sum|cubic terms|\",\
         \"total_wall_s\":{:.1},{}}}",
        t_start.elapsed().as_secs_f64(),
        sections.join(",")
    );
    std::fs::write(out, json).expect("cannot write output");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("probe") => {
            let n: usize = args[2].parse().unwrap();
            let sigma: f64 = args[3].parse().unwrap();
            let d: f64 = args[4].parse().unwrap();
            let t: f64 = args[5].parse().unwrap();
            let rtol: f64 = args[6].parse().unwrap();
            let v0 = decaying_state(n, 100 + n as u64);
            println!(
                "{}",
                go("probe_decaying", model(n, sigma, d), &v0, t, rtol, 0)
            );
            let v1 = top_shell_state(n, 7 + n as u64);
            println!(
                "{}",
                go("probe_top_shell", model(n, sigma, d), &v1, t, rtol, 0)
            );
        }
        Some("full") => full(&args[2], &args[3], &args[4]),
        _ => {
            eprintln!(
                "usage: sigma3_study probe <n_max> <sigma> <D> <T> <rtol> | full <out.json> <well_amps.txt> <git_sha>"
            );
            std::process::exit(2);
        }
    }
}
