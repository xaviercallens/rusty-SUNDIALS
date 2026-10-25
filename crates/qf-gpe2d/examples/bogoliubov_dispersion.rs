//! Bogoliubov dispersion by linear response of the GP ground state, with controls.
//!
//! Writes one JSON document to the path given as the first argument:
//!
//! * `sweep_phase_g1`: phase kick, g = 1, 25 wavevectors k/k_s in [0.05, 5] (log spaced);
//! * `sweep_density_g1`: density kick, same k (an independent frequency readout);
//! * `sweep_phase_g0`: negative control, g = 0 (free particle), same k;
//! * `sweep_phase_g1_reference`: the g = 1 phase sweep with the crate's Python-reference
//!   splitting (first order) instead of Strang — the recorded failure mode;
//! * `sweep_phase_g1_uncapped`: Strang, but with the `max_dt` cap switched off;
//! * `dt_convergence`: dt = max_dt(grid, s) for s = 1, 1/2, 1/4, 1/8, 1/16 at k/k_s = 0.25, 1, 4
//!   (Strang, eps = 1e-4 so the O(eps^2) frequency shift sits below the time-step error);
//! * `dt_convergence_reference`: the same refinement at k/k_s = 1 with the reference splitting;
//! * `eps_scan`: kick strengths 1e-3 .. 3e-2 at k/k_s = 1;
//! * `diagonal_mode`: the same k on modes (2,0), (1,1) and (2,2) — isotropy on the grid.
//!
//! Run: `cargo run --release -p qf-gpe2d --example bogoliubov_dispersion -- OUT.json`

use qf_gpe2d::Splitting;
use qf_gpe2d::linear_response::{
    Kick, Measurement, ResponseConfig, bogoliubov_energy, bogoliubov_s, dual_scale_bound,
    free_energy, measure, sound_wavenumber,
};
use std::fmt::Write as _;

const N: usize = 32;
const PERIODS: usize = 6;
const SPP: usize = 200;
const EPS: f64 = 1e-3;
const SAFETY: f64 = 0.25;

struct Run {
    g: f64,
    kick: Kick,
    spp: usize,
    eps: f64,
    mode: (i64, i64),
    safety: Option<f64>,
    splitting: Splitting,
}

fn cfg(k: f64, r: &Run) -> ResponseConfig {
    ResponseConfig {
        k,
        mode: r.mode,
        n: N,
        g: r.g,
        n0: 1.0,
        eps: r.eps,
        steps_per_period: r.spp,
        periods: PERIODS,
        omega_ref: bogoliubov_energy(k, r.g, 1.0),
        kick: r.kick,
        dt_safety: r.safety,
        splitting: r.splitting,
    }
}

fn row(m: &Measurement, eps: f64, kick: Kick, extra: &str) -> String {
    let ks = sound_wavenumber(1.0, 1.0);
    let eb = bogoliubov_energy(m.k, 1.0, 1.0);
    let e0 = free_energy(m.k);
    let s_amp = match kick {
        Kick::Phase => format!("{:.17e}", m.s_amp(1.0, eps)),
        Kick::Density => "null".to_string(),
    };
    let mut s = String::new();
    write!(
        s,
        "{{\"k\":{:.17e},\"x\":{:.17e},\"dt\":{:.17e},\"n_steps\":{},\"box_l\":{:.17e},\
         \"gs_residual\":{:.6e},\"omega_zc\":{:.17e},\"omega_fit\":{:.17e},\"amplitude\":{:.17e},\
         \"n_crossings\":{},\"eps0\":{:.17e},\"eps_B_g1\":{:.17e},\"rel_err_vs_eps_B_g1\":{:.6e},\
         \"rel_err_vs_eps0\":{:.6e},\"S_freq\":{:.17e},\"S_amp\":{},\"S_B_g1\":{:.17e},\
         \"bound\":{:.17e}{}}}",
        m.k,
        m.k / ks,
        m.dt,
        m.times.len() - 1,
        m.box_l,
        m.ground_state_residual,
        m.omega_zero_crossing,
        m.omega_fit,
        m.amplitude,
        m.n_crossings,
        e0,
        eb,
        (m.omega_zero_crossing - eb) / eb,
        (m.omega_zero_crossing - e0) / e0,
        m.s_freq(),
        s_amp,
        bogoliubov_s(m.k, 1.0, 1.0),
        dual_scale_bound(m.k, ks),
        extra
    )
    .unwrap();
    s
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .expect("usage: bogoliubov_dispersion OUT.json");
    let ks = sound_wavenumber(1.0, 1.0);
    let xs: Vec<f64> = (0..25)
        .map(|i| 0.05 * (100f64).powf(i as f64 / 24.0))
        .collect();
    let mut json = String::from("{\n");
    writeln!(
        json,
        "\"units\":\"hbar=m=1, n0=1; g=1 gives c=1, k_s=2mc/hbar={ks}, xi=1/sqrt2\",\n\
         \"grid_n\":{N},\"periods\":{PERIODS},\"steps_per_period\":{SPP},\"eps\":{EPS},\
         \"dt_safety\":{SAFETY},"
    )
    .unwrap();

    let base = |g: f64, kick: Kick, safety: Option<f64>| Run {
        g,
        kick,
        spp: SPP,
        eps: EPS,
        mode: (2, 0),
        safety,
        splitting: Splitting::Strang,
    };
    let sweeps = [
        ("sweep_phase_g1", base(1.0, Kick::Phase, Some(SAFETY))),
        ("sweep_density_g1", base(1.0, Kick::Density, Some(SAFETY))),
        ("sweep_phase_g0", base(0.0, Kick::Phase, Some(SAFETY))),
        (
            "sweep_phase_g1_reference",
            Run {
                splitting: Splitting::PythonReference,
                ..base(1.0, Kick::Phase, Some(SAFETY))
            },
        ),
        ("sweep_phase_g1_uncapped", base(1.0, Kick::Phase, None)),
    ];
    for (name, run) in &sweeps {
        let rows: Vec<String> = xs
            .iter()
            .map(|&x| row(&measure(&cfg(x * ks, run)), run.eps, run.kick, ""))
            .collect();
        writeln!(json, "\"{name}\":[\n{}\n],", rows.join(",\n")).unwrap();
        eprintln!("{name}: {} rows", rows.len());
    }

    // dt refinement: spp = 1 makes the max_dt cap the binding rule, so dt = max_dt(grid, s).
    let conv_eps = 1e-4;
    for (name, xs_c, splitting) in [
        ("dt_convergence", vec![0.25, 1.0, 4.0], Splitting::Strang),
        (
            "dt_convergence_reference",
            vec![1.0],
            Splitting::PythonReference,
        ),
    ] {
        let mut conv = Vec::new();
        for &x in &xs_c {
            for &s in &[1.0, 0.5, 0.25, 0.125, 0.0625] {
                let run = Run {
                    spp: 1,
                    eps: conv_eps,
                    splitting,
                    ..base(1.0, Kick::Phase, Some(s))
                };
                let m = measure(&cfg(x * ks, &run));
                conv.push(row(&m, conv_eps, Kick::Phase, &format!(",\"safety\":{s}")));
            }
        }
        writeln!(json, "\"{name}\":[\n{}\n],", conv.join(",\n")).unwrap();
        eprintln!("{name}: {} rows", conv.len());
    }

    let mut scan = Vec::new();
    for &eps in &[1e-3, 3e-3, 1e-2, 3e-2] {
        let run = Run {
            eps,
            ..base(1.0, Kick::Phase, Some(SAFETY))
        };
        let m = measure(&cfg(ks, &run));
        scan.push(row(&m, eps, Kick::Phase, &format!(",\"eps\":{eps}")));
    }
    writeln!(json, "\"eps_scan\":[\n{}\n],", scan.join(",\n")).unwrap();

    let mut diag = Vec::new();
    for &mode in &[(2i64, 0i64), (1, 1), (2, 2)] {
        let run = Run {
            mode,
            ..base(1.0, Kick::Phase, Some(SAFETY))
        };
        let m = measure(&cfg(ks, &run));
        diag.push(row(
            &m,
            EPS,
            Kick::Phase,
            &format!(",\"mode\":[{},{}]", mode.0, mode.1),
        ));
    }
    writeln!(json, "\"diagonal_mode\":[\n{}\n]", diag.join(",\n")).unwrap();
    json.push_str("}\n");
    std::fs::write(&out, json).expect("write output");
    eprintln!("wrote {out}");
}
