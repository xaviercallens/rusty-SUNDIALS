//! Experiment driver for the preregistered study `results/preregistration.json` (YUK-TRI-1).
//!
//! `yukawa_crystal refs|controls|main|disk <crate_dir>` writes JSON results under `<crate_dir>/results/`.

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
            let mut jobs: Vec<Box<dyn FnOnce() -> (String, Vec<f64>, f64) + Send>> = Vec::new();
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
        "fss-lattice" => {
            // Finite-size check of the perfect triangular lattice in commensurate periodic boxes.
            // With the minimum image, E/N equals the direct lattice sum e_tri exactly once both box sides are
            // >= 2 rc, whatever N is. The 6 x 12 box (6.45 x 11.17, below 2 rc = 16) is the negative control:
            // there the minimum image misses periodic images inside rc, so E/N must come out wrong (too low).
            println!(
                "kappa nx ny N lx ly sides_ge_2rc e_box_per_particle e_tri_direct_sum e_box_minus_e_tri"
            );
            for kappa in [2.0, 1.0] {
                let e_ref = e_tri(kappa);
                for (nx, ny) in [(6, 12), (16, 18), (32, 36), (48, 54)] {
                    let (lx, ly) = (nx as f64 * a, ny as f64 * a * 3.0_f64.sqrt() / 2.0);
                    let sys = System {
                        kappa,
                        rc: 8.0,
                        geometry: Geometry::Periodic { lx, ly },
                    };
                    let n = nx * ny;
                    let e_box = sys.energy(&triangular(nx, ny, a)) / n as f64;
                    let ok = lx.min(ly) >= 2.0 * sys.rc;
                    println!(
                        "{kappa} {nx} {ny} {n} {lx:.4} {ly:.4} {ok} {e_box:.15e} {e_ref:.15e} {:.3e}",
                        e_box - e_ref
                    );
                }
            }
        }
        other => panic!("unknown command {other}"),
    }
}
