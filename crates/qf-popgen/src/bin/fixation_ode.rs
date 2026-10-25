//! Preregistered-style experiment matrix for `qf-popgen` (Tier X): CVODE absorption probabilities
//! of Moran and Wright-Fisher chains against the closed forms kernel-checked in Lean, with
//! positive and negative controls. Usage: `fixation_ode OUT_DIR`.
//!
//! Writes `OUT_DIR/results.jsonl` (one JSON object per run) and `OUT_DIR/summary.json`.

use qf_popgen::{
    CertError, Certificate, Chain, Settings, Tolerances, certify_absorption, flipped_fix,
    identity_kernel, moran_fix, moran_kernel, wright_fisher_kernel,
};
use std::fmt::Write as _;
use std::fs;

struct Row {
    json: String,
    ok: bool,
}

fn trace_json(c: &Certificate) -> String {
    let mut s = String::from("[");
    for (k, (t, pn, int)) in c.trace.iter().enumerate() {
        if k > 0 {
            s.push(',');
        }
        let _ = write!(s, "[{t:e},{pn:.17e},{int:.6e}]");
    }
    s.push(']');
    s
}

/// JSON for the fitness field: a number, or `null` when the row has no meaningful fitness
/// (the identity-kernel control is not a Moran chain).
fn r_json(r: Option<f64>) -> String {
    r.map_or_else(|| "null".to_string(), |v| format!("{v}"))
}

fn accepted(
    kind: &str,
    label: &str,
    n: usize,
    r: Option<f64>,
    i: usize,
    target: f64,
    c: &Certificate,
) -> Row {
    Row {
        ok: true,
        json: format!(
            "{{\"kind\":\"{kind}\",\"label\":\"{label}\",\"N\":{n},\"r\":{},\"i\":{i},\"verdict\":\"accepted\",\"target\":{target:.17e},\"p_fix\":{:.17e},\"abs_err\":{:e},\"p_loss\":{:.17e},\"interior\":{:e},\"mass_drift\":{:e},\"min_component\":{:e},\"t_stop\":{:e},\"trace\":{}}}",
            r_json(r),
            c.p_fix,
            (c.p_fix - target).abs(),
            c.p_loss,
            c.interior,
            c.mass_drift,
            c.min_component,
            c.t_stop,
            trace_json(c)
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn refused(
    kind: &str,
    label: &str,
    n: usize,
    r: Option<f64>,
    i: usize,
    target: f64,
    e: &CertError,
    expect_mismatch: bool,
) -> Row {
    let reason = format!("{e}").replace('"', "'");
    let ok = match e {
        CertError::ValueMismatch { .. } => expect_mismatch,
        _ => !expect_mismatch,
    };
    Row {
        ok,
        json: format!(
            "{{\"kind\":\"{kind}\",\"label\":\"{label}\",\"N\":{n},\"r\":{},\"i\":{i},\"verdict\":\"refused\",\"target\":{target:.17e},\"reason\":\"{reason}\"}}",
            r_json(r)
        ),
    }
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .expect("usage: fixation_ode OUT_DIR");
    fs::create_dir_all(&out).expect("create OUT_DIR");
    let tol = Tolerances::default();
    let set = Settings::default();
    let mut rows: Vec<Row> = Vec::new();
    let mut failures = 0usize;

    // Positive controls and main matrix: every interior start of the Moran chain.
    let moran_cases: [(usize, f64, &str, f64); 7] = [
        (2, 2.0, "control_lean_N2_r2", 1e5),
        (3, 2.0, "control_lean_N3_r2", 1e5),
        (10, 1.5, "moran_N10_r1.5", 1e6),
        (10, 0.8, "moran_N10_r0.8", 1e6),
        (10, 1.0, "moran_N10_neutral", 1e6),
        (20, 1.2, "moran_N20_r1.2", 1e7),
        (30, 2.0, "moran_N30_r2", 1e7),
    ];
    for (n, r, label, t_max) in moran_cases {
        let chain = moran_kernel(n, r);
        for i in 1..n {
            let target = moran_fix(n, r, i);
            match certify_absorption(&chain, i, target, t_max, tol, set) {
                Ok(c) => rows.push(accepted("moran", label, n, Some(r), i, target, &c)),
                Err(e) => {
                    failures += 1;
                    rows.push(refused("moran", label, n, Some(r), i, target, &e, false));
                }
            }
        }
    }
    for (n, t_max) in [(10usize, 1e6), (20usize, 1e7)] {
        let chain = wright_fisher_kernel(n);
        for i in 1..n {
            let target = i as f64 / n as f64;
            match certify_absorption(&chain, i, target, t_max, tol, set) {
                Ok(c) => rows.push(accepted(
                    "wright_fisher",
                    "wf_neutral",
                    n,
                    Some(1.0),
                    i,
                    target,
                    &c,
                )),
                Err(e) => {
                    failures += 1;
                    rows.push(refused(
                        "wright_fisher",
                        "wf_neutral",
                        n,
                        Some(1.0),
                        i,
                        target,
                        &e,
                        false,
                    ));
                }
            }
        }
    }

    // Negative controls: each must be refused for the stated reason.
    let neg_chain = moran_kernel(10, 1.5);
    let mut negative = |label: &str,
                        chain: &Chain,
                        r: Option<f64>,
                        i: usize,
                        target: f64,
                        t_max: f64,
                        expect_mismatch: bool,
                        rows: &mut Vec<Row>| {
        match certify_absorption(chain, i, target, t_max, tol, set) {
            Ok(c) => {
                failures += 1;
                let mut row = accepted("negative_control", label, chain.n, r, i, target, &c);
                row.ok = false;
                rows.push(row);
            }
            Err(e) => {
                let row = refused(
                    "negative_control",
                    label,
                    chain.n,
                    r,
                    i,
                    target,
                    &e,
                    expect_mismatch,
                );
                if !row.ok {
                    failures += 1;
                }
                rows.push(row);
            }
        }
    };
    for i in [1usize, 3, 7] {
        negative(
            "flipped_exponent_formula",
            &neg_chain,
            Some(1.5),
            i,
            flipped_fix(10, 1.5, i),
            1e6,
            true,
            &mut rows,
        );
        negative(
            "neutral_i_over_N_under_selection",
            &neg_chain,
            Some(1.5),
            i,
            i as f64 / 10.0,
            1e6,
            true,
            &mut rows,
        );
    }
    negative(
        "identity_kernel_no_absorption",
        &identity_kernel(2),
        None,
        1,
        0.5,
        1e4,
        false,
        &mut rows,
    );
    negative(
        "horizon_too_short",
        &neg_chain,
        Some(1.5),
        3,
        moran_fix(10, 1.5, 3),
        4.0,
        false,
        &mut rows,
    );
    let mut bad = moran_kernel(10, 1.5);
    bad.p[4][4] += 1e-3;
    negative(
        "corrupted_row_sum",
        &bad,
        Some(1.5),
        3,
        moran_fix(10, 1.5, 3),
        1e6,
        false,
        &mut rows,
    );

    let mut jl = String::new();
    for r in &rows {
        jl.push_str(&r.json);
        jl.push('\n');
    }
    fs::write(format!("{out}/results.jsonl"), jl).expect("write results.jsonl");
    let accepted_n = rows
        .iter()
        .filter(|r| r.json.contains("\"verdict\":\"accepted\""))
        .count();
    let summary = format!(
        "{{\"rows\":{},\"accepted\":{},\"refused\":{},\"controls_or_runs_not_as_expected\":{},\"cvode_rtol\":{:e},\"cvode_atol\":{:e},\"tol_interior\":{:e},\"tol_value\":{:e}}}\n",
        rows.len(),
        accepted_n,
        rows.len() - accepted_n,
        failures,
        set.rtol,
        set.atol,
        tol.interior,
        tol.value
    );
    fs::write(format!("{out}/summary.json"), &summary).expect("write summary.json");
    print!("{summary}");
    if failures > 0 {
        std::process::exit(1);
    }
}
