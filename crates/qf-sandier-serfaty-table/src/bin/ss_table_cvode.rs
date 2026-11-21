//! Runs the CVODE cross-check of the certificate's table propagation and prints a JSON report.
//!
//! Usage: `ss_table_cvode [rtol atol]...` (default sweep: rtol 1e-8 .. 1e-14, atol = rtol * 1e-4).

use qf_sandier_serfaty_table::{
    Perturb, TableData, Tol, Verdict, check_global, check_local, global_drift, load_default,
};

type Check = fn(&TableData, Perturb, Tol) -> Result<Verdict, String>;

fn show(name: &str, tol: Tol, pert: Perturb, v: &Verdict, secs: f64) -> String {
    format!(
        "{{\"check\": \"{name}\", \"rtol\": {:e}, \"atol\": {:e}, \"b_rel\": {:e}, \"k_rel\": {:e}, \"nodes\": {}, \
         \"values\": {}, \"outside\": {}, \"outside_s1_s2_ps\": {:?}, \"max_abs_rel_pos_s1_s2_ps\": [{:.3e}, {:.3e}, {:.3e}], \
         \"max_excess_s1_s2_ps\": [{:.3e}, {:.3e}, {:.3e}], \"first_outside\": {}, \"seconds\": {:.1}}}",
        tol.rtol,
        tol.atol,
        pert.b_rel,
        pert.k_rel,
        v.nodes,
        v.values,
        v.outside,
        v.outside_by_group,
        v.max_abs_rel_pos[0],
        v.max_abs_rel_pos[1],
        v.max_abs_rel_pos[2],
        v.max_excess[0],
        v.max_excess[1],
        v.max_excess[2],
        v.first_outside
            .map_or("null".to_string(), |(n, c)| format!("[{n}, {c}]")),
        secs
    )
}

fn main() {
    let d = match load_default() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("data: {e}");
            std::process::exit(2);
        }
    };
    let args: Vec<f64> = std::env::args()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    let tols: Vec<Tol> = if args.len() >= 2 {
        args.chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| Tol {
                rtol: c[0],
                atol: c[1],
            })
            .collect()
    } else {
        [1e-8, 1e-10, 1e-12, 1e-13, 1e-14]
            .iter()
            .map(|&r| Tol {
                rtol: r,
                atol: r * 1e-4,
            })
            .collect()
    };
    let perts = [
        Perturb::default(),
        Perturb {
            b_rel: 1e-6,
            k_rel: 0.0,
        },
        Perturb {
            b_rel: 0.0,
            k_rel: 1e-6,
        },
    ];
    let mut lines = Vec::new();
    for tol in &tols {
        for pert in &perts {
            let checks: [(&str, Check); 2] = [("local", check_local), ("global", check_global)];
            for (name, f) in checks {
                let t0 = std::time::Instant::now();
                match f(&d, *pert, *tol) {
                    Ok(v) => lines.push(show(name, *tol, *pert, &v, t0.elapsed().as_secs_f64())),
                    Err(e) => lines.push(format!(
                        "{{\"check\": \"{name}\", \"rtol\": {:e}, \"b_rel\": {:e}, \"k_rel\": {:e}, \"error\": \"{}\"}}",
                        tol.rtol,
                        pert.b_rel,
                        pert.k_rel,
                        e.replace('"', "'")
                    )),
                }
                eprintln!("{}", lines.last().map(String::as_str).unwrap_or(""));
            }
        }
    }
    for w in tols.windows(2) {
        match global_drift(&d, w[0], w[1]) {
            Ok(dr) => lines.push(format!(
                "{{\"drift_between_rtol\": [{:e}, {:e}], \"max_abs_diff_s1_s2_ps\": [{:.3e}, {:.3e}, {:.3e}]}}",
                w[0].rtol, w[1].rtol, dr[0], dr[1], dr[2]
            )),
            Err(e) => lines.push(format!("{{\"drift_error\": \"{}\"}}", e.replace('"', "'"))),
        }
        eprintln!("{}", lines.last().map(String::as_str).unwrap_or(""));
    }
    println!("[\n{}\n]", lines.join(",\n"));
}
