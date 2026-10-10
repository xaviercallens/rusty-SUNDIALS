//! The complexified dyadic shell cascade, a pure-Rust port of
//! `src/quantumfluids/w4_shell_model/{shell_dynamics,integrate}.py` in
//! [SocrateAI-Scientific-QuantumFluids](https://github.com/xaviercallens/SocrateAI-Scientific-QuantumFluids).
//!
//! # The model
//!
//! Shells `n = 0..=N`, wavenumbers `k_n = 2^n`, complex amplitudes `a_n`, boundary convention
//! `a_{-1} = a_{N+1} = 0`:
//!
//! ```text
//! da_n/dt = B_n(a) - nu k_n^2 a_n - i D k_n^2 a_n
//! ```
//!
//! where the nonlinearity is the **conjugated complexification**
//!
//! ```text
//! B_n(a) = k_{n-1} a_{n-1}^2 - k_n conj(a_n) a_{n+1}
//! ```
//!
//! This conserves `E = 1/2 sum |a_n|^2` exactly (to round-off) and, on real input, reduces
//! *exactly* to the real Katz–Pavlovic / Desnyansky–Novikov dyadic nonlinearity — the reals are
//! an invariant subspace at `D = 0`. The two linear terms share the same `k^2` structure with the
//! coefficient rotated 90 degrees in the complex plane: `-nu k^2 a` (real `nu`) is **dissipative**;
//! `-i D k^2 a` (real `D`) is **dispersive** and energy-neutral, at the cost of breaking the
//! reality invariance (a real state picks up an imaginary part under this term — not a defect,
//! the whole point of the dispersive regulator).
//!
//! # The cross-repo positive control
//!
//! This model was designed so that, at `D = 0` with real initial data, it reproduces
//! [MechanicaFluidorum](https://github.com/xaviercallens/SocrateAI-Scientific-MechanicaFluidorum)'s
//! independently written `exploration/dyadic_cascade.py` — a Numba index-loop implementation of
//! the plain real dyadic model — to within integrator round-off. Because `k_n = 2^n` are exact
//! powers of two, multiplying by them only shifts the IEEE754 exponent and leaves the mantissa
//! untouched, so the two implementations agree far more tightly than a generic shell spacing
//! would allow (QuantumFluids' own Python/Rust cross-check found `0.00` relative difference on
//! every tested configuration). [`tests/positive_control.rs`] reproduces that comparison here
//! against the reference CSV values (`N = 8`, `nu in {0.1, 0.01, 0.001}`, profiles `P1/P2/P3`).
//! This crate exists so QuantumFluids, MechanicaFluidorum, and any future stream needing this
//! model no longer maintain three parallel implementations of the same cascade.
//!
//! # Scope
//!
//! Tier B (unit-testable): no claim is made here about boundedness, blow-up or uniformity beyond
//! what the tests establish. The dispersive term's physical reading (BEC/GPE quantum pressure,
//! `D = hbar/2m`) does not model a roton minimum and has no counterpart in superfluid helium's
//! excitation spectrum.

use num_complex::Complex64;

/// Wavenumbers `k_n = 2^n` for `n = 0..=n_max`.
pub fn k_shells(n_max: usize) -> Vec<f64> {
    (0..=n_max).map(|n| 2f64.powi(n as i32)).collect()
}

fn check_shapes(a_len: usize, k_len: usize) {
    assert_eq!(
        a_len, k_len,
        "amplitude array length {a_len} does not match wavenumber array length {k_len} -- these must be equal (N+1)"
    );
}

/// The real dyadic (Katz–Pavlovic / Desnyansky–Novikov) nonlinearity, `B_n = k_{n-1} a_{n-1}^2 -
/// k_n a_n a_{n+1}`. Provided for cross-checking against the real model (MechanicaFluidorum's
/// reference) and as the `D = 0` positive control. Conserves `1/2 sum a_n^2` for real `a`.
pub fn nonlinear_real(a: &[f64], k: &[f64]) -> Vec<f64> {
    check_shapes(a.len(), k.len());
    let m = a.len();
    let mut out = vec![0.0; m];
    for n in 0..m {
        let a_nm1 = if n >= 1 { a[n - 1] } else { 0.0 };
        let k_nm1 = if n >= 1 { k[n - 1] } else { 0.0 };
        let a_np1 = if n + 1 < m { a[n + 1] } else { 0.0 };
        out[n] = k_nm1 * a_nm1 * a_nm1 - k[n] * a[n] * a_np1;
    }
    out
}

/// The conjugated complexification, `B_n = k_{n-1} a_{n-1}^2 - k_n conj(a_n) a_{n+1}`. Conserves
/// `E = 1/2 sum |a_n|^2` for complex `a`, and coincides with [`nonlinear_real`] on real input
/// (conjugation is a no-op there). The single conjugation placement is what makes the energy
/// cancellation work — it is not unique, and this one is chosen for the invariant-subspace
/// property (moving or removing it breaks conservation; see the negative controls in the test
/// suite, ported unchanged from the Python original).
pub fn nonlinear_conj(a: &[Complex64], k: &[f64]) -> Vec<Complex64> {
    check_shapes(a.len(), k.len());
    let m = a.len();
    let zero = Complex64::new(0.0, 0.0);
    let mut out = vec![zero; m];
    for n in 0..m {
        let a_nm1 = if n >= 1 { a[n - 1] } else { zero };
        let k_nm1 = if n >= 1 { k[n - 1] } else { 0.0 };
        let a_np1 = if n + 1 < m { a[n + 1] } else { zero };
        out[n] = k_nm1 * a_nm1 * a_nm1 - k[n] * a[n].conj() * a_np1;
    }
    out
}

/// Dissipative regulator: `-nu k^2 a`. Real coefficient, removes energy.
pub fn viscous(a: &[Complex64], k: &[f64], nu: f64) -> Vec<Complex64> {
    check_shapes(a.len(), k.len());
    a.iter()
        .zip(k)
        .map(|(&an, &kn)| -nu * kn * kn * an)
        .collect()
}

/// Dispersive regulator (W4): `-i D k^2 a`. Imaginary coefficient, energy-neutral. `D = hbar/2m`
/// in the GPE reading. Always complex, even on real-valued input — dispersion takes you out of
/// the real subspace by design.
pub fn quantum_pressure(a: &[Complex64], k: &[f64], d: f64) -> Vec<Complex64> {
    check_shapes(a.len(), k.len());
    let i = Complex64::new(0.0, 1.0);
    a.iter()
        .zip(k)
        .map(|(&an, &kn)| -i * d * kn * kn * an)
        .collect()
}

/// Full right-hand side: conjugated nonlinearity plus both regulator terms. `nu = D = 0` gives
/// the inviscid, undispersed model, whose real subspace is where Katz–Pavlovic (2005) proves
/// finite-time blow-up of the *infinite* system (the O5 falsification trap; see
/// [`ShellRun`]/[`integrate`] for why the *truncated* system provably cannot blow up).
pub fn rhs(a: &[Complex64], k: &[f64], nu: f64, d: f64) -> Vec<Complex64> {
    let mut out = nonlinear_conj(a, k);
    if nu != 0.0 {
        let v = viscous(a, k, nu);
        for (o, vi) in out.iter_mut().zip(v) {
            *o += vi;
        }
    }
    if d != 0.0 {
        let q = quantum_pressure(a, k, d);
        for (o, qi) in out.iter_mut().zip(q) {
            *o += qi;
        }
    }
    out
}

/// `E = 1/2 sum |a_n|^2`. Conserved by the nonlinearity alone.
pub fn energy(a: &[Complex64]) -> f64 {
    0.5 * a.iter().map(|z| z.norm_sqr()).sum::<f64>()
}

/// `Omega_sum = 1/2 sum_n k_n^2 |a_n|^2` — enstrophy as conventionally defined.
pub fn enstrophy_sum(a: &[Complex64], k: &[f64]) -> f64 {
    check_shapes(a.len(), k.len());
    0.5 * a
        .iter()
        .zip(k)
        .map(|(z, &kn)| kn * kn * z.norm_sqr())
        .sum::<f64>()
}

/// `Omega_max = max_n 1/2 k_n^2 |a_n|^2` — the largest single-shell contribution. **Not**
/// enstrophy: MechanicaFluidorum's reference script writes this quantity under the name
/// `sup_Omega` while its own docstring defines the sum form (`docs/DEFECT_REPORT_MF_ENSTROPHY.md`
/// upstream). Both conventions are provided so the two can be compared rather than assumed equal.
pub fn enstrophy_max(a: &[Complex64], k: &[f64]) -> f64 {
    check_shapes(a.len(), k.len());
    a.iter()
        .zip(k)
        .map(|(z, &kn)| 0.5 * kn * kn * z.norm_sqr())
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Both enstrophy conventions in one call: `(sum, max)`.
pub fn enstrophy_both(a: &[Complex64], k: &[f64]) -> (f64, f64) {
    (enstrophy_sum(a, k), enstrophy_max(a, k))
}

/// `dE/dt = sum_n Re(conj(a_n) da_n)` — the diagnostic the conservation tests are built on.
pub fn energy_rate(a: &[Complex64], da: &[Complex64]) -> f64 {
    assert_eq!(
        a.len(),
        da.len(),
        "shape mismatch: a {} vs da {}",
        a.len(),
        da.len()
    );
    a.iter().zip(da).map(|(ai, dai)| (ai.conj() * dai).re).sum()
}

/// `dt = 0.1 / ((nu + D) k_N^2 + k_N)`, extended from MechanicaFluidorum's `dt = 0.1 / (nu k_N^2 +
/// k_N)` rule to include the dispersive term's stiffness `D k_N^2` (`|-i D k^2| = D k^2`, the same
/// order as the viscous term). At `D = 0` this reduces exactly to the MechanicaFluidorum rule,
/// required for the positive control to be a like-for-like comparison.
pub fn step_size(n_max: usize, nu: f64, d: f64) -> f64 {
    let k_n = 2f64.powi(n_max as i32);
    0.1 / ((nu + d) * k_n * k_n + k_n)
}

/// A named initial profile, matching MechanicaFluidorum's `make_profile` exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// `a_0 = 1`, all others zero.
    P1,
    /// `a_n = 2^{-n}`.
    P2,
    /// `a_0 = 1`, `a_1 = 0.5`, rest zero.
    P3,
}

impl Profile {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "P1" => Some(Profile::P1),
            "P2" => Some(Profile::P2),
            "P3" => Some(Profile::P3),
            _ => None,
        }
    }
}

/// Real-valued initial state for `profile` at truncation `n_max`. Real, not complex: at `D = 0`
/// the reals are an invariant subspace, and this is what exercises it.
pub fn make_profile(profile: Profile, n_max: usize) -> Vec<f64> {
    let mut a = vec![0.0; n_max + 1];
    match profile {
        Profile::P1 => a[0] = 1.0,
        Profile::P2 => {
            for (n, an) in a.iter_mut().enumerate() {
                *an = 2f64.powi(-(n as i32));
            }
        }
        Profile::P3 => {
            a[0] = 1.0;
            if n_max >= 1 {
                a[1] = 0.5;
            }
        }
    }
    a
}

/// Default time horizon, matching MechanicaFluidorum's reference sweep.
pub const T_HORIZON: f64 = 10.0;
/// `|a_n|` above this trips the divergence guard.
pub const DIVERGE_THRESHOLD: f64 = 1e12;

/// Outcome of one integration. Both enstrophy conventions are recorded, per the upstream design's
/// audit ruling O1: whether a fitted exponent depends on the choice is settled empirically.
#[derive(Debug, Clone)]
pub struct ShellRun {
    pub n_max: usize,
    pub nu: f64,
    pub d: f64,
    pub dt: f64,
    pub steps: u64,
    pub diverged: bool,
    pub sup_enstrophy_sum: f64,
    pub sup_enstrophy_max: f64,
    pub energy_initial: f64,
    pub energy_final: f64,
    pub t_stop: f64,
    pub max_abs_imag: f64,
    /// `sup` over `t` and `n` of `|a_n|`. Energy conservation bounds this by `sqrt(2E)` in the
    /// conservative case, which is why a truncated inviscid dyadic model cannot blow up.
    pub sup_abs_amplitude: f64,
    /// `(t, Omega_sum, Omega_max)` time series, recorded every `trace_every` steps when
    /// requested; `None` otherwise.
    pub trace: Option<Vec<(f64, f64, f64)>>,
}

/// Integrate the shell model with classical RK4 and a divergence guard.
///
/// `max_steps`, if given, makes the call return `Err` rather than silently truncating: a
/// `sup_Omega` measured over a dynamically meaningless sliver of the horizon is worse than no
/// number at all. `a0`, if given, overrides `profile` (its length must be `n_max + 1`; a complex
/// `a0` is honoured even at `D = 0`).
#[allow(clippy::too_many_arguments)]
pub fn integrate(
    n_max: usize,
    nu: f64,
    d: f64,
    profile: Profile,
    t_horizon: f64,
    dt: Option<f64>,
    max_steps: Option<u64>,
    trace_every: Option<u64>,
    a0: Option<&[Complex64]>,
) -> Result<ShellRun, String> {
    let k = k_shells(n_max);
    let mut a: Vec<Complex64> = match a0 {
        Some(a0) => {
            if a0.len() != n_max + 1 {
                return Err(format!(
                    "a0 has length {}, expected {}",
                    a0.len(),
                    n_max + 1
                ));
            }
            a0.to_vec()
        }
        None => make_profile(profile, n_max)
            .into_iter()
            .map(|x| Complex64::new(x, 0.0))
            .collect(),
    };

    let dt = dt.unwrap_or_else(|| step_size(n_max, nu, d));
    let steps_required = (t_horizon / dt).ceil() as u64;
    if let Some(cap) = max_steps
        && steps_required > cap
    {
        return Err(format!(
            "N={n_max}, nu={nu}, D={d} needs {steps_required} RK4 steps to reach t={t_horizon}, \
             exceeding max_steps={cap}. Refusing to run rather than report a sup_Omega measured \
             over a fraction of the horizon. Raise max_steps deliberately, or reduce N."
        ));
    }

    let e_initial = energy(&a);
    let mut sup_sum = enstrophy_sum(&a, &k);
    let mut sup_max = enstrophy_max(&a, &k);
    let mut max_abs_imag = a.iter().map(|z| z.im.abs()).fold(0.0, f64::max);
    let mut sup_abs = a.iter().map(|z| z.norm()).fold(0.0, f64::max);

    let mut trace: Option<Vec<(f64, f64, f64)>> =
        trace_every.map(|_| vec![(0.0, sup_sum, sup_max)]);

    let mut t = 0.0;
    let mut steps_done = 0u64;
    let mut diverged = false;

    for _ in 0..steps_required {
        let remaining = t_horizon - t;
        let h = if remaining >= dt { dt } else { remaining };
        if h <= 0.0 {
            break;
        }

        let k1 = rhs(&a, &k, nu, d);
        let a2: Vec<_> = a
            .iter()
            .zip(&k1)
            .map(|(ai, ki)| ai + 0.5 * h * ki)
            .collect();
        let k2 = rhs(&a2, &k, nu, d);
        let a3: Vec<_> = a
            .iter()
            .zip(&k2)
            .map(|(ai, ki)| ai + 0.5 * h * ki)
            .collect();
        let k3 = rhs(&a3, &k, nu, d);
        let a4: Vec<_> = a.iter().zip(&k3).map(|(ai, ki)| ai + h * ki).collect();
        let k4 = rhs(&a4, &k, nu, d);

        for n in 0..a.len() {
            a[n] += (h / 6.0) * (k1[n] + 2.0 * k2[n] + 2.0 * k3[n] + k4[n]);
        }

        steps_done += 1;
        t += h;

        let max_abs = a.iter().map(|z| z.norm()).fold(0.0, f64::max);
        if !a.iter().all(|z| z.re.is_finite() && z.im.is_finite()) || max_abs > DIVERGE_THRESHOLD {
            diverged = true;
            break;
        }

        sup_sum = sup_sum.max(enstrophy_sum(&a, &k));
        sup_max = sup_max.max(enstrophy_max(&a, &k));
        sup_abs = sup_abs.max(max_abs);
        max_abs_imag = max_abs_imag.max(a.iter().map(|z| z.im.abs()).fold(0.0, f64::max));
        if let (Some(every), Some(tr)) = (trace_every, trace.as_mut())
            && every != 0
            && steps_done.is_multiple_of(every)
        {
            tr.push((t, enstrophy_sum(&a, &k), enstrophy_max(&a, &k)));
        }
    }

    // A diverged state's energy is meaningless and its square overflows, so report NaN rather
    // than computing a number nobody should use.
    let e_final = if diverged { f64::NAN } else { energy(&a) };

    Ok(ShellRun {
        n_max,
        nu,
        d,
        dt,
        steps: steps_done,
        diverged,
        sup_enstrophy_sum: sup_sum,
        sup_enstrophy_max: sup_max,
        energy_initial: e_initial,
        energy_final: e_final,
        t_stop: t,
        max_abs_imag,
        sup_abs_amplitude: sup_abs,
        trace,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    // A small, deterministic PRNG (xorshift64*) so the Rust tests use the same *kind* of random
    // trials as the Python suite without depending on an external crate or matching its exact
    // stream (the Python tests fix a numpy seed; this crate has no numpy to match bit-for-bit, so
    // these tests establish the same properties on an independent random source instead).
    struct Xorshift64(u64);
    impl Xorshift64 {
        fn new(seed: u64) -> Self {
            Xorshift64(seed ^ 0x2545F4914F6CDD1D)
        }
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
        /// Standard normal via Box-Muller.
        fn normal(&mut self) -> f64 {
            let u1 = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            let u2 = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            (-2.0 * u1.max(1e-300).ln()).sqrt() * (2.0 * PI * u2).cos()
        }
    }

    const N: usize = 10;
    const N_TRIALS: usize = 200;
    const ROUNDOFF: f64 = 1e-8;

    fn random_complex(rng: &mut Xorshift64, n: usize) -> Vec<Complex64> {
        (0..n)
            .map(|_| Complex64::new(rng.normal(), rng.normal()))
            .collect()
    }
    fn random_real(rng: &mut Xorshift64, n: usize) -> Vec<f64> {
        (0..n).map(|_| rng.normal()).collect()
    }
    fn to_complex(a: &[f64]) -> Vec<Complex64> {
        a.iter().map(|&x| Complex64::new(x, 0.0)).collect()
    }

    #[test]
    fn k_shells_values() {
        assert_eq!(k_shells(4), vec![1.0, 2.0, 4.0, 8.0, 16.0]);
    }

    #[test]
    #[should_panic(expected = "does not match")]
    fn rejects_mismatched_array_lengths() {
        nonlinear_real(&[0.0; 5], &k_shells(7));
    }

    #[test]
    fn real_nonlinearity_conserves_energy() {
        let k = k_shells(N);
        let mut rng = Xorshift64::new(20260814);
        let mut worst = 0.0f64;
        for _ in 0..N_TRIALS {
            let a = random_real(&mut rng, N + 1);
            let da = nonlinear_real(&a, &k);
            let rate: f64 = a.iter().zip(&da).map(|(ai, dai)| ai * dai).sum();
            worst = worst.max(rate.abs());
        }
        assert!(
            worst < ROUNDOFF,
            "real nonlinearity failed to conserve energy: {worst:.3e}"
        );
    }

    #[test]
    fn conjugated_nonlinearity_conserves_energy() {
        let k = k_shells(N);
        let mut rng = Xorshift64::new(1);
        let mut worst = 0.0f64;
        for _ in 0..N_TRIALS {
            let a = random_complex(&mut rng, N + 1);
            let da = nonlinear_conj(&a, &k);
            worst = worst.max(energy_rate(&a, &da).abs());
        }
        assert!(
            worst < ROUNDOFF,
            "conjugated nonlinearity failed to conserve energy: {worst:.3e}"
        );
    }

    #[test]
    fn quantum_pressure_is_energy_neutral() {
        let k = k_shells(N);
        let mut rng = Xorshift64::new(2);
        let mut worst = 0.0f64;
        for _ in 0..N_TRIALS {
            let a = random_complex(&mut rng, N + 1);
            let da = quantum_pressure(&a, &k, 0.03);
            worst = worst.max(energy_rate(&a, &da).abs());
        }
        assert!(
            worst < ROUNDOFF,
            "quantum-pressure term is not energy-neutral: {worst:.3e}"
        );
    }

    // NEGATIVE CONTROLS -- the diagnostic must REJECT these. Ported unchanged from
    // tests/test_shell_dynamics.py: without them, the conservation assertions above could be
    // passing vacuously.

    fn nonlinear_naive(a: &[Complex64], k: &[f64]) -> Vec<Complex64> {
        let m = a.len();
        let zero = Complex64::new(0.0, 0.0);
        let mut out = vec![zero; m];
        for n in 0..m {
            let a_nm1 = if n >= 1 { a[n - 1] } else { zero };
            let k_nm1 = if n >= 1 { k[n - 1] } else { 0.0 };
            let a_np1 = if n + 1 < m { a[n + 1] } else { zero };
            out[n] = k_nm1 * a_nm1 * a_nm1 - k[n] * a[n] * a_np1; // no conjugate
        }
        out
    }

    fn nonlinear_signflip(a: &[Complex64], k: &[f64]) -> Vec<Complex64> {
        let m = a.len();
        let zero = Complex64::new(0.0, 0.0);
        let mut out = vec![zero; m];
        for n in 0..m {
            let a_nm1 = if n >= 1 { a[n - 1] } else { zero };
            let k_nm1 = if n >= 1 { k[n - 1] } else { 0.0 };
            let a_np1 = if n + 1 < m { a[n + 1] } else { zero };
            out[n] = k_nm1 * a_nm1 * a_nm1 + k[n] * a[n].conj() * a_np1; // sign flipped
        }
        out
    }

    fn nonlinear_conj_misplaced(a: &[Complex64], k: &[f64]) -> Vec<Complex64> {
        let m = a.len();
        let zero = Complex64::new(0.0, 0.0);
        let mut out = vec![zero; m];
        for n in 0..m {
            let a_nm1 = if n >= 1 { a[n - 1] } else { zero };
            let k_nm1 = if n >= 1 { k[n - 1] } else { 0.0 };
            let a_np1 = if n + 1 < m { a[n + 1] } else { zero };
            out[n] = k_nm1 * a_nm1.conj() * a_nm1.conj() - k[n] * a[n] * a_np1; // wrong factor
        }
        out
    }

    #[test]
    fn negative_controls_are_detected() {
        let k = k_shells(N);
        for (broken, label) in [
            (
                nonlinear_naive as fn(&[Complex64], &[f64]) -> Vec<Complex64>,
                "naive (no conjugate)",
            ),
            (nonlinear_signflip, "sign-flipped outgoing term"),
            (nonlinear_conj_misplaced, "conjugate on the wrong factor"),
        ] {
            let mut rng = Xorshift64::new(3);
            let mut worst = 0.0f64;
            for _ in 0..N_TRIALS {
                let a = random_complex(&mut rng, N + 1);
                let da = broken(&a, &k);
                worst = worst.max(energy_rate(&a, &da).abs());
            }
            assert!(
                worst > 1.0,
                "NEGATIVE CONTROL FAILED for {label}: max |dE/dt| = {worst:.3e}"
            );
        }
    }

    #[test]
    fn viscous_term_is_detected_as_dissipative() {
        let k = k_shells(N);
        let mut rng = Xorshift64::new(4);
        let mut worst_rate = f64::NEG_INFINITY;
        for _ in 0..N_TRIALS {
            let a = random_complex(&mut rng, N + 1);
            let da = viscous(&a, &k, 0.03);
            worst_rate = worst_rate.max(energy_rate(&a, &da));
        }
        assert!(
            worst_rate < 0.0,
            "viscous term did not register as dissipative: {worst_rate:.3e}"
        );
    }

    #[test]
    fn conjugated_reduces_exactly_to_real_model() {
        let k = k_shells(N);
        let mut rng = Xorshift64::new(5);
        for _ in 0..N_TRIALS {
            let a = random_real(&mut rng, N + 1);
            let real_out = nonlinear_real(&a, &k);
            let conj_out = nonlinear_conj(&to_complex(&a), &k);
            for (r, c) in real_out.iter().zip(&conj_out) {
                assert_eq!(
                    *r, c.re,
                    "conjugated nonlinearity does not reduce exactly to the real model"
                );
                assert_eq!(c.im, 0.0);
            }
        }
    }

    #[test]
    fn reals_are_invariant_subspace_at_d_zero() {
        let k = k_shells(N);
        let mut rng = Xorshift64::new(6);
        for _ in 0..50 {
            let a = to_complex(&random_real(&mut rng, N + 1));
            let out = rhs(&a, &k, 0.01, 0.0);
            assert!(
                out.iter().all(|z| z.im == 0.0),
                "reality-invariance broken at D=0"
            );
        }
    }

    #[test]
    fn dispersive_term_breaks_reality_invariance() {
        let k = k_shells(N);
        let mut rng = Xorshift64::new(7);
        let a = to_complex(&random_real(&mut rng, N + 1));
        let out = rhs(&a, &k, 0.0, 0.03);
        assert!(out.iter().any(|z| z.im.abs() > 0.0));
    }

    #[test]
    fn enstrophy_sum_and_max_reproduce_the_mf_discrepancy() {
        // On profile P2 (a_n = 2^-n) every shell contributes equally, so sum = (N+1)*max exactly.
        // At N=8 that is 4.5 vs 0.5 -- pins docs/DEFECT_REPORT_MF_ENSTROPHY.md's numbers.
        let m = 8;
        let k = k_shells(m);
        let a = to_complex(&make_profile(Profile::P2, m));
        let s = enstrophy_sum(&a, &k);
        let mx = enstrophy_max(&a, &k);
        assert!((s - 4.5).abs() < 1e-12);
        assert!((mx - 0.5).abs() < 1e-12);
        assert!((s / mx - (m + 1) as f64).abs() < 1e-9);
    }

    #[test]
    fn step_size_matches_mechanicafluidorum_rule_at_d_zero() {
        for (n, nu) in [(8usize, 0.1), (8, 0.01), (12, 0.001), (4, 1.0)] {
            let k_n = 2f64.powi(n as i32);
            let expected = 0.1 / (nu * k_n * k_n + k_n);
            assert!((step_size(n, nu, 0.0) - expected).abs() / expected < 1e-15);
        }
    }

    #[test]
    fn step_size_shrinks_when_d_added() {
        assert!(step_size(8, 0.01, 0.1) < step_size(8, 0.01, 0.0));
    }

    #[test]
    fn profile_p2_is_geometric() {
        let a = make_profile(Profile::P2, 4);
        let expected = [1.0, 0.5, 0.25, 0.125, 0.0625];
        for (x, e) in a.iter().zip(expected) {
            assert!((x - e).abs() < 1e-15);
        }
    }

    #[test]
    fn viscosity_dissipates_energy_under_integration() {
        let run = integrate(5, 0.5, 0.0, Profile::P2, 0.5, None, None, None, None).unwrap();
        assert!(!run.diverged);
        assert!(run.energy_final < run.energy_initial);
    }

    #[test]
    fn pure_dispersion_conserves_energy_under_integration() {
        let run = integrate(5, 0.0, 0.2, Profile::P2, 0.5, None, None, None, None).unwrap();
        assert!(!run.diverged);
        let rel_drift = (run.energy_final - run.energy_initial).abs() / run.energy_initial;
        assert!(
            rel_drift < 1e-6,
            "pure dispersion leaked energy: relative drift {rel_drift:.3e}"
        );
    }

    #[test]
    fn dispersive_energy_drift_shrinks_with_timestep() {
        let coarse =
            integrate(5, 0.0, 0.2, Profile::P2, 0.2, Some(1e-4), None, None, None).unwrap();
        let fine = integrate(5, 0.0, 0.2, Profile::P2, 0.2, Some(5e-5), None, None, None).unwrap();
        let d_coarse = (coarse.energy_final - coarse.energy_initial).abs();
        let d_fine = (fine.energy_final - fine.energy_initial).abs();
        assert!(
            d_fine < d_coarse,
            "energy drift did not shrink with dt: {d_coarse:.3e} -> {d_fine:.3e}"
        );
    }

    #[test]
    fn inviscid_undispersed_conserves_energy_over_short_horizon() {
        let run = integrate(5, 0.0, 0.0, Profile::P2, 0.2, None, None, None, None).unwrap();
        let rel = (run.energy_final - run.energy_initial).abs() / run.energy_initial;
        assert!(rel < 1e-8, "conservative model drifted by {rel:.3e}");
    }

    #[test]
    fn reality_preserved_through_integration_at_d_zero() {
        let run = integrate(5, 0.01, 0.0, Profile::P3, 0.3, None, None, None, None).unwrap();
        assert_eq!(run.max_abs_imag, 0.0);
    }

    #[test]
    fn dispersion_takes_the_state_out_of_the_reals() {
        let run = integrate(5, 0.0, 0.2, Profile::P3, 0.3, None, None, None, None).unwrap();
        assert!(run.max_abs_imag > 0.0);
    }

    #[test]
    fn both_enstrophy_conventions_genuinely_differ() {
        let run = integrate(6, 0.05, 0.0, Profile::P2, 0.3, None, None, None, None).unwrap();
        assert!(run.sup_enstrophy_sum > run.sup_enstrophy_max * 1.01);
    }

    #[test]
    fn max_steps_refuses_rather_than_truncating() {
        let err = integrate(
            16,
            0.1,
            0.0,
            Profile::P1,
            T_HORIZON,
            None,
            Some(1000),
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("Refusing to run"));
    }

    #[test]
    fn truncated_inviscid_model_stays_bounded() {
        let run = integrate(6, 0.0, 0.0, Profile::P3, 40.0, Some(1e-3), None, None, None).unwrap();
        assert!(
            !run.diverged,
            "truncated inviscid model diverged -- it cannot"
        );
        assert!((run.t_stop - 40.0).abs() < 1e-6);
    }

    #[test]
    fn amplitudes_respect_the_sqrt_2e_bound() {
        let run = integrate(6, 0.0, 0.0, Profile::P3, 40.0, Some(1e-3), None, None, None).unwrap();
        let bound = (2.0 * run.energy_initial).sqrt();
        assert!(run.sup_abs_amplitude <= bound * (1.0 + 1e-6));
    }

    #[test]
    fn divergence_guard_trips_on_numerical_instability() {
        let run = integrate(8, 0.0, 0.0, Profile::P1, 50.0, Some(0.5), None, None, None).unwrap();
        assert!(
            run.diverged,
            "an absurdly large dt did not trip the divergence guard"
        );
        assert!(run.t_stop < 50.0);
    }

    #[test]
    fn explicit_initial_state_overrides_profile() {
        let a0: Vec<Complex64> = make_profile(Profile::P3, 4)
            .iter()
            .enumerate()
            .map(|(n, &x)| Complex64::new(x, 0.0) * Complex64::from_polar(1.0, 0.7 * n as f64))
            .collect();
        let run = integrate(
            4,
            0.0,
            0.0,
            Profile::P1,
            0.2,
            None,
            None,
            Some(1),
            Some(&a0),
        )
        .unwrap();
        assert!((run.energy_initial - 0.625).abs() < 1e-12); // P3's energy, not P1's
        assert!(run.max_abs_imag > 0.0); // stayed complex at D=0
    }

    #[test]
    fn a0_shape_mismatch_rejected() {
        let a0 = vec![Complex64::new(0.0, 0.0); 3];
        let err =
            integrate(4, 0.1, 0.0, Profile::P1, 1.0, None, None, None, Some(&a0)).unwrap_err();
        assert!(err.contains("expected"));
    }
}
