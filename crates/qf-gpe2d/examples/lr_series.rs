//! Print the recorded density-response series `t A(t)` of one linear-response run, for
//! inspection and plotting.
//!
//! Run: `cargo run --release -p qf-gpe2d --example lr_series -- X G SPP KICK(phase|density) [SAFETY] [reference|strang]`

use qf_gpe2d::Splitting;
use qf_gpe2d::linear_response::{
    Kick, ResponseConfig, bogoliubov_energy, measure, sound_wavenumber,
};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let x: f64 = a[1].parse().expect("x");
    let g: f64 = a[2].parse().expect("g");
    let spp: usize = a[3].parse().expect("spp");
    let kick = if a[4] == "density" {
        Kick::Density
    } else {
        Kick::Phase
    };
    let k = x * sound_wavenumber(1.0, 1.0);
    let m = measure(&ResponseConfig {
        k,
        mode: (2, 0),
        n: 32,
        g,
        n0: 1.0,
        eps: 1e-3,
        steps_per_period: spp,
        periods: 6,
        omega_ref: bogoliubov_energy(k, g, 1.0),
        kick,
        dt_safety: a.get(5).map(|v| v.parse().expect("safety")),
        splitting: if a.get(6).map(String::as_str) == Some("reference") {
            Splitting::PythonReference
        } else {
            Splitting::Strang
        },
    });
    println!(
        "# k={} omega_zc={} omega_fit={} amp={} crossings={}",
        m.k, m.omega_zero_crossing, m.omega_fit, m.amplitude, m.n_crossings
    );
    for (t, v) in m.times.iter().zip(&m.series) {
        println!("{t:.10e} {v:.10e}");
    }
}
