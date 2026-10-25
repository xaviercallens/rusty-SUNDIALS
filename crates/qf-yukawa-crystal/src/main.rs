//! Experiment driver for the preregistered study `results/preregistration.json` (YUK-TRI-1).
//!
//! `yukawa_crystal refs|controls|main|disk <crate_dir>` writes JSON results under `<crate_dir>/results/`.
//!
//! Exploratory additions (postdate the preregistration, written under `results/exploratory/`):
//! `yukawa_crystal global <crate_dir> [kappa] [nrun] [hops] [rounds]` runs basin hopping and parallel tempering from
//! the `uniform_*` starts of `main`; `yukawa_crystal floor <crate_dir> [kappa] [csv...]` checks configurations against
//! the exact triangular-lattice floor (`global_opt::floor_check`).

use qf_yukawa_crystal::*;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const FTOL: f64 = 1e-9;
const TMAX: f64 = 1e7;

fn tri_box(kappa: f64) -> System {
    let a = spacing_density_one();
    System {
        kappa,
        rc: 8.0,
        geometry: Geometry::Periodic {
            lx: 16.0 * a,
            ly: 9.0 * 3.0_f64.sqrt() * a,
        },
    }
}

fn square_box(kappa: f64) -> System {
    let l = 289.0_f64.sqrt();
    System {
        kappa,
        rc: 8.0,
        geometry: Geometry::Periodic { lx: l, ly: l },
    }
}

fn e_tri(kappa: f64) -> f64 {
    let a = spacing_density_one();
    let sys = System {
        kappa,
        rc: 8.0,
        geometry: Geometry::Bowl { k: 0.0 },
    };
    lattice_energy(&sys, [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]])
}

fn e_square(kappa: f64) -> f64 {
    let sys = System {
        kappa,
        rc: 8.0,
        geometry: Geometry::Bowl { k: 0.0 },
    };
    lattice_energy(&sys, [[1.0, 0.0], [0.0, 1.0]])
}

fn read_csv(p: &Path) -> Vec<f64> {
    fs::read_to_string(p)
        .unwrap_or_else(|e| panic!("read {p:?}: {e}"))
        .lines()
        .flat_map(|l| {
            l.split(',')
                .map(|v| v.trim().parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Lowest Hessian eigenvalues (after the two translation zero modes are included) for a periodic configuration.
fn hessian_summary(sys: &System, x: &[f64]) -> (f64, f64, f64, usize) {
    let m = x.len();
    let ev = symmetric_eigenvalues(&sys.hessian(x), m);
    let scale = ev[m - 1].abs();
    let neg = ev.iter().filter(|&&v| v < -1e-9 * scale).count();
    (ev[0], ev[1], ev[2], neg)
}

struct RunOut {
    json: String,
}

#[allow(clippy::too_many_arguments)]
fn one_run(
    label: &str,
    family: &str,
    sys: &System,
    x0: &[f64],
    eref: f64,
    anneal_it: bool,
    seed: u64,
    hess: bool,
) -> RunOut {
    let n = x0.len() / 2;
    let mut s = String::new();
    let quench = relax(sys, x0, FTOL, TMAX);
    let mut fields = format!(
        "\"label\":\"{label}\",\"family\":\"{family}\",\"n\":{n},\"kappa\":{},\"e_ref\":{eref:.15e}",
        sys.kappa
    );
    let record = |tag: &str, r: &Result<Relaxed, String>, fields: &mut String| match r {
        Ok(r) => {
            let e = sys.energy(&r.x) / n as f64;
            let o = order(sys, &r.x, None);
            let _ = write!(
                fields,
                ",\"{tag}\":{{\"e\":{e:.15e},\"gap\":{:.6e},\"rel_gap\":{:.6e},\"defects\":{},\"psi6_global\":{:.6},\"psi6_local\":{:.6},\"max_force\":{:.3e},\"t\":{:.3e},\"cvode_steps\":{},\"rhs_evals\":{},\"converged\":{}",
                e - eref,
                (e - eref) / eref.abs(),
                o.defects,
                o.psi6_global,
                o.psi6_local,
                r.max_force,
                r.t,
                r.steps,
                r.rhs_evals,
                r.converged
            );
            if hess {
                let (l0, l1, l2, neg) = hessian_summary(sys, &r.x);
                let _ = write!(
                    fields,
                    ",\"hess_lowest\":[{l0:.3e},{l1:.3e},{l2:.3e}],\"hess_negative\":{neg}"
                );
            }
            fields.push('}');
        }
        Err(e) => {
            let _ = write!(
                fields,
                ",\"{tag}\":{{\"error\":\"{}\"}}",
                e.replace('"', "'")
            );
        }
    };
    record("quench", &quench, &mut fields);
    if anneal_it {
        let xa = anneal(sys, x0, 0.05, 1e-5, 300_000, 0.01, 0.05, seed);
        let r = relax(sys, &xa, FTOL, TMAX);
        record("anneal_quench", &r, &mut fields);
    }
    s.push('{');
    s.push_str(&fields);
    s.push('}');
    RunOut { json: s }
}

/// A boxed job for [`parallel`].
type Job<T> = Box<dyn FnOnce() -> T + Send>;

fn parallel<T: Send>(jobs: Vec<Box<dyn FnOnce() -> T + Send>>, threads: usize) -> Vec<T> {
    let jobs = std::sync::Mutex::new(jobs.into_iter().enumerate().collect::<Vec<_>>());
    let out = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                loop {
                    let job = jobs.lock().unwrap().pop();
                    match job {
                        Some((i, f)) => {
                            let r = f();
                            out.lock().unwrap().push((i, r));
                        }
                        None => break,
                    }
                }
            });
        }
    });
    let mut v = out.into_inner().unwrap();
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, r)| r).collect()
}

fn write(dir: &Path, name: &str, lines: &[String]) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(name), lines.join("\n") + "\n").unwrap();
    println!("wrote {}", dir.join(name).display());
}

/// Exact triangular-floor check of a periodic configuration as a JSON object.
fn floor_json(sys: &System, x: &[f64], r_exact: f64) -> String {
    match global_opt::floor_check(sys, x, r_exact) {
        Ok(c) => format!(
            "{{\"r_exact\":{r_exact},\"density\":{:.12},\"e_exact\":{:.15e},\"e_floor\":{:.15e},\"margin\":{:.6e},\"tolerance\":{:.3e},\"tail_bound\":{:.3e},\"violated\":{}}}",
            c.density, c.e_config, c.e_floor, c.margin, c.tolerance, c.tail_bound, c.violated
        ),
        Err(e) => format!("{{\"error\":\"{}\"}}", e.replace('"', "'")),
    }
}

fn tri_jellium() -> jellium::Jellium {
    let a = spacing_density_one();
    jellium::Jellium::new(16.0 * a, 9.0 * 3.0_f64.sqrt() * a, 0.66, 8.0, 7.0)
}

fn square_jellium() -> jellium::Jellium {
    jellium::Jellium::new(17.0, 17.0, 0.66, 8.0, 7.0)
}

/// Literature triangular Madelung energy per particle at density 1 (Bonsall & Maradudin; see `jellium`).
fn e_tri_jellium() -> f64 {
    jellium::ETA_TRIANGULAR * std::f64::consts::PI.sqrt()
}

/// One jellium run (T = 0 quench, optional anneal + quench), JSON with the same fields as `one_run`.
/// The reference is the triangular box energy computed with the same Ewald parameters (it equals the literature value
/// to < 1e-8, `jellium::tests`); the anneal uses 100 000 steps (each force costs an Ewald k-sum).
fn jel_run(
    label: &str,
    family: &str,
    j: &jellium::Jellium,
    x0: &[f64],
    anneal_it: bool,
    seed: u64,
    hess: bool,
) -> String {
    let n = x0.len() / 2;
    let a = spacing_density_one();
    let eref = tri_jellium().energy(&triangular(16, 18, a)) / 288.0;
    let geo = j.geometry_system();
    let mut fields = format!(
        "\"label\":\"{label}\",\"family\":\"{family}\",\"n\":{n},\"model\":\"jellium_s1\",\"e_ref\":{eref:.15e},\"e_ref_literature\":{:.15e}",
        e_tri_jellium()
    );
    let record = |tag: &str, r: &Result<Relaxed, String>, fields: &mut String| match r {
        Ok(r) => {
            let e = j.energy(&r.x) / n as f64;
            let o = order(&geo, &r.x, None);
            let _ = write!(
                fields,
                ",\"{tag}\":{{\"e\":{e:.15e},\"gap\":{:.6e},\"rel_gap\":{:.6e},\"defects\":{},\"psi6_global\":{:.6},\"psi6_local\":{:.6},\"max_force\":{:.3e},\"t\":{:.3e},\"cvode_steps\":{},\"rhs_evals\":{},\"converged\":{}",
                e - eref,
                (e - eref) / eref.abs(),
                o.defects,
                o.psi6_global,
                o.psi6_local,
                r.max_force,
                r.t,
                r.steps,
                r.rhs_evals,
                r.converged
            );
            if hess {
                let m = r.x.len();
                let ev = symmetric_eigenvalues(&j.hessian_fd(&r.x, 1e-5), m);
                let scale = ev[m - 1].abs();
                let neg = ev.iter().filter(|&&v| v < -1e-9 * scale).count();
                let _ = write!(
                    fields,
                    ",\"hess_lowest\":[{:.3e},{:.3e},{:.3e}],\"hess_negative\":{neg}",
                    ev[0], ev[1], ev[2]
                );
            }
            fields.push('}');
        }
        Err(e) => {
            let _ = write!(
                fields,
                ",\"{tag}\":{{\"error\":\"{}\"}}",
                e.replace('"', "'")
            );
        }
    };
    record("quench", &j.relax(x0, FTOL, TMAX), &mut fields);
    if anneal_it {
        let xa = j.anneal(x0, 0.05, 1e-5, 100_000, 0.01, 0.05, seed);
        record("anneal_quench", &j.relax(&xa, FTOL, TMAX), &mut fields);
    }
    format!("{{{fields}}}")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("refs");
    let crate_dir = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| ".".into()));
    let res = crate_dir.join("results");
    let init = crate_dir.join("data/initial");
    // Worker threads: YUKAWA_THREADS if set, else 2 (the machine is shared; the owner asked for 2 cores overnight).
    let threads = std::env::var("YUKAWA_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&t| t > 0)
        .unwrap_or(2);
    let a = spacing_density_one();
    match cmd {
        "refs" => {
            let mut lines = Vec::new();
            for kappa in [1.0, 2.0] {
                let sys = tri_box(kappa);
                let x = triangular(16, 18, a);
                let e_box = sys.energy(&x) / 288.0;
                lines.push(format!(
                    "{{\"kappa\":{kappa},\"rc\":8.0,\"e_tri_sum\":{:.15e},\"e_tri_periodic_box\":{e_box:.15e},\"e_square_sum\":{:.15e},\"square_minus_tri\":{:.6e},\"truncation_v_rc\":{:.3e}}}",
                    e_tri(kappa), e_square(kappa), e_square(kappa) - e_tri(kappa), (-kappa * 8.0_f64).exp() / 8.0
                ));
            }
            write(&res, "references.jsonl", &lines);
        }
        "controls" => {
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            for kappa in [2.0, 1.0] {
                jobs.push(Box::new(move || {
                    let sys = tri_box(kappa);
                    one_run(
                        "ctl_pos_perfect",
                        "control",
                        &sys,
                        &triangular(16, 18, a),
                        e_tri(kappa),
                        false,
                        0,
                        true,
                    )
                    .json
                }));
                jobs.push(Box::new(move || {
                    let sys = tri_box(kappa);
                    let mut x = triangular(16, 18, a);
                    let mut rng = Rng::new(3);
                    x.iter_mut().for_each(|v| *v += 0.1 * a * rng.gauss());
                    one_run(
                        "ctl_pos_noisy",
                        "control",
                        &sys,
                        &x,
                        e_tri(kappa),
                        false,
                        0,
                        true,
                    )
                    .json
                }));
                jobs.push(Box::new(move || {
                    let sys = square_box(kappa);
                    let x = square(17, 1.0);
                    let n = 289.0;
                    let e_sq_box = sys.energy(&x) / n;
                    let (l0, l1, l2, neg) = hessian_summary(&sys, &x);
                    let mut xs = x.clone();
                    let mut rng = Rng::new(5);
                    xs.iter_mut().for_each(|v| *v += 1e-3 * rng.gauss());
                    let r = relax(&sys, &xs, FTOL, TMAX).unwrap();
                    let e_after = sys.energy(&r.x) / n;
                    let o = order(&sys, &r.x, None);
                    format!(
                        "{{\"label\":\"ctl_neg_square_saddle\",\"kappa\":{kappa},\"e_square_box\":{e_sq_box:.15e},\"e_square_sum\":{:.15e},\"hess_lowest\":[{l0:.3e},{l1:.3e},{l2:.3e}],\"hess_negative\":{neg},\"e_after_relax\":{e_after:.15e},\"lowered\":{},\"e_after_minus_e_tri\":{:.6e},\"defects_after\":{},\"psi6_global_after\":{:.6}}}",
                        e_square(kappa), e_after < e_sq_box, e_after - e_tri(kappa), o.defects, o.psi6_global
                    )
                }));
            }
            let lines = parallel(jobs, threads);
            write(&res, "controls.jsonl", &lines);
        }
        "main" => {
            let kappa: f64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(2.0);
            let nrun: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(20);
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            for k in 0..nrun {
                let p = init.join(format!("oc20_tri_box_{k:02}.csv"));
                jobs.push(Box::new(move || {
                    let sys = tri_box(kappa);
                    let x0 = read_csv(&p);
                    one_run(
                        &format!("oc20_{k:02}"),
                        "hf_oc20",
                        &sys,
                        &x0,
                        e_tri(kappa),
                        true,
                        100 + k as u64,
                        true,
                    )
                    .json
                }));
                jobs.push(Box::new(move || {
                    let sys = tri_box(kappa);
                    let x0 = uniform_config(&sys, 288, 0.5 * a, None, 500 + k as u64);
                    one_run(
                        &format!("uniform_{k:02}"),
                        "uniform",
                        &sys,
                        &x0,
                        e_tri(kappa),
                        true,
                        900 + k as u64,
                        true,
                    )
                    .json
                }));
            }
            for k in 0..(nrun / 2).max(1) {
                let p = init.join(format!("oc20_square_box_{k:02}.csv"));
                jobs.push(Box::new(move || {
                    let sys = square_box(kappa);
                    let x0 = read_csv(&p);
                    one_run(
                        &format!("incommensurate_{k:02}"),
                        "ctl_neg_incommensurate_box",
                        &sys,
                        &x0,
                        e_tri(kappa),
                        true,
                        700 + k as u64,
                        false,
                    )
                    .json
                }));
            }
            let lines = parallel(jobs, threads);
            write(&res, &format!("main_kappa{kappa}.jsonl"), &lines);
        }
        "disk" => {
            let kappa = 2.0;
            let rdisk = (300.0 / std::f64::consts::PI).sqrt();
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            for k in 0..10usize {
                for (gname, geom) in [
                    ("open_bowl", Geometry::Bowl { k: 0.02 }),
                    (
                        "closed_wall",
                        Geometry::Wall {
                            radius: rdisk,
                            eps: 100.0,
                        },
                    ),
                ] {
                    let p = init.join(format!("oc20_disk_{k:02}.csv"));
                    jobs.push(Box::new(move || {
                        let sys = System { kappa, rc: 8.0, geometry: geom };
                        let x0 = read_csv(&p);
                        let xa = anneal(&sys, &x0, 0.05, 1e-5, 300_000, 0.01, 0.05, 300 + k as u64);
                        match relax(&sys, &xa, FTOL, TMAX) {
                            Ok(r) => {
                                let n = 300usize;
                                let (cx, cy) = ((0..n).map(|i| r.x[2 * i]).sum::<f64>() / n as f64, (0..n).map(|i| r.x[2 * i + 1]).sum::<f64>() / n as f64);
                                let rmax = (0..n).map(|i| ((r.x[2 * i] - cx).powi(2) + (r.x[2 * i + 1] - cy).powi(2)).sqrt()).fold(0.0, f64::max);
                                let all = order(&sys, &r.x, None);
                                let interior = order(&sys, &r.x, Some(rmax - 2.0 * all.d0));
                                format!(
                                    "{{\"label\":\"disk_{k:02}\",\"geometry\":\"{gname}\",\"n\":300,\"e_per_particle\":{:.12e},\"cluster_radius\":{rmax:.4},\"d0\":{:.5},\"interior_counted\":{},\"interior_defects\":{},\"interior_psi6_local\":{:.5},\"edge_defects\":{},\"edge_counted\":{},\"max_force\":{:.2e},\"converged\":{}}}",
                                    sys.energy(&r.x) / 300.0, all.d0, interior.counted, interior.defects, interior.psi6_local,
                                    all.defects - interior.defects, all.counted - interior.counted, r.max_force, r.converged
                                )
                            }
                            Err(e) => format!("{{\"label\":\"disk_{k:02}\",\"geometry\":\"{gname}\",\"error\":\"{}\"}}", e.replace('"', "'")),
                        }
                    }));
                }
            }
            let lines = parallel(jobs, threads);
            write(&res, "disk.jsonl", &lines);
        }
        "snapshot" => {
            // Re-run selected published runs with their exact seeds and parameters and write coordinates for figures.
            // Each line of snapshots.jsonl records the energy, so it can be compared with the published jsonl value.
            let out = res.join("figures_data");
            fs::create_dir_all(&out).unwrap();
            let save = |name: &str, x: &[f64]| {
                let s: String = (0..x.len() / 2)
                    .map(|i| format!("{:.12},{:.12}\n", x[2 * i], x[2 * i + 1]))
                    .collect();
                fs::write(out.join(format!("{name}.csv")), s).unwrap();
            };
            let mut jobs: Vec<Job<(String, Vec<f64>, f64)>> = Vec::new();
            let p0 = init.join("oc20_tri_box_00.csv");
            let p17 = init.join("oc20_tri_box_17.csv");
            let pd = init.join("oc20_disk_00.csv");
            jobs.push(Box::new(move || {
                let sys = tri_box(2.0);
                let x0 = read_csv(&p0);
                let e = sys.energy(&x0) / 288.0;
                ("yuk_oc20_00_initial".into(), x0, e)
            }));
            let p0b = init.join("oc20_tri_box_00.csv");
            jobs.push(Box::new(move || {
                let sys = tri_box(2.0);
                let r = relax(&sys, &read_csv(&p0b), FTOL, TMAX).unwrap();
                let e = sys.energy(&r.x) / 288.0;
                ("yuk_oc20_00_quench".into(), r.x, e)
            }));
            jobs.push(Box::new(move || {
                let sys = tri_box(2.0);
                let xa = anneal(&sys, &read_csv(&p17), 0.05, 1e-5, 300_000, 0.01, 0.05, 117);
                let r = relax(&sys, &xa, FTOL, TMAX).unwrap();
                let e = sys.energy(&r.x) / 288.0;
                ("yuk_oc20_17_anneal_quench".into(), r.x, e)
            }));
            jobs.push(Box::new(move || {
                let sys = square_box(2.0);
                let x = square(17, 1.0);
                let mut xs = x.clone();
                let mut rng = Rng::new(5);
                xs.iter_mut().for_each(|v| *v += 1e-3 * rng.gauss());
                let r = relax(&sys, &xs, FTOL, TMAX).unwrap();
                let e = sys.energy(&r.x) / 289.0;
                ("yuk_square_saddle_relaxed".into(), r.x, e)
            }));
            let rdisk = (300.0 / std::f64::consts::PI).sqrt();
            for (gname, geom) in [
                ("open_bowl", Geometry::Bowl { k: 0.02 }),
                (
                    "closed_wall",
                    Geometry::Wall {
                        radius: rdisk,
                        eps: 100.0,
                    },
                ),
            ] {
                let pd = pd.clone();
                jobs.push(Box::new(move || {
                    let sys = System {
                        kappa: 2.0,
                        rc: 8.0,
                        geometry: geom,
                    };
                    let xa = anneal(&sys, &read_csv(&pd), 0.05, 1e-5, 300_000, 0.01, 0.05, 300);
                    let r = relax(&sys, &xa, FTOL, TMAX).unwrap();
                    let e = sys.energy(&r.x) / 300.0;
                    (format!("disk_00_{gname}"), r.x, e)
                }));
            }
            let results = parallel(jobs, threads);
            let mut lines = Vec::new();
            for (name, x, e) in results {
                save(&name, &x);
                lines.push(format!(
                    "{{\"name\":\"{name}\",\"e_per_particle\":{e:.15e}}}"
                ));
            }
            write(&out, "snapshots.jsonl", &lines);
        }
        "jel-controls" => {
            let jres = res.join("jellium");
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            jobs.push(Box::new(move || {
                jel_run(
                    "ctl_pos_perfect",
                    "control",
                    &tri_jellium(),
                    &triangular(16, 18, a),
                    false,
                    0,
                    true,
                )
            }));
            jobs.push(Box::new(move || {
                let mut x = triangular(16, 18, a);
                let mut rng = Rng::new(3);
                x.iter_mut().for_each(|v| *v += 0.1 * a * rng.gauss());
                jel_run(
                    "ctl_pos_noisy",
                    "control",
                    &tri_jellium(),
                    &x,
                    false,
                    0,
                    true,
                )
            }));
            jobs.push(Box::new(move || {
                let j = square_jellium();
                let x = square(17, 1.0);
                let e_sq = j.energy(&x) / 289.0;
                let m = x.len();
                let ev = symmetric_eigenvalues(&j.hessian_fd(&x, 1e-5), m);
                let scale = ev[m - 1].abs();
                let neg = ev.iter().filter(|&&v| v < -1e-9 * scale).count();
                format!(
                    "{{\"label\":\"ctl_neg_square_saddle\",\"e_square_box\":{e_sq:.15e},\"e_square_literature\":{:.15e},\"hess_lowest\":[{:.3e},{:.3e},{:.3e}],\"hess_negative\":{neg},\"e_tri_literature\":{:.15e}}}",
                    jellium::ETA_SQUARE * std::f64::consts::PI.sqrt(), ev[0], ev[1], ev[2], e_tri_jellium()
                )
            }));
            let lines = parallel(jobs, threads);
            write(&jres, "controls.jsonl", &lines);
        }
        "jel-main" => {
            let jres = res.join("jellium");
            let nrun: usize = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(20);
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            for k in 0..nrun {
                let p = init.join(format!("oc20_tri_box_{k:02}.csv"));
                jobs.push(Box::new(move || {
                    jel_run(
                        &format!("oc20_{k:02}"),
                        "hf_oc20",
                        &tri_jellium(),
                        &read_csv(&p),
                        true,
                        100 + k as u64,
                        true,
                    )
                }));
                jobs.push(Box::new(move || {
                    let j = tri_jellium();
                    let x0 =
                        uniform_config(&j.geometry_system(), 288, 0.5 * a, None, 500 + k as u64);
                    jel_run(
                        &format!("uniform_{k:02}"),
                        "uniform",
                        &j,
                        &x0,
                        true,
                        900 + k as u64,
                        true,
                    )
                }));
            }
            for k in 0..(nrun / 2).max(1) {
                let p = init.join(format!("oc20_square_box_{k:02}.csv"));
                jobs.push(Box::new(move || {
                    jel_run(
                        &format!("incommensurate_{k:02}"),
                        "ctl_neg_incommensurate_box",
                        &square_jellium(),
                        &read_csv(&p),
                        true,
                        700 + k as u64,
                        false,
                    )
                }));
            }
            let lines = parallel(jobs, threads);
            write(&jres, "main_kappa_jellium_s1.jsonl", &lines);
        }
        "global" => {
            // Exploratory: can global searches turn the glassy quench / anneal outcomes (soft kappa = 1 especially)
            // into the lattice? Same uniform starts (seeds 500 + k) as `main`, so the runs are directly comparable.
            let kappa: f64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(1.0);
            let nrun: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(4);
            let hops: usize = args.get(5).map(|s| s.parse().unwrap()).unwrap_or(60);
            let rounds: usize = args.get(6).map(|s| s.parse().unwrap()).unwrap_or(400);
            let r_exact = 40.0 / kappa;
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            for k in 0..nrun {
                for method in ["basin_hopping", "parallel_tempering"] {
                    jobs.push(Box::new(move || {
                        let sys = tri_box(kappa);
                        let x0 = uniform_config(&sys, 288, 0.5 * a, None, 500 + k as u64);
                        let clock = std::time::Instant::now();
                        let out = if method == "basin_hopping" {
                            global_opt::basin_hopping(
                                &sys,
                                &x0,
                                &global_opt::HopParams {
                                    hops,
                                    sigma: 0.25 * a,
                                    radius: Some(2.5 * a),
                                    min_sep: 0.3 * a,
                                    temperature: 0.02,
                                    ftol: 1e-7,
                                    tmax: TMAX,
                                    seed: 1300 + k as u64,
                                },
                            )
                        } else {
                            global_opt::parallel_tempering(
                                &sys,
                                &x0,
                                &global_opt::TemperParams {
                                    temps: global_opt::temperature_ladder(4e-3, 2e-2, 24),
                                    rounds,
                                    steps_per_round: 500,
                                    dt: 0.01,
                                    cap: 0.05,
                                    quench_every: 50,
                                    ftol: 1e-7,
                                    tmax: TMAX,
                                    seed: 1700 + k as u64,
                                    threads: 1,
                                },
                            )
                        };
                        let eref = e_tri(kappa);
                        let head = format!(
                            "\"label\":\"uniform_{k:02}\",\"method\":\"{method}\",\"exploratory\":true,\"n\":288,\"kappa\":{kappa},\"e_ref\":{eref:.15e}"
                        );
                        let g = match out {
                            Ok(g) => g,
                            Err(e) => return format!("{{{head},\"error\":\"{}\"}}", e.replace('"', "'")),
                        };
                        // Polish the best minimum to the study's force tolerance.
                        let best = relax(&sys, &g.best.x, FTOL, TMAX).unwrap_or(g.best);
                        let e = sys.energy(&best.x) / 288.0;
                        let o = order(&sys, &best.x, None);
                        let floor = floor_json(&sys, &best.x, r_exact);
                        format!(
                            "{{{head},\"e_start_quench\":{:.15e},\"e\":{e:.15e},\"gap\":{:.6e},\"rel_gap\":{:.6e},\"defects\":{},\"psi6_global\":{:.6},\"psi6_local\":{:.6},\"max_force\":{:.3e},\"converged\":{},\"accepted\":{},\"proposals\":{},\"failures\":{},\"wall_s\":{:.1},\"floor\":{floor}}}",
                            g.start_energy / 288.0,
                            e - eref,
                            (e - eref) / eref.abs(),
                            o.defects,
                            o.psi6_global,
                            o.psi6_local,
                            best.max_force,
                            best.converged,
                            g.accepted,
                            g.proposals,
                            g.failures,
                            clock.elapsed().as_secs_f64()
                        )
                    }));
                }
            }
            let lines = parallel(jobs, threads);
            write(
                &res.join("exploratory"),
                &format!("global_kappa{kappa}.jsonl"),
                &lines,
            );
        }
        "exact-refs" => {
            // Erratum fix (2026-10-25): lattice energies of the plain (untruncated) Yukawa potential with a rigorous
            // tail bound and an independent Ewald-type cross-check, next to the published shifted-force values, plus a
            // cutoff sweep. Written under results/exact/.
            let out = res.join("exact");
            let mut lines = Vec::new();
            for kappa in [1.0, 2.0] {
                let r = 40.0 / kappa;
                let mut obj = format!("{{\"kind\":\"lattice\",\"kappa\":{kappa},\"R\":{r}");
                for (name, basis) in [
                    ("tri", exact::tri_basis()),
                    ("square", exact::square_basis()),
                ] {
                    let d = exact::lattice_energy_exact(kappa, basis, r);
                    let ew: Vec<String> = [0.5, 1.0, 2.0]
                        .iter()
                        .map(|&eta| {
                            format!("{:.16e}", exact::lattice_energy_ewald(kappa, basis, eta))
                        })
                        .collect();
                    let sf = exact::lattice_energy_shifted_force(kappa, basis, 8.0);
                    let _ = write!(
                        obj,
                        ",\"{name}\":{{\"exact_partial\":{:.16e},\"tail_bound\":{:.3e},\"tail_continuum\":{:.3e},\"terms\":{},\"voronoi_delta\":{:.15e},\"ewald_eta_0.5_1_2\":[{}],\"shifted_force_rc8\":{sf:.16e},\"exact_minus_sf\":{:.6e}}}",
                        d.partial,
                        d.tail_bound,
                        d.tail_continuum,
                        d.terms,
                        exact::voronoi_circumradius(basis),
                        ew.join(","),
                        d.partial - sf
                    );
                }
                let t = exact::lattice_energy_exact(kappa, exact::tri_basis(), r).partial;
                let s = exact::lattice_energy_exact(kappa, exact::square_basis(), r).partial;
                let tsf = exact::lattice_energy_shifted_force(kappa, exact::tri_basis(), 8.0);
                let ssf = exact::lattice_energy_shifted_force(kappa, exact::square_basis(), 8.0);
                let _ = write!(
                    obj,
                    ",\"gap_exact\":{:.10e},\"gap_sf_rc8\":{:.10e},\"gap_change\":{:.3e}}}",
                    s - t,
                    ssf - tsf,
                    (s - t) - (ssf - tsf)
                );
                lines.push(obj);
                // Image-summed periodic boxes holding the perfect lattices.
                let tb = exact::ExactBox::new(kappa, r, 16.0 * a, 9.0 * 3.0_f64.sqrt() * a);
                let sb = exact::ExactBox::new(kappa, r, 17.0, 17.0);
                let et = tb.energy(&triangular(16, 18, a)) / 288.0;
                let es = sb.energy(&square(17, 1.0)) / 289.0;
                let (eg, _) = global_opt::periodic_energy_exact(
                    kappa,
                    16.0 * a,
                    9.0 * 3.0_f64.sqrt() * a,
                    &triangular(16, 18, a),
                    r,
                );
                lines.push(format!(
                    "{{\"kind\":\"box\",\"kappa\":{kappa},\"R\":{r},\"tri_box\":{et:.16e},\"tri_box_minus_lattice\":{:.3e},\"tri_box_global_opt_periodic_energy_exact\":{eg:.16e},\"square_box\":{es:.16e},\"square_box_minus_lattice\":{:.3e},\"tri_self_image\":{:.3e},\"square_self_image\":{:.3e}}}",
                    et - t,
                    es - s,
                    tb.self_image(),
                    sb.self_image()
                ));
                for rc in [6.0, 8.0, 10.0, 12.0, 16.0, 20.0, 24.0, 32.0, 40.0] {
                    let mut obj = format!("{{\"kind\":\"sweep\",\"kappa\":{kappa},\"rc\":{rc}");
                    for (name, basis, conv) in [
                        ("tri", exact::tri_basis(), t),
                        ("square", exact::square_basis(), s),
                    ] {
                        let p = exact::lattice_energy_exact(kappa, basis, rc);
                        let sf = exact::lattice_energy_shifted_force(kappa, basis, rc);
                        let _ = write!(
                            obj,
                            ",\"{name}\":{{\"plain\":{:.16e},\"plain_err\":{:.3e},\"plain_tail_err\":{:.3e},\"sf\":{sf:.16e},\"sf_err\":{:.3e},\"tail_bound\":{:.3e}}}",
                            p.partial,
                            p.partial - conv,
                            p.partial + p.tail_continuum - conv,
                            sf - conv,
                            p.tail_bound
                        );
                    }
                    obj.push('}');
                    lines.push(obj);
                }
            }
            write(&out, "exact_refs.jsonl", &lines);
        }
        "exact-relax" => {
            // Erratum fix (2026-10-25): re-run published seeds with the shifted-force protocol (reproducing the
            // published energies), evaluate the exact Yukawa energy at those minimisers, re-relax them under the exact
            // image-summed potential with CVODE, and report gaps to the exact triangular lattice.
            // Args: kappa nrun anneal(0/1) [first].
            let kappa: f64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(2.0);
            let nrun: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(3);
            let do_anneal = args.get(5).map(|s| s == "1").unwrap_or(false);
            let first: usize = args.get(6).map(|s| s.parse().unwrap()).unwrap_or(0);
            let r = 40.0 / kappa;
            let e_tri_exact = exact::lattice_energy_exact(kappa, exact::tri_basis(), r).partial;
            let mut jobs: Vec<Job<String>> = Vec::new();
            let mut specs: Vec<(String, String, bool, u64, u64)> = Vec::new();
            for k in first..first + nrun {
                specs.push((
                    format!("oc20_{k:02}"),
                    "hf_oc20".into(),
                    false,
                    100 + k as u64,
                    k as u64,
                ));
                specs.push((
                    format!("uniform_{k:02}"),
                    "uniform".into(),
                    false,
                    900 + k as u64,
                    k as u64,
                ));
            }
            for k in first..first + (nrun / 2).max(1) {
                specs.push((
                    format!("incommensurate_{k:02}"),
                    "ctl_neg_incommensurate_box".into(),
                    true,
                    700 + k as u64,
                    k as u64,
                ));
            }
            for (label, family, incomm, aseed, k) in specs {
                let init = init.clone();
                jobs.push(Box::new(move || {
                    let t0 = std::time::Instant::now();
                    let (sys, ebox, n) = if incomm {
                        let l = 17.0;
                        (square_box(kappa), exact::ExactBox::new(kappa, r, l, l), 289usize)
                    } else {
                        (
                            tri_box(kappa),
                            exact::ExactBox::new(kappa, r, 16.0 * a, 9.0 * 3.0_f64.sqrt() * a),
                            288usize,
                        )
                    };
                    let x0 = match family.as_str() {
                        "uniform" => uniform_config(&sys, 288, 0.5 * a, None, 500 + k),
                        "hf_oc20" => read_csv(&init.join(format!("oc20_tri_box_{k:02}.csv"))),
                        _ => read_csv(&init.join(format!("oc20_square_box_{k:02}.csv"))),
                    };
                    let mut fields = format!(
                        "\"label\":\"{label}\",\"family\":\"{family}\",\"n\":{n},\"kappa\":{kappa},\"R\":{r},\"e_tri_exact\":{e_tri_exact:.16e},\"e_tri_sf\":{:.16e},\"tail_continuum\":{:.3e}",
                        e_tri(kappa),
                        ebox.tail_continuum(n)
                    );
                    let mut stages: Vec<(&str, Vec<f64>)> = Vec::new();
                    match relax(&sys, &x0, FTOL, TMAX) {
                        Ok(q) => stages.push(("quench", q.x)),
                        Err(e) => {
                            let _ = write!(fields, ",\"quench_error\":\"{}\"", e.replace('"', "'"));
                        }
                    }
                    if do_anneal {
                        let xa = anneal(&sys, &x0, 0.05, 1e-5, 300_000, 0.01, 0.05, aseed);
                        match relax(&sys, &xa, FTOL, TMAX) {
                            Ok(q) => stages.push(("anneal_quench", q.x)),
                            Err(e) => {
                                let _ = write!(fields, ",\"anneal_quench_error\":\"{}\"", e.replace('"', "'"));
                            }
                        }
                    }
                    for (tag, xs) in stages {
                        let e_sf = sys.energy(&xs) / n as f64;
                        let e_ex0 = ebox.energy(&xs) / n as f64;
                        let o0 = order(&sys, &xs, None);
                        let _ = write!(
                            fields,
                            ",\"{tag}\":{{\"sf_e\":{e_sf:.16e},\"sf_gap\":{:.6e},\"sf_defects\":{},\"exact_e_at_sf_min\":{e_ex0:.16e},\"exact_gap_at_sf_min\":{:.6e}",
                            e_sf - e_tri(kappa),
                            o0.defects,
                            e_ex0 - e_tri_exact
                        );
                        match exact::relax_exact(&ebox, &xs, FTOL, TMAX) {
                            Ok(q) => {
                                let e = ebox.energy(&q.x) / n as f64;
                                let o = order(&sys, &q.x, None);
                                let ev = symmetric_eigenvalues(&ebox.hessian(&q.x), q.x.len());
                                let scale = ev[ev.len() - 1].abs();
                                let neg = ev.iter().filter(|&&v| v < -1e-9 * scale).count();
                                let disp = xs
                                    .iter()
                                    .zip(&q.x)
                                    .enumerate()
                                    .map(|(c, (p, qv))| {
                                        let l = if c % 2 == 0 { ebox.lx } else { ebox.ly };
                                        let d = qv - p;
                                        (d - l * (d / l).round()).abs()
                                    })
                                    .fold(0.0_f64, f64::max);
                                let _ = write!(
                                    fields,
                                    ",\"exact_e\":{e:.16e},\"exact_gap\":{:.6e},\"exact_rel_gap\":{:.6e},\"exact_defects\":{},\"exact_psi6_global\":{:.6},\"max_force\":{:.3e},\"converged\":{},\"cvode_steps\":{},\"rhs_evals\":{},\"max_displacement\":{disp:.3e},\"hess_lowest\":[{:.3e},{:.3e},{:.3e}],\"hess_negative\":{neg}}}",
                                    e - e_tri_exact,
                                    (e - e_tri_exact) / e_tri_exact,
                                    o.defects,
                                    o.psi6_global,
                                    q.max_force,
                                    q.converged,
                                    q.steps,
                                    q.rhs_evals,
                                    ev[0],
                                    ev[1],
                                    ev[2]
                                );
                            }
                            Err(e) => {
                                let _ = write!(fields, ",\"exact_error\":\"{}\"}}", e.replace('"', "'"));
                            }
                        }
                    }
                    let _ = write!(fields, ",\"wall_s\":{:.1}", t0.elapsed().as_secs_f64());
                    let line = format!("{{{fields}}}");
                    eprintln!("{line}");
                    line
                }));
            }
            let lines = parallel(jobs, threads);
            write(
                &res.join("exact"),
                &format!(
                    "exact_relax_kappa{kappa}_a{}_f{first}.jsonl",
                    u8::from(do_anneal)
                ),
                &lines,
            );
        }
        "exact-saddle" => {
            // Negative control under the exact potential: the perfect square lattice in its commensurate 17x17 box is a
            // critical point; count unstable Hessian modes and relax a 1e-3 perturbation with CVODE.
            let mut lines = Vec::new();
            for kappa in [2.0, 1.0] {
                let r = 40.0 / kappa;
                let sb = exact::ExactBox::new(kappa, r, 17.0, 17.0);
                let sys = square_box(kappa);
                let x = square(17, 1.0);
                let es = sb.energy(&x) / 289.0;
                let ev = symmetric_eigenvalues(&sb.hessian(&x), x.len());
                let scale = ev[ev.len() - 1].abs();
                let neg = ev.iter().filter(|&&v| v < -1e-9 * scale).count();
                let mut xs = x.clone();
                let mut rng = Rng::new(5);
                xs.iter_mut().for_each(|v| *v += 1e-3 * rng.gauss());
                let q = exact::relax_exact(&sb, &xs, FTOL, TMAX).unwrap();
                let e_after = sb.energy(&q.x) / 289.0;
                let e_t = exact::lattice_energy_exact(kappa, exact::tri_basis(), r).partial;
                let o = order(&sys, &q.x, None);
                lines.push(format!(
                    "{{\"label\":\"ctl_neg_square_saddle_exact\",\"kappa\":{kappa},\"R\":{r},\"e_square_box\":{es:.16e},\"hess_lowest\":[{:.3e},{:.3e},{:.3e}],\"hess_negative\":{neg},\"e_after_relax\":{e_after:.16e},\"lowered\":{},\"e_after_minus_e_tri_exact\":{:.6e},\"defects_after\":{},\"psi6_global_after\":{:.6},\"converged\":{}}}",
                    ev[0], ev[1], ev[2], e_after < es, e_after - e_t, o.defects, o.psi6_global, q.converged
                ));
                eprintln!("{}", lines.last().unwrap());
            }
            write(&res.join("exact"), "exact_saddle.jsonl", &lines);
        }
        "floor" => {
            // Exact-floor audit. Without CSVs: positive controls (lattice, relaxed noisy lattice) and the square
            // lattice. With CSVs: each file is checked in the study box (288 points: triangular box, 289: square box).
            let kappa: f64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(1.0);
            let r_exact = 40.0 / kappa;
            let mut lines = Vec::new();
            let files: Vec<PathBuf> = args.iter().skip(4).map(PathBuf::from).collect();
            if files.is_empty() {
                let sys = tri_box(kappa);
                let x = triangular(16, 18, a);
                lines.push(format!(
                    "{{\"label\":\"triangular\",\"kappa\":{kappa},\"floor\":{}}}",
                    floor_json(&sys, &x, r_exact)
                ));
                let mut xn = x.clone();
                let mut rng = Rng::new(3);
                xn.iter_mut().for_each(|v| *v += 0.1 * a * rng.gauss());
                let r = relax(&sys, &xn, FTOL, TMAX).unwrap();
                lines.push(format!(
                    "{{\"label\":\"noisy_relaxed\",\"kappa\":{kappa},\"floor\":{}}}",
                    floor_json(&sys, &r.x, r_exact)
                ));
                let sq = square_box(kappa);
                lines.push(format!(
                    "{{\"label\":\"square\",\"kappa\":{kappa},\"floor\":{}}}",
                    floor_json(&sq, &square(17, 1.0), r_exact)
                ));
            }
            for p in files {
                let x = read_csv(&p);
                let sys = if x.len() / 2 == 289 {
                    square_box(kappa)
                } else {
                    tri_box(kappa)
                };
                lines.push(format!(
                    "{{\"file\":\"{}\",\"kappa\":{kappa},\"n\":{},\"floor\":{}}}",
                    p.display(),
                    x.len() / 2,
                    floor_json(&sys, &x, r_exact)
                ));
            }
            for l in &lines {
                if l.contains("\"violated\":true") {
                    eprintln!("FLOOR VIOLATION (bug, not physics): {l}");
                }
            }
            write(
                &res.join("exploratory"),
                &format!("floor_kappa{kappa}.jsonl"),
                &lines,
            );
        }
        other => panic!("unknown command {other}"),
    }
}
