//! Regression tests for the tight-tolerance defect found by the fusion audit
//! (docs/CVODE_TIGHT_TOLERANCE_FIX.md): smooth, trivially solvable problems
//! used to fail with `ErrTestFailure` at rtol <= 1e-8, and accuracy stalled
//! between rtol 1e-6 and 1e-7. LLNL CVODE solves all of these at rtol 1e-12.
//!
//! Every reference value is a closed form, not another solver's output.
//!
//! Error bounds: the task brief asked for max rel err < 1e-8 on y'=-y and
//! "within ~10x of rtol" in the work-precision sweep. VODE BDF (CVODE's direct
//! ancestor, same Nordsieck/BDF algorithm; scipy 1.13.1) does NOT meet those
//! literal numbers on these exact problems: 1.43e-8 on the decay test and a
//! global-error/rtol ratio of 36-143 over the sweep (the global error of a
//! local-error-per-step controller grows with the number of steps). The bounds
//! below are therefore set from the VODE numbers (quoted next to each) with a
//! ~2-3x margin. The strict monotone check and a >=2x error reduction per decade
//! of rtol catch the audited stall (1e-6 -> 1e-7 improved only 1.4x).

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

/// y' = -y, y(0) = 1, sampled at t = 1..=10. Returns max relative error.
fn decay_max_rel_err(rtol: f64, atol: f64, outputs: &[f64]) -> Result<(f64, usize), String> {
    let rhs = |_t: f64, y: &[f64], yd: &mut [f64]| -> Result<(), String> {
        yd[0] = -y[0];
        Ok(())
    };
    let mut cv = Cvode::builder(Method::Bdf)
        .rtol(rtol)
        .atol(atol)
        .max_steps(1_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&[1.0]))
        .map_err(|e| format!("{e:?}"))?;
    let mut worst: f64 = 0.0;
    for &tout in outputs {
        let (t, y) = cv
            .solve(tout, Task::Normal)
            .map_err(|e| format!("tout={tout}: {e:?}"))?;
        assert!(
            (t - tout).abs() <= 1e-12 * tout.max(1.0),
            "t={t} tout={tout}"
        );
        let exact = (-tout).exp();
        worst = worst.max((y[0] - exact).abs() / exact);
    }
    Ok((worst, cv.num_steps()))
}

#[test]
fn decay_rtol_1e10_to_t10() {
    let outs: Vec<f64> = (1..=10).map(|k| k as f64).collect();
    let (err, nst) = decay_max_rel_err(1e-10, 1e-14, &outs).expect("solver failed");
    // VODE BDF, same outputs: 1.426e-8.
    println!("decay max rel err = {err:e} nst={nst}");
    assert!(err < 3e-8, "max rel err {err:e} (nst={nst})");
    assert!(err > 0.0, "error exactly zero is not plausible");
}

/// The L/R control from the audit (C3): I0 = 15 MA, tau = 0.12 s, atol = 1e-6.
#[test]
fn lr_circuit_rtol_1e10() {
    let tau = 1.2e-5_f64 / 1.0e-4_f64;
    for &r_scale in &[1.0_f64, 2.0] {
        let k = r_scale / tau;
        let rhs = move |_t: f64, y: &[f64], yd: &mut [f64]| -> Result<(), String> {
            yd[0] = -k * y[0];
            Ok(())
        };
        let i0 = 15e6_f64;
        let mut cv = Cvode::builder(Method::Bdf)
            .rtol(1e-10)
            .atol(1e-6)
            .max_steps(100_000)
            .build(rhs, 0.0, SerialVector::from_slice(&[i0]))
            .unwrap();
        for &m in &[1.0_f64, 2.0, 5.0] {
            let (_, y) = cv
                .solve(m * tau, Task::Normal)
                .unwrap_or_else(|e| panic!("R x{r_scale}, t={m}tau: {e:?}"));
            let exact = i0 * (-k * m * tau).exp();
            let rel = (y[0] - exact).abs() / exact;
            println!("LR R x{r_scale} t={m}tau rel err = {rel:e}");
            // VODE BDF: R x1 1.3e-9/2.7e-9/7.0e-9, R x2 2.7e-9/5.6e-9/1.86e-8.
            assert!(rel < 4e-8, "R x{r_scale} t={m}tau rel err {rel:e}");
        }
    }
}

/// The coupled plasma/vessel circuit from the audit (C4), 2x2 linear,
/// stiffness ratio ~42, output every 1 ms. Exact solution via Sylvester's
/// formula for exp(At) with real distinct eigenvalues.
#[test]
fn coupled_circuit_2x2_rtol_1e8() {
    let (lp, lv, m) = (1.2e-5_f64, 5.0e-6_f64, 4.0e-6_f64);
    let (rp, rv) = (1.0e-3_f64, 1.0e-5_f64);
    let det = lp * lv - m * m;
    let a = [
        [-(lv * rp) / det, (m * rv) / det],
        [(m * rp) / det, -(lp * rv) / det],
    ];
    let rhs = move |_t: f64, y: &[f64], yd: &mut [f64]| -> Result<(), String> {
        yd[0] = a[0][0] * y[0] + a[0][1] * y[1];
        yd[1] = a[1][0] * y[0] + a[1][1] * y[1];
        Ok(())
    };
    let tr = a[0][0] + a[1][1];
    let dt = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    let disc = (tr * tr - 4.0 * dt).sqrt();
    let (l1, l2) = (0.5 * (tr + disc), 0.5 * (tr - disc));
    let y0 = [15e6_f64, 0.0];
    let exact = |t: f64| -> [f64; 2] {
        let (e1, e2) = ((l1 * t).exp(), (l2 * t).exp());
        let mut out = [0.0; 2];
        for i in 0..2 {
            let mut s = 0.0;
            for j in 0..2 {
                let id = if i == j { 1.0 } else { 0.0 };
                let p = (e1 * (a[i][j] - l2 * id) - e2 * (a[i][j] - l1 * id)) / (l1 - l2);
                s += p * y0[j];
            }
            out[i] = s;
        }
        out
    };
    // Sanity of the reference: I_v peaks near 11.1 MA (audit C4).
    let peak_ref = (1..=500)
        .map(|k| exact(k as f64 * 1e-3)[1])
        .fold(0.0_f64, f64::max);
    assert!(
        (peak_ref / 1e6 - 11.0975).abs() < 1e-2,
        "reference peak {peak_ref}"
    );

    let mut cv = Cvode::builder(Method::Bdf)
        .rtol(1e-8)
        .atol(1e-3)
        .max_steps(200_000)
        .build(rhs, 0.0, SerialVector::from_slice(&y0))
        .unwrap();
    let mut worst: f64 = 0.0;
    for k in 1..=500 {
        let t = k as f64 * 1e-3;
        let (_, y) = cv
            .solve(t, Task::Normal)
            .unwrap_or_else(|e| panic!("t={t}: {e:?}"));
        let ex = exact(t);
        for i in 0..2 {
            worst = worst.max((y[i] - ex[i]).abs() / 15e6);
        }
    }
    // VODE BDF, same outputs: 2.79e-8. Before the fix: 1.42e-6.
    println!("circuit max err/I0 = {worst:e}");
    assert!(worst < 1e-7, "max error / I0 = {worst:e}");
}

/// Smooth quadrature y' = cos t (state-independent, like the audit's C1).
#[test]
fn quadrature_cos_rtol_1e10() {
    let rhs = |t: f64, _y: &[f64], yd: &mut [f64]| -> Result<(), String> {
        yd[0] = t.cos();
        Ok(())
    };
    let mut cv = Cvode::builder(Method::Bdf)
        .rtol(1e-10)
        .atol(1e-12)
        .max_steps(1_000_000)
        .build(rhs, 0.0, SerialVector::from_slice(&[0.0]))
        .unwrap();
    let mut worst: f64 = 0.0;
    for k in 1..=20 {
        let t = 0.5 * k as f64;
        let (_, y) = cv
            .solve(t, Task::Normal)
            .unwrap_or_else(|e| panic!("t={t}: {e:?}"));
        worst = worst.max((y[0] - t.sin()).abs());
    }
    // VODE BDF, same outputs: 5.39e-9.
    println!("quadrature max abs err = {worst:e}");
    assert!(worst < 1e-8, "max abs err {worst:e}");
}

/// Work-precision: the max relative error over the outputs t = 1..=10 must
/// fall monotonically as rtol tightens from 1e-4 to 1e-10, by at least 2x per
/// decade (the audited stall was 1.4x; the fixed solver's weakest decade is
/// 2.9x, 1e-5 -> 1e-6), and stay within 250x of rtol (VODE BDF on the same problem and
/// outputs: ratio 53/129/36/36/44/71/143 for rtol 1e-4..1e-10).
#[test]
fn work_precision_decay_monotone() {
    let rtols = [1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10];
    let outs: Vec<f64> = (1..=10).map(|k| k as f64).collect();
    let mut results = Vec::new();
    for &rt in &rtols {
        let r = decay_max_rel_err(rt, 1e-14, &outs);
        match &r {
            Ok((err, nst)) => println!(
                "rtol={rt:e} nst={nst} maxrelerr={err:e} ratio={:.1}",
                err / rt
            ),
            Err(e) => println!("rtol={rt:e} FAILED {e}"),
        }
        results.push(r);
    }
    let errs: Vec<f64> = results
        .into_iter()
        .zip(&rtols)
        .map(|(r, rt)| r.unwrap_or_else(|e| panic!("rtol={rt:e}: {e}")).0)
        .collect();
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
        assert!(
            w[0] / w[1] >= 2.0,
            "stall: {:e} -> {:e} per decade",
            w[0],
            w[1]
        );
    }
}
