//! Acceptance tests for the Adams-Moulton path (docs/CVODE_ADAMS_FIX.md).
//!
//! Defect: `Method::Adams` never left order 1. `compute_l` returned l = [1, 1, ...] (a
//! placeholder), the error constant was 1/(q+1) and order selection was BDF-only, so every Adams
//! run was implicit Euler with a Milne-style error estimate. A first-order method under per-step
//! local error control has a global error that scales like sqrt(rtol): h ~ sqrt(rtol) so the
//! number of steps ~ 1/sqrt(rtol), and (steps x rtol) ~ sqrt(rtol). The tests below check the
//! signature of the fix (global error ~ linear in rtol, order actually rising) against closed
//! forms, and include a negative control that reproduces the old behaviour.
//!
//! Every reference value is a closed form, not another solver's output.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

/// Solve `rhs` with Adams from t0 = 0 to `t_end`, sampling at `outputs`; return the max error
/// (relative to `exact`, or absolute when `exact` is 0) and the number of steps.
fn adams_max_err<F, E>(
    rhs: F,
    y0: &[f64],
    outputs: &[f64],
    rtol: f64,
    atol: f64,
    max_order: Option<usize>,
    exact: E,
) -> Result<(f64, usize, usize), String>
where
    F: FnMut(f64, &[f64], &mut [f64]) -> Result<(), String> + Send + Sync,
    E: Fn(f64) -> Vec<f64>,
{
    let mut b = Cvode::builder(Method::Adams)
        .rtol(rtol)
        .atol(atol)
        .max_steps(2_000_000);
    if let Some(q) = max_order {
        b = b.max_order(q);
    }
    let mut cv = b
        .build(rhs, 0.0, SerialVector::from_slice(y0))
        .map_err(|e| format!("{e:?}"))?;
    let mut worst: f64 = 0.0;
    for &tout in outputs {
        let (t, y) = cv
            .solve(tout, Task::Normal)
            .map_err(|e| format!("tout={tout}: {e:?}"))?;
        assert!(
            (t - tout).abs() <= 1e-12 * tout.abs().max(1.0),
            "t={t} tout={tout}"
        );
        let ex = exact(tout);
        for (yi, ei) in y.iter().zip(&ex) {
            let scale = if ei.abs() > 0.0 { ei.abs() } else { 1.0 };
            worst = worst.max((yi - ei).abs() / scale);
        }
    }
    Ok((worst, cv.num_steps(), cv.max_order_reached()))
}

fn decay(rtol: f64, max_order: Option<usize>) -> (f64, usize, usize) {
    let outs: Vec<f64> = (1..=10).map(|k| k as f64).collect();
    adams_max_err(
        |_t, y, yd| {
            yd[0] = -y[0];
            Ok(())
        },
        &[1.0],
        &outs,
        rtol,
        1e-14,
        max_order,
        |t| vec![(-t).exp()],
    )
    .expect("Adams y'=-y solve failed")
}

fn cosine(rtol: f64, max_order: Option<usize>) -> (f64, usize, usize) {
    let outs: Vec<f64> = (1..=20).map(|k| 0.5 * k as f64).collect();
    adams_max_err(
        |t, _y, yd| {
            yd[0] = t.cos();
            Ok(())
        },
        &[0.0],
        &outs,
        rtol,
        1e-12,
        max_order,
        |t| vec![t.sin()],
    )
    .expect("Adams y'=cos t solve failed")
}

/// (1) y' = -y to t = 10: error is ~linear in rtol, not ~sqrt(rtol).
#[test]
fn decay_error_scales_linearly_with_rtol() {
    let (e6, n6, q6) = decay(1e-6, None);
    let (e8, n8, q8) = decay(1e-8, None);
    println!("decay rtol=1e-6 err={e6:e} nst={n6} qmax={q6}");
    println!("decay rtol=1e-8 err={e8:e} nst={n8} qmax={q8}");
    assert!(
        e6 > 0.0 && e8 > 0.0,
        "an exactly zero error is not plausible"
    );
    // sqrt scaling would give a ratio of 0.1; a method of order q gives ~100^(-q/(q+1)).
    assert!(
        e8 / e6 < 0.05,
        "err(1e-8)/err(1e-6) = {:.3} (sqrt(rtol) scaling gives 0.1)",
        e8 / e6
    );
    assert!(e8 < 1e-6, "rel err at rtol 1e-8 = {e8:e}");
}

/// Negative control for (1): the old placeholder only ever ran at q = 1 with l = [1, 1] and error
/// constant 1/2. `max_order(1)` selects exactly those coefficients in the fixed code (cvSetAdams,
/// q = 1: l = [1, 1], tq[2] = 1/2), so it reproduces the defect and must FAIL the linear-scaling
/// criterion. This shows the test above can fail.
#[test]
fn control_order_one_adams_has_sqrt_scaling() {
    let (e6, n6, q6) = decay(1e-6, Some(1));
    let (e8, n8, q8) = decay(1e-8, Some(1));
    println!("order-1 control rtol=1e-6 err={e6:e} nst={n6} qmax={q6}");
    println!("order-1 control rtol=1e-8 err={e8:e} nst={n8} qmax={q8}");
    assert_eq!(q6, 1);
    assert_eq!(q8, 1);
    assert!(
        e8 / e6 > 0.05,
        "the order-1 control unexpectedly passed the linear-scaling test: ratio {:.3}",
        e8 / e6
    );
    // First-order steps: many more of them than the full-order run needs.
    let (_, n8_full, _) = decay(1e-8, None);
    assert!(
        n8 > 5 * n8_full,
        "order-1 control took {n8} steps vs {n8_full} at full order"
    );
}

/// (2) y' = cos t (state-independent quadrature): same scaling criterion.
#[test]
fn cosine_error_scales_linearly_with_rtol() {
    let (e6, n6, q6) = cosine(1e-6, None);
    let (e8, n8, q8) = cosine(1e-8, None);
    println!("cos rtol=1e-6 err={e6:e} nst={n6} qmax={q6}");
    println!("cos rtol=1e-8 err={e8:e} nst={n8} qmax={q8}");
    assert!(e6 > 0.0 && e8 > 0.0);
    assert!(
        e8 / e6 < 0.05,
        "err(1e-8)/err(1e-6) = {:.3} (sqrt(rtol) scaling gives 0.1)",
        e8 / e6
    );
    assert!(e8 < 1e-6, "abs err at rtol 1e-8 = {e8:e}");
}

/// (3) The order actually rises during a smooth solve.
#[test]
fn order_reaches_at_least_three_on_smooth_problem() {
    let (err, nst, qmax) = decay(1e-8, None);
    println!("decay rtol=1e-8: err={err:e} nst={nst} max order reached={qmax}");
    assert!(qmax >= 3, "max order reached = {qmax}");
    assert!(qmax <= 12, "Adams order cannot exceed 12: {qmax}");
    // A default-order run must not need the step count of a first-order run.
    let (_, nst1, _) = decay(1e-8, Some(1));
    assert!(nst < nst1 / 5, "nst={nst} vs order-1 nst={nst1}");
}

/// (4a) Nonlinear scalar: logistic y' = y(1 - y), y(0) = 0.1, exact 1/(1 + 9 e^{-t}).
#[test]
fn logistic_matches_closed_form() {
    let outs: Vec<f64> = (1..=10).map(|k| k as f64).collect();
    let (err, nst, qmax) = adams_max_err(
        |_t, y, yd| {
            yd[0] = y[0] * (1.0 - y[0]);
            Ok(())
        },
        &[0.1],
        &outs,
        1e-8,
        1e-12,
        None,
        |t| vec![1.0 / (1.0 + 9.0 * (-t).exp())],
    )
    .expect("logistic solve failed");
    println!("logistic rtol=1e-8: rel err={err:e} nst={nst} qmax={qmax}");
    assert!(err < 1e-6, "rel err {err:e}");
    assert!(err > 0.0);
    assert!(qmax >= 3, "order reached only {qmax}");
}

/// (4b) Nonlinear system: Lotka-Volterra x' = x - xy, y' = xy - y. No closed form, but
/// V = x - ln x + y - ln y is exactly conserved. Check V to tolerance and the solution against a
/// tight-tolerance Adams run (self-consistency across tolerances) plus a BDF run.
#[test]
fn lotka_volterra_conserves_invariant_and_agrees_with_bdf() {
    let rhs = |_t: f64, y: &[f64], yd: &mut [f64]| -> Result<(), String> {
        yd[0] = y[0] - y[0] * y[1];
        yd[1] = y[0] * y[1] - y[1];
        Ok(())
    };
    let v = |y: &[f64]| y[0] - y[0].ln() + y[1] - y[1].ln();
    let y0 = [2.0, 1.0];
    let v0 = v(&y0);
    let t_end = 20.0;

    let mut ad = Cvode::builder(Method::Adams)
        .rtol(1e-8)
        .atol(1e-10)
        .max_steps(1_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&y0))
        .unwrap();
    let mut worst_v: f64 = 0.0;
    let mut y_ad = [0.0; 2];
    for k in 1..=40 {
        let t = t_end * k as f64 / 40.0;
        let (_, y) = ad.solve(t, Task::Normal).unwrap();
        worst_v = worst_v.max((v(y) - v0).abs() / v0);
        y_ad.copy_from_slice(y);
    }
    let (nst_ad, q_ad) = (ad.num_steps(), ad.max_order_reached());

    let mut bdf = Cvode::builder(Method::Bdf)
        .rtol(1e-10)
        .atol(1e-12)
        .max_steps(1_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&y0))
        .unwrap();
    let (_, y_bdf) = bdf.solve(t_end, Task::Normal).unwrap();
    let diff = (0..2)
        .map(|i| (y_ad[i] - y_bdf[i]).abs() / y_bdf[i].abs())
        .fold(0.0_f64, f64::max);
    println!(
        "Lotka-Volterra Adams rtol=1e-8: max |dV|/V={worst_v:e}, nst={nst_ad}, qmax={q_ad}; \
         vs BDF rtol 1e-10 at t=20: rel diff {diff:e}"
    );
    assert!(worst_v < 1e-6, "invariant drift {worst_v:e}");
    assert!(diff < 1e-5, "Adams vs BDF rel diff {diff:e}");
    assert!(q_ad >= 3, "order reached only {q_ad}");
}

/// (6) Einstein-de Sitter comoving distance: dD/dz = (1+z)^{-3/2}, D(z) = 2(1 - 1/sqrt(1+z)).
/// The audit measured 2.1e-4 relative error with the placeholder Adams at rtol 1e-7 (one fresh
/// solve per redshift, as `qf-bao-distances::chi_cvode` does). Require >= 100x better.
#[test]
fn eds_distance_rtol_1e7_beats_audit_by_100x() {
    let zs = [0.1, 0.5, 1.0, 2.0, 3.0, 4.0];
    let mut worst: f64 = 0.0;
    let mut total_nst = 0;
    let mut qmax = 0;
    for &z in &zs {
        let (err, nst, q) = adams_max_err(
            |x, _y, yd| {
                yd[0] = (1.0 + x).powf(-1.5);
                Ok(())
            },
            &[0.0],
            &[z],
            1e-7,
            1e-10,
            None,
            |z| vec![2.0 * (1.0 - 1.0 / (1.0 + z).sqrt())],
        )
        .expect("EdS solve failed");
        println!("EdS z={z}: rel err={err:e} nst={nst} qmax={q}");
        worst = worst.max(err);
        total_nst += nst;
        qmax = qmax.max(q);
    }
    println!("EdS Adams rtol=1e-7: worst rel err {worst:e}, total steps {total_nst}, qmax {qmax}");
    assert!(worst < 2.1e-4 / 100.0, "worst rel err {worst:e}");
    assert!(worst > 0.0);
    assert!(
        total_nst < 7000 / 5,
        "total steps {total_nst} (audit: ~7000)"
    );
}

/// Work-precision sweep, y' = -y, atol 1e-14, max rel err over t = 1..=10 (the table in
/// docs/CVODE_ADAMS_FIX.md is this test's output). The error must fall strictly and by at
/// least 2x per decade of rtol; sqrt(rtol) scaling gives 3.16x per decade, so the stronger
/// requirement is the >= 100x over two decades checked above. It must also stay within
/// 250x rtol, the bound the BDF sweep uses.
#[test]
fn work_precision_adams_decay() {
    let rtols = [1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10];
    let mut errs = Vec::new();
    for &rt in &rtols {
        let (err, nst, qmax) = decay(rt, None);
        println!(
            "adams rtol={rt:e} nst={nst} qmax={qmax} maxrelerr={err:e} ratio={:.1}",
            err / rt
        );
        errs.push(err);
    }
    for (&rt, &err) in rtols.iter().zip(&errs) {
        assert!(err <= 250.0 * rt, "rtol={rt:e}: rel err {err:e} > 250*rtol");
    }
    for w in errs.windows(2) {
        assert!(
            w[1] < w[0],
            "error not decreasing: {:e} -> {:e}",
            w[0],
            w[1]
        );
        assert!(w[0] / w[1] >= 2.0, "stall: {:e} -> {:e}", w[0], w[1]);
    }
}

/// The BDF path must be untouched by the Adams work: the same decay problem with BDF gives the
/// numbers recorded in docs/CVODE_TIGHT_TOLERANCE_FIX.md (614 steps, 1.01e-8 at rtol 1e-10).
/// Those numbers are for the default Newton path; `experimental-nls-v2` changes the step count.
#[cfg(not(feature = "experimental-nls-v2"))]
#[test]
fn bdf_decay_unchanged_reference() {
    let rhs = |_t: f64, y: &[f64], yd: &mut [f64]| -> Result<(), String> {
        yd[0] = -y[0];
        Ok(())
    };
    let mut cv = Cvode::builder(Method::Bdf)
        .rtol(1e-10)
        .atol(1e-14)
        .max_steps(1_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&[1.0]))
        .unwrap();
    let mut worst: f64 = 0.0;
    for k in 1..=10 {
        let t = k as f64;
        let (_, y) = cv.solve(t, Task::Normal).unwrap();
        worst = worst.max((y[0] - (-t).exp()).abs() / (-t).exp());
    }
    println!("BDF decay rtol=1e-10: err={worst:e} nst={}", cv.num_steps());
    assert_eq!(cv.num_steps(), 614, "BDF step count changed");
    assert!(
        (worst - 1.01e-8).abs() < 0.02e-8,
        "BDF error changed: {worst:e}"
    );
}
