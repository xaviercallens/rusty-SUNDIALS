//! Driver for the bilayer dipolar-exciton study (preregistration `results/dipolar/preregistration.json`).
//!
//! Usage: `dipolar_crystal <refs|gaps|controls|main> <crate_dir> [d]`. Writes JSONL into `<crate_dir>/results/dipolar/`.
//! Threads: `DIPOLAR_THREADS` (default 2).

use qf_yukawa_crystal::dipolar::{Dipolar, honeycomb_energy_full, lattice_energy_full, tail};
use qf_yukawa_crystal::dipolar_ewald::{
    DipolarEwald, honeycomb_exact, square_exact, triangular_exact,
};
use qf_yukawa_crystal::{
    Rng, order, spacing_density_one, square, symmetric_eigenvalues, triangular, uniform_config,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const FTOL: f64 = 1e-9;
const TMAX: f64 = 1e7;
const RC: f64 = 8.0;
const R_REPORT: [f64; 2] = [32.0, 64.0];
const D_VALUES: [f64; 5] = [0.0209, 0.066, 0.114, 0.3, 1.0];
const ETA: f64 = 0.8;

fn a() -> f64 {
    spacing_density_one()
}

fn tri_basis() -> [[f64; 2]; 2] {
    let a = a();
    [[a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]]
}

fn tri_box(d: f64) -> Dipolar {
    let a = a();
    Dipolar::new(d, RC, 16.0 * a, 9.0 * 3.0_f64.sqrt() * a)
}

fn square_box(d: f64) -> Dipolar {
    Dipolar::new(d, RC, 17.0, 17.0)
}

fn write(dir: &Path, name: &str, lines: &[String]) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(name), lines.join("\n") + "\n").unwrap();
    println!("wrote {}", dir.join(name).display());
}

fn parallel(jobs: Vec<Box<dyn FnOnce() -> String + Send>>, threads: usize) -> Vec<String> {
    let n = jobs.len();
    let queue = Arc::new(Mutex::new(jobs.into_iter().enumerate().collect::<Vec<_>>()));
    let out = Arc::new(Mutex::new(vec![String::new(); n]));
    let mut hs = Vec::new();
    for _ in 0..threads.max(1) {
        let (q, o) = (Arc::clone(&queue), Arc::clone(&out));
        hs.push(std::thread::spawn(move || {
            loop {
                let job = q.lock().unwrap().pop();
                match job {
                    Some((i, f)) => {
                        let s = f();
                        eprintln!("done job {i}");
                        o.lock().unwrap()[i] = s;
                    }
                    None => break,
                }
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    Arc::try_unwrap(out).unwrap().into_inner().unwrap()
}

/// Hessian summary: three lowest eigenvalues and the number below -1e-8 (relative to the largest).
fn hessian_summary(sys: &Dipolar, x: &[f64]) -> (f64, f64, f64, usize) {
    let m = x.len();
    let ev = symmetric_eigenvalues(&sys.hessian_sf(x), m);
    let scale = ev.last().unwrap().abs().max(1e-300);
    let neg = ev.iter().filter(|&&v| v < -1e-8 * scale).count();
    (ev[0], ev[1], ev[2], neg)
}

struct Refs {
    e_tri_sf: f64,
    e_tri_full: [f64; 2],
    e_tri_exact: f64,
}

fn refs_for(d: f64) -> Refs {
    let sys = tri_box(d);
    let x = triangular(16, 18, a());
    Refs {
        e_tri_sf: sys.energy_sf(&x) / 288.0,
        e_tri_full: [
            lattice_energy_full(d, tri_basis(), R_REPORT[0]),
            lattice_energy_full(d, tri_basis(), R_REPORT[1]),
        ],
        e_tri_exact: triangular_exact(d, ETA),
    }
}

/// Exact (Ewald) energy per particle of a configuration in the periodic box of `sys`.
fn exact_box(sys: &Dipolar, x: &[f64]) -> f64 {
    DipolarEwald::new(sys.d, ETA, [sys.lx, 0.0], [0.0, sys.ly]).energy_per_particle(x)
}

/// Relax and record a run (truncated and full energies, gaps to the triangular references, order, Hessian).
fn record(tag: &str, sys: &Dipolar, x0: &[f64], r: &Refs, hess: bool) -> String {
    let t0 = Instant::now();
    match sys.relax(x0, FTOL, TMAX) {
        Ok(rel) => {
            let n = rel.x.len() / 2;
            let e_sf = sys.energy_sf(&rel.x) / n as f64;
            let ef0 = sys.energy_full(&rel.x, R_REPORT[0]);
            let ef1 = sys.energy_full(&rel.x, R_REPORT[1]);
            let ex = exact_box(sys, &rel.x);
            let o = order(&sys.geometry_system(), &rel.x, None);
            let mut s = format!(
                "\"{tag}\":{{\"e_exact\":{ex:.15e},\"gap_exact\":{:.6e},\"rel_gap_exact\":{:.6e},\"e_sf\":{e_sf:.15e},\"gap_sf\":{:.6e},\"e_full_R32\":{ef0:.15e},\"gap_full_R32\":{:.6e},\"e_full_R64\":{ef1:.15e},\"gap_full_R64\":{:.6e},\"defects\":{},\"psi6_global\":{:.6},\"psi6_local\":{:.6},\"max_force\":{:.3e},\"t\":{:.3e},\"cvode_steps\":{},\"rhs_evals\":{},\"converged\":{},\"wall_s\":{:.2}",
                ex - r.e_tri_exact,
                (ex - r.e_tri_exact) / r.e_tri_exact,
                e_sf - r.e_tri_sf,
                ef0 - r.e_tri_full[0],
                ef1 - r.e_tri_full[1],
                o.defects,
                o.psi6_global,
                o.psi6_local,
                rel.max_force,
                rel.t,
                rel.steps,
                rel.rhs_evals,
                rel.converged,
                t0.elapsed().as_secs_f64()
            );
            if hess {
                let (l0, l1, l2, neg) = hessian_summary(sys, &rel.x);
                s += &format!(
                    ",\"hess_lowest\":[{l0:.3e},{l1:.3e},{l2:.3e}],\"hess_negative\":{neg}"
                );
            }
            s + "}"
        }
        Err(e) => format!("\"{tag}\":{{\"error\":\"{}\"}}", e.replace('"', "'")),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("refs");
    let crate_dir = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| ".".into()));
    let res = crate_dir.join("results").join("dipolar");
    let threads = std::env::var("DIPOLAR_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&t| t > 0)
        .unwrap_or(2);
    match cmd {
        "refs" => {
            let mut lines = Vec::new();
            for d in D_VALUES {
                let sys = tri_box(d);
                let x = triangular(16, 18, a());
                let mut conv = String::new();
                for big_r in [8.0, 16.0, 32.0, 64.0, 128.0, 256.0] {
                    let e = lattice_energy_full(d, tri_basis(), big_r);
                    let esq = lattice_energy_full(d, [[1.0, 0.0], [0.0, 1.0]], big_r);
                    conv += &format!(
                        "{}{{\"R\":{big_r},\"e_tri_full\":{e:.15e},\"e_square_full\":{esq:.15e},\"square_minus_tri\":{:.9e},\"tail\":{:.9e}}}",
                        if conv.is_empty() { "" } else { "," },
                        esq - e,
                        tail(d, 1.0, big_r)
                    );
                }
                let (bs, bt) = sys.energy_full_parts(&x, 64.0);
                let xs = square(17, 1.0);
                let ssys = square_box(d);
                let et = triangular_exact(d, ETA);
                let et_eta = [triangular_exact(d, 0.6), triangular_exact(d, 1.2)];
                let es = square_exact(d, ETA);
                let eh = honeycomb_exact(d, ETA);
                let ebox = exact_box(&sys, &x);
                let esbox = exact_box(&ssys, &xs);
                lines.push(format!(
                    "{{\"d\":{d},\"eta\":{ETA},\"e_tri_exact\":{et:.15e},\"e_tri_exact_eta0.6\":{:.15e},\"e_tri_exact_eta1.2\":{:.15e},\"e_tri_exact_box288\":{ebox:.15e},\"e_square_exact\":{es:.15e},\"e_square_exact_box289\":{esbox:.15e},\"e_honeycomb_exact\":{eh:.15e},\"square_minus_tri_exact\":{:.9e},\"honeycomb_minus_tri_exact\":{:.9e},\"rc\":{RC},\"e_tri_sf_box\":{:.15e},\"e_square_sf_box\":{:.15e},\"e_tri_full_box_R64\":{:.15e},\"box_sum_part_R64\":{bs:.15e},\"box_tail_part_R64\":{bt:.15e},\"e_tri_full_lattice_R64\":{:.15e},\"e_honeycomb_full_R64\":{:.15e},\"convergence\":[{conv}]}}",
                    et_eta[0],
                    et_eta[1],
                    es - et,
                    eh - et,
                    sys.energy_sf(&x) / 288.0,
                    ssys.energy_sf(&xs) / 289.0,
                    bs + bt,
                    lattice_energy_full(d, tri_basis(), 64.0),
                    honeycomb_energy_full(d, 64.0)
                ));
            }
            write(&res, "references.jsonl", &lines);
        }
        "gaps" => {
            let mut lines = Vec::new();
            for k in 0..=40 {
                let d = 0.01 * 300.0_f64.powf(k as f64 / 40.0);
                let e_tri = triangular_exact(d, ETA);
                let e_sq = square_exact(d, ETA);
                let e_hc = honeycomb_exact(d, ETA);
                let e_tri64 = lattice_energy_full(d, tri_basis(), 64.0);
                let e_sq64 = lattice_energy_full(d, [[1.0, 0.0], [0.0, 1.0]], 64.0);
                lines.push(format!(
                    "{{\"d\":{d:.6e},\"e_tri\":{e_tri:.15e},\"e_square\":{e_sq:.15e},\"e_honeycomb\":{e_hc:.15e},\"rel_gap_square\":{:.9e},\"rel_gap_honeycomb\":{:.9e},\"square_minus_tri\":{:.9e},\"square_minus_tri_direct_R64\":{:.9e},\"e_tri_exact_eta1.2\":{:.15e}}}",
                    e_sq / e_tri - 1.0,
                    e_hc / e_tri - 1.0,
                    e_sq - e_tri,
                    e_sq64 - e_tri64,
                    triangular_exact(d, 1.2)
                ));
            }
            write(&res, "lattice_gaps.jsonl", &lines);
        }
        "controls" => {
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            for d in D_VALUES {
                jobs.push(Box::new(move || {
                    let r = refs_for(d);
                    let sys = tri_box(d);
                    format!(
                        "{{\"label\":\"ctl_pos_perfect\",\"d\":{d},{}}}",
                        record("quench", &sys, &triangular(16, 18, a()), &r, true)
                    )
                }));
                jobs.push(Box::new(move || {
                    let r = refs_for(d);
                    let sys = tri_box(d);
                    let mut x = triangular(16, 18, a());
                    let mut rng = Rng::new(3);
                    x.iter_mut().for_each(|v| *v += 0.1 * a() * rng.gauss());
                    format!(
                        "{{\"label\":\"ctl_pos_noisy\",\"d\":{d},{}}}",
                        record("quench", &sys, &x, &r, true)
                    )
                }));
                jobs.push(Box::new(move || {
                    let r = refs_for(d);
                    let sys = square_box(d);
                    let x = square(17, 1.0);
                    let e_sq_sf = sys.energy_sf(&x) / 289.0;
                    let e_sq_ex = exact_box(&sys, &x);
                    let (l0, l1, l2, neg) = hessian_summary(&sys, &x);
                    let mut xs = x.clone();
                    let mut rng = Rng::new(5);
                    xs.iter_mut().for_each(|v| *v += 1e-3 * rng.gauss());
                    format!(
                        "{{\"label\":\"ctl_neg_square_saddle\",\"d\":{d},\"e_square_sf_box\":{e_sq_sf:.15e},\"e_square_exact_box\":{e_sq_ex:.15e},\"square_minus_tri_exact\":{:.6e},\"hess_lowest\":[{l0:.3e},{l1:.3e},{l2:.3e}],\"hess_negative\":{neg},{}}}",
                        e_sq_ex - r.e_tri_exact,
                        record("relax_from_square_plus_noise", &sys, &xs, &r, false)
                    )
                }));
            }
            let lines = parallel(jobs, threads);
            write(&res, "controls.jsonl", &lines);
        }
        "main" => {
            let d: f64 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(0.066);
            let nq: usize = args.get(4).map(|s| s.parse().unwrap()).unwrap_or(8);
            let na: usize = args.get(5).map(|s| s.parse().unwrap()).unwrap_or(2);
            let mut jobs: Vec<Box<dyn FnOnce() -> String + Send>> = Vec::new();
            for k in 0..nq {
                jobs.push(Box::new(move || {
                    let r = refs_for(d);
                    let sys = tri_box(d);
                    let x0 = uniform_config(&sys.geometry_system(), 288, 0.5 * a(), None, 500 + k as u64);
                    format!(
                        "{{\"label\":\"uniform_{k:02}\",\"family\":\"uniform\",\"d\":{d},\"n\":288,{}}}",
                        record("quench", &sys, &x0, &r, true)
                    )
                }));
            }
            for k in 0..na {
                jobs.push(Box::new(move || {
                    let r = refs_for(d);
                    let sys = tri_box(d);
                    let x0 = uniform_config(&sys.geometry_system(), 288, 0.5 * a(), None, 600 + k as u64);
                    let t0 = Instant::now();
                    let xa = sys.anneal(&x0, 0.05, 1e-5, 300_000, 0.002, 0.05 * a(), 900 + k as u64);
                    format!(
                        "{{\"label\":\"anneal_{k:02}\",\"family\":\"uniform_annealed\",\"d\":{d},\"n\":288,\"anneal_wall_s\":{:.2},{}}}",
                        t0.elapsed().as_secs_f64(),
                        record("anneal_quench", &sys, &xa, &r, true)
                    )
                }));
            }
            for k in 0..2usize {
                jobs.push(Box::new(move || {
                    let r = refs_for(d);
                    let sys = square_box(d);
                    let x0 = uniform_config(&sys.geometry_system(), 289, 0.5 * a(), None, 700 + k as u64);
                    format!(
                        "{{\"label\":\"incommensurate_{k:02}\",\"family\":\"ctl_neg_incommensurate_box\",\"d\":{d},\"n\":289,{}}}",
                        record("quench", &sys, &x0, &r, false)
                    )
                }));
            }
            let lines = parallel(jobs, threads);
            write(&res, &format!("main_d{d}.jsonl"), &lines);
        }
        other => panic!("unknown command {other}"),
    }
}
