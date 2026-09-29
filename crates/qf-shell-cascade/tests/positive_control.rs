//! Positive Control #1: at `D = 0` with real initial data, this crate must reproduce
//! MechanicaFluidorum's independently written `exploration/dyadic_cascade.py` to within
//! integrator round-off, because the two implementations integrate the same real dyadic
//! nonlinearity (a Numba index loop there, a vectorised-by-shell loop here).
//!
//! Reference values below are transcribed verbatim from that repository's own published
//! `data/dyadic_omega_sup.csv` (`N = 8`, `status = OK` rows only; `N >= 12` needs 1.7e7+ RK4
//! steps, a Numba-scale run not worth reproducing here to make the same point). The comparison is
//! against `sup_Omega` as the reference script's `_simulate` actually computes it — the MAX
//! convention, not the sum its own docstring defines (a discrepancy QuantumFluids' own Python
//! port already documented and reproduces via `enstrophy_max`, not `enstrophy_sum`).
//!
//! Why the agreement is tight rather than merely close: `k_n = 2^n` are exact powers of two, so
//! multiplying by them only shifts the IEEE754 exponent and leaves the mantissa untouched. A
//! shell spacing other than 2 would not reproduce this tightly, and this test would then need a
//! looser tolerance.

use num_complex::Complex64;
use qf_shell_cascade::{Profile, integrate};

/// `(nu, profile, mf_sup_omega_max, mf_energy_final)`, N = 8, D = 0, from MechanicaFluidorum's
/// `data/dyadic_omega_sup.csv`.
const REFERENCE: &[(f64, Profile, f64, f64)] = &[
    (0.1, Profile::P1, 0.5416451354760369, 0.002382018155692759),
    (0.1, Profile::P2, 0.8753841785180844, 0.0022060904845584814),
    (0.1, Profile::P3, 0.9011104739704103, 0.0020975826698934026),
    (0.01, Profile::P1, 6.662633302097537, 0.01146406572793716),
    (0.01, Profile::P2, 7.865239231572015, 0.010697621299224658),
    (0.01, Profile::P3, 9.804893760201573, 0.010343502262071154),
    (0.001, Profile::P1, 70.87959161476942, 0.017540018047529218),
    (0.001, Profile::P2, 73.51748113068476, 0.016451811179123255),
    (0.001, Profile::P3, 115.75863351411745, 0.01595711295041411),
];

#[test]
fn positive_control_1_matches_mechanicafluidorum_at_d_zero() {
    const N: usize = 8;
    let mut worst_om = 0.0f64;
    let mut worst_e = 0.0f64;

    for &(nu, profile, mf_sup_omega, mf_e_final) in REFERENCE {
        let run = integrate(N, nu, 0.0, profile, 10.0, None, None, None, None).unwrap();
        assert!(
            !run.diverged,
            "N={N} nu={nu} profile={profile:?} diverged unexpectedly"
        );

        let d_om = (run.sup_enstrophy_max - mf_sup_omega).abs() / mf_sup_omega.abs();
        let d_e = (run.energy_final - mf_e_final).abs() / mf_e_final.abs();
        worst_om = worst_om.max(d_om);
        worst_e = worst_e.max(d_e);

        assert!(
            d_om < 1e-9,
            "N={N} nu={nu} profile={profile:?}: sup_Omega relative diff {d_om:.3e} (ours \
             {}, MF {mf_sup_omega})",
            run.sup_enstrophy_max
        );
        assert!(
            d_e < 1e-9,
            "N={N} nu={nu} profile={profile:?}: E_final relative diff {d_e:.3e} (ours \
             {}, MF {mf_e_final})",
            run.energy_final
        );
    }

    println!(
        "POSITIVE CONTROL #1: worst relative difference sup_Omega {worst_om:.3e}  E_final {worst_e:.3e}"
    );
}

/// `nonlinear_real` on real `f64` arrays and `nonlinear_conj` on the same values promoted to
/// `Complex64` with a zero imaginary part must agree bit-for-bit on this reference initial state,
/// independent of the integration test above -- a second, narrower check of the exact-reduction
/// claim on real production data rather than random trials.
#[test]
fn exact_reduction_on_reference_profiles() {
    use qf_shell_cascade::{k_shells, make_profile, nonlinear_conj, nonlinear_real};
    let k = k_shells(8);
    for profile in [Profile::P1, Profile::P2, Profile::P3] {
        let a = make_profile(profile, 8);
        let real_out = nonlinear_real(&a, &k);
        let complex_a: Vec<Complex64> = a.iter().map(|&x| Complex64::new(x, 0.0)).collect();
        let conj_out = nonlinear_conj(&complex_a, &k);
        for (r, c) in real_out.iter().zip(&conj_out) {
            assert_eq!(*r, c.re);
            assert_eq!(c.im, 0.0);
        }
    }
}
