//! Linear-response controls for `qf_gpe2d::linear_response` (small grids so they run quickly in
//! the debug profile; the full sweep lives in `examples/bogoliubov_dispersion.rs`).

use qf_gpe2d::Splitting;
use qf_gpe2d::linear_response::{
    Kick, ResponseConfig, bogoliubov_energy, bogoliubov_s, dual_scale_bound, free_energy, measure,
    sound_wavenumber,
};

fn cfg(x: f64, g: f64, safety: f64, splitting: Splitting, eps: f64) -> ResponseConfig {
    let k = x * sound_wavenumber(1.0, 1.0);
    ResponseConfig {
        k,
        mode: (2, 0),
        n: 16,
        g,
        n0: 1.0,
        eps,
        steps_per_period: 1,
        periods: 3,
        omega_ref: bogoliubov_energy(k, g, 1.0),
        kick: Kick::Phase,
        dt_safety: Some(safety),
        splitting,
    }
}

#[test]
fn positive_control_strang_reproduces_bogoliubov_and_the_f_sum_amplitude() {
    let m = measure(&cfg(1.0, 1.0, 0.5, Splitting::Strang, 1e-4));
    let eb = bogoliubov_energy(m.k, 1.0, 1.0);
    let rel = (m.omega_zero_crossing - eb).abs() / eb;
    assert!(rel < 1e-5, "omega rel err {rel:.3e}");
    let s_amp = m.s_amp(1.0, 1e-4);
    let sb = bogoliubov_s(m.k, 1.0, 1.0);
    assert!((s_amp - sb).abs() / sb < 1e-4, "S_amp {s_amp} vs S_B {sb}");
    assert!(m.ground_state_residual < 1e-10);
}

#[test]
fn positive_control_frequency_error_is_second_order_in_dt() {
    let e = |s: f64| {
        let m = measure(&cfg(1.0, 1.0, s, Splitting::Strang, 1e-4));
        let eb = bogoliubov_energy(m.k, 1.0, 1.0);
        (m.omega_zero_crossing - eb).abs() / eb
    };
    let ratio = e(1.0) / e(0.5);
    assert!(ratio > 3.5 && ratio < 4.5, "error ratio {ratio}");
}

#[test]
fn negative_control_g0_is_the_free_particle_and_fires_the_diffraction_test() {
    let m = measure(&cfg(0.25, 0.0, 0.5, Splitting::Strang, 1e-3));
    let e0 = free_energy(m.k);
    assert!((m.omega_zero_crossing - e0).abs() / e0 < 1e-9);
    // Far from the interacting (g = 1) Bogoliubov frequency at this k.
    let eb = bogoliubov_energy(m.k, 1.0, 1.0);
    assert!((m.omega_zero_crossing - eb).abs() / eb > 0.3);
    // S = 1 exceeds sqrt(k/2k_s) for k < 2 k_s: the test fires on the free value.
    let ks = sound_wavenumber(1.0, 1.0);
    assert!((m.s_freq() - 1.0).abs() < 1e-9);
    assert!(m.s_freq() > dual_scale_bound(m.k, ks));
}

#[test]
fn reference_splitting_amplitude_converges_only_at_first_order() {
    let amp_err = |s: f64| {
        let m = measure(&cfg(1.0, 1.0, s, Splitting::PythonReference, 1e-4));
        let sb = bogoliubov_s(m.k, 1.0, 1.0);
        (m.s_amp(1.0, 1e-4) - sb).abs() / sb
    };
    let (a, b) = (amp_err(1.0), amp_err(0.5));
    assert!(
        a > 1e-3,
        "reference amplitude error {a:.3e} unexpectedly small"
    );
    let ratio = a / b;
    assert!(ratio > 1.6 && ratio < 2.5, "first-order ratio {ratio}");
}

#[test]
fn measured_bogoliubov_s_stays_under_the_dual_scale_bound() {
    let ks = sound_wavenumber(1.0, 1.0);
    for &x in &[0.1, 0.5, 1.0, 2.0, 5.0] {
        let m = measure(&cfg(x, 1.0, 0.5, Splitting::Strang, 1e-4));
        let bound = dual_scale_bound(m.k, ks);
        assert!(
            m.s_freq() <= bound * (1.0 + 1e-6),
            "x={x}: S={} bound={bound}",
            m.s_freq()
        );
    }
}
