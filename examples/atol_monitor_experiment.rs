//! Preregistered experiment for the atol-dominance monitor (issue 17).
//!
//! Rule, controls and success criteria: docs/verification/PREREG_ATOL_MONITOR_2026-11-19.md.
//! Each problem is solved twice (monitor off, monitor on) and the outputs are compared bit for bit.
//! The ten well-posed problems replicate the method, tolerances, initial values and output times of
//! the example files of the same name.

use cvode::{AtolMonitorReport, Cvode, CvodeError, Method, Task};
use nvector::SerialVector;

type Rhs = fn(f64, &[f64], &mut [f64]) -> Result<(), String>;

struct Problem {
    name: String,
    group: &'static str,
    method: Method,
    rtol: f64,
    atol: f64,
    max_order: Option<usize>,
    y0: Vec<f64>,
    times: Vec<f64>,
    rhs: Rhs,
    analytic_robertson_jac: bool,
}

struct Outcome {
    bits: Vec<u64>,
    steps: usize,
    nfe: usize,
    report: Option<AtolMonitorReport>,
}

fn run(p: &Problem, monitor: bool) -> Result<Outcome, CvodeError> {
    let mut b = Cvode::builder(p.method)
        .rtol(p.rtol)
        .atol(p.atol)
        .max_steps(50_000_000);
    if let Some(q) = p.max_order {
        b = b.max_order(q);
    }
    if p.analytic_robertson_jac {
        b = b.jacobian(|_t, y, j| {
            j.cols[0][0] = -0.04;
            j.cols[0][1] = 0.04;
            j.cols[0][2] = 0.0;
            j.cols[1][0] = 1e4 * y[2];
            j.cols[1][1] = -1e4 * y[2] - 6e7 * y[1];
            j.cols[1][2] = 6e7 * y[1];
            j.cols[2][0] = 1e4 * y[1];
            j.cols[2][1] = -1e4 * y[1];
            j.cols[2][2] = 0.0;
            Ok(())
        });
    }
    let mut s = b.build(p.rhs, 0.0, SerialVector::from_slice(&p.y0))?;
    if monitor {
        s.enable_atol_monitor();
    }
    let mut bits = Vec::new();
    for &tout in &p.times {
        let (_, y) = s.solve(tout, Task::Normal)?;
        bits.extend(y.iter().map(|v| v.to_bits()));
    }
    Ok(Outcome {
        bits,
        steps: s.num_steps(),
        nfe: s.num_rhs_evals(),
        report: s.atol_monitor_report(),
    })
}

fn robertson_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    f[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
    f[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
    f[2] = 3e7 * y[1] * y[1];
    Ok(())
}
fn lorenz_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    f[0] = 10.0 * (y[1] - y[0]);
    f[1] = y[0] * (28.0 - y[2]) - y[1];
    f[2] = y[0] * y[1] - (8.0 / 3.0) * y[2];
    Ok(())
}
fn hh_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    let (v, m, h, n) = (y[0], y[1], y[2], y[3]);
    f[0] = 10.0
        - 120.0 * m.powi(3) * h * (v - 50.0)
        - 36.0 * n.powi(4) * (v + 77.0)
        - 0.3 * (v + 54.387);
    f[1] = hh_am(v) * (1.0 - m) - hh_bm(v) * m;
    f[2] = hh_ah(v) * (1.0 - h) - hh_bh(v) * h;
    f[3] = hh_an(v) * (1.0 - n) - hh_bn(v) * n;
    Ok(())
}
fn hh_am(v: f64) -> f64 {
    let d = v + 40.0;
    if d.abs() < 1e-7 {
        1.0
    } else {
        0.1 * d / (1.0 - (-0.1 * d).exp())
    }
}
fn hh_bm(v: f64) -> f64 {
    4.0 * (-(v + 65.0) / 18.0).exp()
}
fn hh_ah(v: f64) -> f64 {
    0.07 * (-(v + 65.0) / 20.0).exp()
}
fn hh_bh(v: f64) -> f64 {
    1.0 / (1.0 + (-(v + 35.0) / 10.0).exp())
}
fn hh_an(v: f64) -> f64 {
    let d = v + 55.0;
    if d.abs() < 1e-7 {
        0.1
    } else {
        0.01 * d / (1.0 - (-0.1 * d).exp())
    }
}
fn hh_bn(v: f64) -> f64 {
    0.125 * (-(v + 65.0) / 80.0).exp()
}
fn sir_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    let inf = 0.3 * y[0] * y[1] / 1000.0;
    f[0] = -inf;
    f[1] = inf - 0.1 * y[1];
    f[2] = 0.1 * y[1];
    Ok(())
}
fn lv_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    f[0] = 1.5 * y[0] - y[0] * y[1];
    f[1] = y[0] * y[1] - 3.0 * y[1];
    Ok(())
}
fn hires_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    f[0] = -1.71 * y[0] + 0.43 * y[1] + 8.32 * y[2] + 0.0007;
    f[1] = 1.71 * y[0] - 8.75 * y[1];
    f[2] = -10.03 * y[2] + 0.43 * y[3] + 0.035 * y[4];
    f[3] = 8.32 * y[1] + 1.71 * y[2] - 1.12 * y[3];
    f[4] = -1.745 * y[4] + 0.43 * y[5] + 0.43 * y[6];
    f[5] = -280.0 * y[5] * y[7] + 0.69 * y[3] + 1.71 * y[4] - 0.43 * y[5] + 0.69 * y[6];
    f[6] = 280.0 * y[5] * y[7] - 1.81 * y[6];
    f[7] = -280.0 * y[5] * y[7] + 1.81 * y[6];
    Ok(())
}
fn dp_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    let g = 9.81;
    let (th1, w1, th2, w2) = (y[0], y[1], y[2], y[3]);
    let d = th1 - th2;
    let (cd, sd) = (d.cos(), d.sin());
    let den = 2.0 - cd * cd;
    f[0] = w1;
    f[1] = (-w1 * w1 * sd * cd + g * th2.sin() * cd - w2 * w2 * sd - 2.0 * g * th1.sin()) / den;
    f[2] = w2;
    f[3] = (w2 * w2 * sd * cd + 2.0 * (g * th1.sin() * cd + w1 * w1 * sd - g * th2.sin())) / den;
    Ok(())
}
fn rb_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    let (i1, i2, i3) = (0.5, 1.0, 2.0);
    f[0] = ((1.0 / i3) - (1.0 / i2)) * y[1] * y[2];
    f[1] = ((1.0 / i1) - (1.0 / i3)) * y[0] * y[2];
    f[2] = ((1.0 / i2) - (1.0 / i1)) * y[0] * y[1];
    Ok(())
}
fn rossler_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    f[0] = -y[1] - y[2];
    f[1] = y[0] + 0.2 * y[1];
    f[2] = 0.2 + y[2] * (y[0] - 5.7);
    Ok(())
}
fn fn_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    f[0] = y[0] - y[0].powi(3) / 3.0 - y[1] + 0.5;
    f[1] = (y[0] + 0.7 - 0.8 * y[1]) / 12.5;
    Ok(())
}
fn three_body_rhs(_t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String> {
    let (x1, y1, x2, y2, x3, y3) = (y[0], y[1], y[2], y[3], y[4], y[5]);
    let r12 = ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt().max(1e-10);
    let r13 = ((x1 - x3).powi(2) + (y1 - y3).powi(2)).sqrt().max(1e-10);
    let r23 = ((x2 - x3).powi(2) + (y2 - y3).powi(2)).sqrt().max(1e-10);
    let (r12_3, r13_3, r23_3) = (r12.powi(3), r13.powi(3), r23.powi(3));
    f[..6].copy_from_slice(&y[6..12]);
    f[6] = -(x1 - x2) / r12_3 - (x1 - x3) / r13_3;
    f[7] = -(y1 - y2) / r12_3 - (y1 - y3) / r13_3;
    f[8] = -(x2 - x1) / r12_3 - (x2 - x3) / r23_3;
    f[9] = -(y2 - y1) / r12_3 - (y2 - y3) / r23_3;
    f[10] = -(x3 - x1) / r13_3 - (x3 - x2) / r23_3;
    f[11] = -(y3 - y1) / r13_3 - (y3 - y2) / r23_3;
    Ok(())
}

fn grid(step: f64, n: usize) -> Vec<f64> {
    (1..=n).map(|i| step * i as f64).collect()
}

fn problems() -> Vec<Problem> {
    let v0 = -65.0;
    let hh_y0 = vec![
        v0,
        hh_am(v0) / (hh_am(v0) + hh_bm(v0)),
        hh_ah(v0) / (hh_ah(v0) + hh_bh(v0)),
        hh_an(v0) / (hh_an(v0) + hh_bn(v0)),
    ];
    let rob_times = vec![
        0.1, 0.4, 1.0, 4.0, 10.0, 40.0, 100.0, 400.0, 1000.0, 4000.0, 10000.0, 40000.0,
    ];
    let mk = |name: &str,
              group: &'static str,
              method: Method,
              rtol: f64,
              atol: f64,
              max_order: Option<usize>,
              y0: Vec<f64>,
              times: Vec<f64>,
              rhs: Rhs| Problem {
        name: name.to_string(),
        group,
        method,
        rtol,
        atol,
        max_order,
        y0,
        times,
        rhs,
        analytic_robertson_jac: false,
    };
    let mut v = Vec::new();
    for &(rtol, atol) in &[
        (1e-4, 1e-7),
        (1e-6, 1e-9),
        (1e-8, 1e-11),
        (1e-10, 1e-10),
        (1e-10, 1e-13),
    ] {
        let mut p = mk(
            &format!("robertson rtol={rtol:e} atol={atol:e}"),
            "positive",
            Method::Bdf,
            rtol,
            atol,
            None,
            vec![1.0, 0.0, 0.0],
            rob_times.clone(),
            robertson_rhs,
        );
        p.analytic_robertson_jac = true;
        v.push(p);
    }
    let mut p = mk(
        "robertson rtol=1e-4 atol=1e-13 (small atol)",
        "negative_robertson",
        Method::Bdf,
        1e-4,
        1e-13,
        None,
        vec![1.0, 0.0, 0.0],
        rob_times,
        robertson_rhs,
    );
    p.analytic_robertson_jac = true;
    v.push(p);
    let (r, a) = (1e-3, 1e-5);
    let neg = "negative";
    v.push(mk(
        "lorenz",
        neg,
        Method::Bdf,
        r,
        a,
        Some(1),
        vec![1.0, 1.0, 1.0],
        grid(0.25, 20),
        lorenz_rhs,
    ));
    v.push(mk(
        "hodgkin_huxley",
        neg,
        Method::Bdf,
        r,
        a,
        None,
        hh_y0,
        grid(0.5, 20),
        hh_rhs,
    ));
    v.push(mk(
        "sir_epidemic",
        neg,
        Method::Adams,
        r,
        a,
        None,
        vec![999.0, 1.0, 0.0],
        vec![5.0, 10.0, 20.0, 30.0, 50.0, 75.0, 100.0, 150.0, 200.0],
        sir_rhs,
    ));
    v.push(mk(
        "lotka_volterra",
        neg,
        Method::Adams,
        r,
        a,
        None,
        vec![10.0, 5.0],
        grid(0.5, 20),
        lv_rhs,
    ));
    v.push(mk(
        "hires",
        neg,
        Method::Bdf,
        r,
        1e-6,
        None,
        vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0057],
        vec![0.5, 1.0, 5.0, 10.0, 50.0, 100.0],
        hires_rhs,
    ));
    v.push(mk(
        "double_pendulum",
        neg,
        Method::Bdf,
        r,
        a,
        None,
        vec![1.5, 0.0, 1.0, 0.0],
        grid(0.25, 20),
        dp_rhs,
    ));
    v.push(mk(
        "rigid_body",
        neg,
        Method::Adams,
        r,
        a,
        None,
        vec![1.0, 0.0, 0.9],
        grid(0.5, 20),
        rb_rhs,
    ));
    v.push(mk(
        "rossler",
        neg,
        Method::Bdf,
        r,
        a,
        None,
        vec![1.0, 1.0, 0.0],
        grid(1.0, 20),
        rossler_rhs,
    ));
    v.push(mk(
        "fitzhugh_nagumo",
        neg,
        Method::Adams,
        r,
        a,
        None,
        vec![-1.0, 1.0],
        grid(5.0, 20),
        fn_rhs,
    ));
    v.push(mk(
        "three_body",
        neg,
        Method::Bdf,
        r,
        a,
        None,
        vec![
            -0.5, 0.0, 0.5, 0.0, 0.0, 0.866, 0.0, 0.5, 0.0, -0.5, -0.5, 0.0,
        ],
        grid(0.25, 20),
        three_body_rhs,
    ));
    v
}

fn main() {
    let mut ten_runs_flagged = 0usize;
    let mut ten_runs = 0usize;
    let (mut ten_hits, mut ten_cs, mut ten_atol_dom) = (0usize, 0usize, 0usize);
    let mut all_identical = true;
    println!(
        "RUN | group | status | steps | nfe | bit_identical | flagged_components | per-component hit/atol_dom/jac_stiff/monitored"
    );
    for p in problems() {
        let off = run(&p, false);
        let on = run(&p, true);
        let (off, on) = match (off, on) {
            (Ok(a), Ok(b)) => (a, b),
            (a, b) => {
                println!(
                    "{} | {} | BLOCKED (monitor off: {:?}; monitor on: {:?}) | - | - | - | - | -",
                    p.name,
                    p.group,
                    a.err(),
                    b.err()
                );
                continue;
            }
        };
        let identical = off.bits == on.bits && off.steps == on.steps && off.nfe == on.nfe;
        all_identical &= identical;
        let Some(rep) = on.report else {
            println!("{} | {} | monitor report missing", p.name, p.group);
            continue;
        };
        let comps: Vec<String> = rep
            .components
            .iter()
            .map(|c| {
                format!(
                    "{}/{}/{}/{}",
                    c.steps_hit, c.steps_atol_dominated, c.steps_jac_stiff, c.steps_monitored
                )
            })
            .collect();
        println!(
            "{} | {} | ok | {} | {} | {} | {:?} | {}",
            p.name,
            p.group,
            on.steps,
            on.nfe,
            identical,
            rep.flagged_components(),
            comps.join(" ")
        );
        if p.group == "negative" {
            ten_runs += 1;
            ten_runs_flagged += usize::from(rep.any_flagged());
            for c in &rep.components {
                ten_hits += c.steps_hit;
                ten_cs += c.steps_monitored;
                ten_atol_dom += c.steps_atol_dominated;
            }
        }
    }
    println!("SUMMARY well-posed runs: {ten_runs}, flagged runs: {ten_runs_flagged}");
    println!(
        "SUMMARY component-steps: monitored {ten_cs}, hit {ten_hits} (rate {:.6}), atol-dominated only {ten_atol_dom} (rate {:.6})",
        ten_hits as f64 / ten_cs.max(1) as f64,
        ten_atol_dom as f64 / ten_cs.max(1) as f64
    );
    println!("SUMMARY all runs bit-identical with monitor on: {all_identical}");
}
