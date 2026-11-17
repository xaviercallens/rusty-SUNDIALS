//! Negative controls for the accuracy checker.
//!
//! Background: an independent audit (Zenodo 10.5281/zenodo.24216837) showed
//! that this crate can pass a tolerance test while its true error (against a
//! validated enclosure of the exact solution) is several times larger than the
//! tolerance suggests. A check that can never fail proves nothing, so this file
//! shows that the checker `assess` ACCEPTS correct results and REJECTS
//! deliberately wrong ones. The controls test the checker and two ODEs; they do
//! not establish the solver's general correctness.
//!
//! Robertson reference at t = 40 (midpoints of a rigorous enclosure of radius
//! about 3e-18; source: papers/ph2_validated_robertson/data/enclosures.json in
//! https://github.com/xaviercallens/AutoevolveAI, Zenodo 10.5281/zenodo.24216837).
//!
//! (This run measures max ratio 5.56 with the default finite-difference Jacobian.)
//! FACTOR_ROBERTSON = 20. Where it comes from: the study measured the true
//! error of this solver at rtol 1e-8, atol 1e-11 (atol = 1e-3 * rtol), t = 40,
//! as error / (rtol*|y| + atol) = 5.46 (y1), 0.15 (y2), 13.74 (y3); the largest
//! is 13.74. A factor of 20 is the smallest round number above that, so the
//! positive control accepts what the study measured and does not pretend the
//! port is more accurate than that. FACTOR_DECAY = 10 is set from this run:
//! the decay positive control measures ratio 1.74 (abs err 6.4e-9), so 10 leaves
//! about 5x headroom while still rejecting the controls below.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

const FACTOR_ROBERTSON: f64 = 20.0;
const FACTOR_DECAY: f64 = 10.0;
const RTOL: f64 = 1e-8;
const ATOL: f64 = 1e-11;
const ROB_REF: [f64; 3] = [
    0.715_827_068_719_405,
    9.185_534_764_557_764e-6,
    0.284_163_745_745_830_37,
];

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Verdict {
    Accept,
    Reject,
}

/// Accept only if |y_num_i - y_ref_i| <= factor * (rtol*|y_ref_i| + atol) for
/// every i. Non-finite values and length mismatches are always rejected.
fn assess(y_num: &[f64], y_ref: &[f64], rtol: f64, atol: f64, factor: f64) -> Verdict {
    if y_num.len() != y_ref.len() || y_num.is_empty() {
        return Verdict::Reject;
    }
    for (a, b) in y_num.iter().zip(y_ref.iter()) {
        if !a.is_finite() || !b.is_finite() {
            return Verdict::Reject;
        }
        if (a - b).abs() > factor * (rtol * b.abs() + atol) {
            return Verdict::Reject;
        }
    }
    Verdict::Accept
}

/// Largest error / (rtol*|ref| + atol) over components (the "ratio").
fn max_ratio(y_num: &[f64], y_ref: &[f64], rtol: f64, atol: f64) -> f64 {
    let mut worst = 0.0_f64;
    for (a, b) in y_num.iter().zip(y_ref.iter()) {
        worst = worst.max((a - b).abs() / (rtol * b.abs() + atol));
    }
    worst
}

/// Largest absolute error over components.
fn max_abs_err(y_num: &[f64], y_ref: &[f64]) -> f64 {
    let mut worst = 0.0_f64;
    for (a, b) in y_num.iter().zip(y_ref.iter()) {
        worst = worst.max((a - b).abs());
    }
    worst
}

fn decay(rtol: f64, atol: f64) -> Vec<f64> {
    let rhs = |_t: f64, y: &[f64], yd: &mut [f64]| -> Result<(), String> {
        yd[0] = -y[0];
        Ok(())
    };
    let mut cv = Cvode::builder(Method::Bdf)
        .rtol(rtol)
        .atol(atol)
        .max_steps(1_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&[1.0]))
        .expect("build decay");
    let (_t, y) = cv.solve(1.0, Task::Normal).expect("solve decay");
    y.to_vec()
}

/// Robertson to t = 40 with rate constants (k1, k2, k3) = (0.04, 1e4, 3e7)
/// unless deliberately changed by the caller.
fn robertson(k1: f64, k3: f64) -> Vec<f64> {
    let rhs = move |_t: f64, y: &[f64], yd: &mut [f64]| -> Result<(), String> {
        yd[0] = -k1 * y[0] + 1e4 * y[1] * y[2];
        yd[1] = k1 * y[0] - 1e4 * y[1] * y[2] - k3 * y[1] * y[1];
        yd[2] = k3 * y[1] * y[1];
        Ok(())
    };
    let mut cv = Cvode::builder(Method::Bdf)
        .rtol(RTOL)
        .atol(ATOL)
        .max_steps(1_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&[1.0, 0.0, 0.0]))
        .expect("build robertson");
    let (_t, y) = cv.solve(40.0, Task::Normal).expect("solve robertson");
    y.to_vec()
}

fn exact_decay() -> [f64; 1] {
    [(-1.0_f64).exp()]
}

#[test]
fn decay_positive_control() {
    let y = decay(RTOL, ATOL);
    let r = exact_decay();
    let ratio = max_ratio(&y, &r, RTOL, ATOL);
    println!(
        "POS decay rtol=1e-8: abs err = {:e}, ratio = {ratio:.3}, factor = {FACTOR_DECAY}",
        max_abs_err(&y, &r)
    );
    assert_eq!(assess(&y, &r, RTOL, ATOL, FACTOR_DECAY), Verdict::Accept);
    assert!(max_abs_err(&y, &r) > 0.0, "zero error is not plausible");
}

#[test]
fn robertson_positive_control() {
    let y = robertson(0.04, 3e7);
    let ratio = max_ratio(&y, &ROB_REF, RTOL, ATOL);
    println!(
        "POS robertson t=40 rtol=1e-8: abs err = {:e}, ratio = {ratio:.3}, factor = {FACTOR_ROBERTSON}",
        max_abs_err(&y, &ROB_REF)
    );
    assert_eq!(
        assess(&y, &ROB_REF, RTOL, ATOL, FACTOR_ROBERTSON),
        Verdict::Accept
    );
}

fn robertson_broken(k1: f64, k3: f64, label: &str) {
    let good = robertson(0.04, 3e7);
    let accepted_err = max_abs_err(&good, &ROB_REF);
    let y = robertson(k1, k3);
    let err = max_abs_err(&y, &ROB_REF);
    println!(
        "NEG robertson {label}: abs err = {err:e} (accepted control {accepted_err:e}, x{:.1})",
        err / accepted_err
    );
    assert_eq!(
        assess(&y, &ROB_REF, RTOL, ATOL, FACTOR_ROBERTSON),
        Verdict::Reject
    );
    assert!(err >= 100.0 * accepted_err);
}

#[test]
fn negative_robertson_k3_changed() {
    robertson_broken(0.04, 3.3e7, "k3=3.3e7");
}

#[test]
fn negative_robertson_k1_changed() {
    robertson_broken(0.0401, 3e7, "k1=0.0401");
}

#[test]
fn negative_decay_loose_tolerance() {
    let accepted_err = max_abs_err(&decay(RTOL, ATOL), &exact_decay());
    let y = decay(1e-2, 1e-4);
    let r = exact_decay();
    let err = max_abs_err(&y, &r);
    println!(
        "NEG decay solved at rtol=1e-2, judged at 1e-8: abs err = {err:e} (accepted control {accepted_err:e}, x{:.1})",
        err / accepted_err
    );
    assert_eq!(assess(&y, &r, RTOL, ATOL, FACTOR_DECAY), Verdict::Reject);
    assert!(err >= 100.0 * accepted_err);
}

#[test]
fn negative_checker_input_mutation() {
    // Mutate the reference by 1e-6 relative: a previously accepted result must
    // now be rejected, because 1e-6 is far above factor*(rtol*|y|+atol).
    let y = decay(RTOL, ATOL);
    let r = exact_decay();
    let accepted_err = max_abs_err(&y, &r);
    assert_eq!(assess(&y, &r, RTOL, ATOL, FACTOR_DECAY), Verdict::Accept);
    let mutated = [r[0] * (1.0 + 1e-6)];
    let err = max_abs_err(&y, &mutated);
    println!(
        "NEG mutated reference (+1e-6 rel): abs err = {err:e} (accepted control {accepted_err:e}, x{:.1})",
        err / accepted_err
    );
    assert_eq!(
        assess(&y, &mutated, RTOL, ATOL, FACTOR_DECAY),
        Verdict::Reject
    );
    // A 1e-6 relative perturbation of exp(-1) is 3.6e-7, only about 56x the
    // accepted error 6.4e-9, so this control asserts 50x, not 100x.
    assert!(err >= 50.0 * accepted_err);
}

#[test]
fn checker_sanity() {
    assert_eq!(
        assess(&ROB_REF, &ROB_REF, RTOL, ATOL, FACTOR_ROBERTSON),
        Verdict::Accept
    );
    let nan = [f64::NAN, 0.0, 0.0];
    assert_eq!(
        assess(&nan, &ROB_REF, RTOL, ATOL, FACTOR_ROBERTSON),
        Verdict::Reject
    );
    let inf = [f64::INFINITY, 0.0, 0.0];
    assert_eq!(
        assess(&inf, &ROB_REF, RTOL, ATOL, FACTOR_ROBERTSON),
        Verdict::Reject
    );
    assert_eq!(
        assess(&[1.0], &ROB_REF, RTOL, ATOL, FACTOR_ROBERTSON),
        Verdict::Reject
    );
}
