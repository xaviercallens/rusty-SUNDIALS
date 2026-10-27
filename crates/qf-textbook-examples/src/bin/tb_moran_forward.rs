//! Appendix F, example 7b: Moran fixation from the Kolmogorov forward equation, with CVODE (BDF).
//!
//! `d pi / dt = pi (P - I)`, `pi(0) = e_i`, integrated until the interior mass is below 1e-12.
//! Positive controls: `N = 3, r = 2, i = 1` gives 4/7; `N = 10`, `r = 0.8, 1, 1.5`, every `i`,
//! match `(1 - r^-i) / (1 - r^-N)`. Negative controls: the identity kernel never absorbs; the
//! inverted-exponent formula `(1 - r^i) / (1 - r^N)` is refused.
//!
//! Usage: `tb_moran_forward [OUT_DIR]` (writes `moran_trace_N3.csv` and `moran_N10.csv` there).

use qf_textbook_examples::{absorb, moran_exact, moran_kernel};
use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&out).expect("create OUT_DIR");
    let mut ok = true;

    let a = absorb(&moran_kernel(3, 2.0), 1, 1e-12, 1e6).expect("CVODE run");
    let err = (a.p_fix - 4.0 / 7.0).abs();
    ok &= err < 1e-9 && a.mass_defect < 1e-10;
    println!(
        "N=3 r=2 i=1: p_fix = {:.16} |p_fix - 4/7| = {err:.2e} interior = {:.1e} t = {} steps = {}",
        a.p_fix, a.interior, a.t, a.steps
    );
    let mut csv = String::from("t,p_absorbed_N,interior\n");
    for (t, p, q) in &a.trace {
        let _ = writeln!(csv, "{t},{p:.17e},{q:.17e}");
    }
    std::fs::write(out.join("moran_trace_N3.csv"), csv).expect("write trace");

    let mut csv = String::from("r,i,p_cvode,p_exact\n");
    for r in [0.8, 1.0, 1.5] {
        let mut worst: f64 = 0.0;
        for i in 1..10 {
            let a = absorb(&moran_kernel(10, r), i, 1e-12, 1e7).expect("CVODE run");
            let ex = moran_exact(10, r, i);
            worst = worst.max((a.p_fix - ex).abs());
            let _ = writeln!(csv, "{r},{i},{:.17e},{ex:.17e}", a.p_fix);
        }
        ok &= worst < 1e-8;
        println!("N=10 r={r}: max_i |p_cvode - p_exact| = {worst:.2e}");
    }
    std::fs::write(out.join("moran_N10.csv"), csv).expect("write table");

    let id: Vec<Vec<f64>> = (0..4)
        .map(|i| (0..4).map(|j| f64::from(u8::from(i == j))).collect())
        .collect();
    let b = absorb(&id, 1, 1e-12, 1e3).expect("CVODE run");
    let refused = b.interior > 1e-9;
    ok &= refused;
    println!(
        "negative control, identity kernel: interior mass at t = {} is {:.3} -> refused (NotAbsorbed): {refused}",
        b.t, b.interior
    );
    let wrong = (1.0 - 2f64.powi(1)) / (1.0 - 2f64.powi(3));
    let refused = (a.p_fix - wrong).abs() > 1e-8;
    ok &= refused;
    println!(
        "negative control, inverted exponent (1-r^i)/(1-r^N) = {wrong:.6} vs CVODE {:.6} -> refused: {refused}",
        a.p_fix
    );
    println!("controls as expected: {ok}");
    std::process::exit(if ok { 0 } else { 1 });
}
