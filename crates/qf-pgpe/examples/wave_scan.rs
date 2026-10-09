//! The registered vortex-phonon scattering scan (`run_wave_scan.py`, `run_wave_scan_d0.py`, `analyze_wave_scan.py`,
//! `analyze_wave_scan_d0.py`), run in parallel over runs and analysed in one go.
//!
//!     cargo run --release -p qf-pgpe --example wave_scan -- --out-dir DIR [--workers 2] [--d0 20,24,28]
//!         [--t-max 400] [--L 64] [--N 128] [--ms 4,8,...] [--dry]
//!
//! Without `--d0`: the registered matrix at `d0 = 32`: a control (`eps = 0`) and `m` in {1,2,3,4,6,8,10,12,14,16,20,24,28}
//! x `dir = +-1` at `av = 0.04`, plus `av = 0.08` repeats at `m = 8, 16`. With `--d0 LIST`: the follow-up scan (WS-A1),
//! one control per `d0` and `m` in {4,8,10,12,14,16,20} x `dir = +-1` at `av = 0.04`. File names follow the Python
//! runners (`WS_m04_p_av0.04`, `WS_d20_m04_p_av0.04`, `WS_control`, `WS_d20_control`), each with `.csv` and `.json`;
//! finished runs are skipped (resume). The summary `wave_scan_rust_results.json` holds, per `d0`, the control slopes and
//! `sigma_par(k)`, `sigma_perp(k)` at `av = 0.04` (control subtracted, directions averaged).
use qf_pgpe::ComplexField2D;
use qf_pgpe::scattering::{
    Sigma, Slopes, WaveParams, WaveRun, drift_slopes, eps_from_av, run_wave_pair, sigma_from_pair,
    wavenumber,
};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Clone)]
struct Job {
    name: String,
    d0: f64,
    m: usize,
    av: f64,
    dir: i32,
    control: bool,
}

fn job_name(d0: f64, follow_up: bool, m: usize, av: f64, dir: i32, control: bool) -> String {
    let pre = if follow_up {
        format!("WS_d{}_", d0 as i64)
    } else {
        "WS_".to_string()
    };
    if control {
        format!("{pre}control")
    } else {
        format!("{pre}m{m:02}_{}_av{av}", if dir > 0 { "p" } else { "m" })
    }
}

fn parse_list<T: std::str::FromStr>(s: &str) -> Vec<T>
where
    T::Err: std::fmt::Debug,
{
    s.split(',').map(|x| x.trim().parse().unwrap()).collect()
}

fn jobs(d0s: &[f64], follow_up: bool, ms: &[usize]) -> Vec<Job> {
    let mut j = Vec::new();
    for &d0 in d0s {
        j.push(Job {
            name: job_name(d0, follow_up, 4, 0.0, 1, true),
            d0,
            m: 4,
            av: 0.0,
            dir: 1,
            control: true,
        });
        for &m in ms {
            for dir in [1, -1] {
                j.push(Job {
                    name: job_name(d0, follow_up, m, 0.04, dir, false),
                    d0,
                    m,
                    av: 0.04,
                    dir,
                    control: false,
                });
            }
        }
        if !follow_up {
            for m in [8, 16] {
                for dir in [1, -1] {
                    j.push(Job {
                        name: job_name(d0, follow_up, m, 0.08, dir, false),
                        d0,
                        m,
                        av: 0.08,
                        dir,
                        control: false,
                    });
                }
            }
        }
    }
    j
}

fn load(dir: &Path, name: &str) -> Option<WaveRun> {
    let csv = std::fs::read_to_string(dir.join(format!("{name}.csv"))).ok()?;
    let meta = std::fs::read_to_string(dir.join(format!("{name}.json"))).ok()?;
    WaveRun::from_files(&csv, &meta)
}

fn jnum(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        "null".into()
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (mut out_dir, mut workers, mut t_max, mut l, mut n) =
        (PathBuf::from("wave_scan"), 2usize, 400.0, 64.0, 128usize);
    let (mut d0s, mut follow_up, mut dry) = (vec![32.0], false, false);
    let mut ms: Option<Vec<usize>> = None;
    let mut i = 1;
    while i < a.len() {
        let val = |i: usize| {
            a.get(i + 1)
                .unwrap_or_else(|| panic!("missing value for {}", a[i]))
        };
        match a[i].as_str() {
            "--out-dir" => out_dir = PathBuf::from(val(i)),
            "--workers" => workers = val(i).parse().unwrap(),
            "--t-max" => t_max = val(i).parse().unwrap(),
            "--L" => l = val(i).parse().unwrap(),
            "--N" => n = val(i).parse().unwrap(),
            "--d0" => {
                d0s = parse_list(val(i));
                follow_up = true;
            }
            "--ms" => ms = Some(parse_list(val(i))),
            "--dry" => {
                dry = true;
                i += 1;
                continue;
            }
            other => panic!("unknown option {other}"),
        }
        i += 2;
    }
    let ms = ms.unwrap_or_else(|| {
        if follow_up {
            vec![4, 8, 10, 12, 14, 16, 20]
        } else {
            vec![1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28]
        }
    });
    std::fs::create_dir_all(&out_dir).unwrap();
    let all = jobs(&d0s, follow_up, &ms);
    let todo: Vec<Job> = all
        .iter()
        .filter(|j| !out_dir.join(format!("{}.json", j.name)).exists())
        .cloned()
        .collect();
    println!(
        "{} runs, {} to do, {workers} workers",
        all.len(),
        todo.len()
    );
    if dry {
        todo.iter().for_each(|j| println!("{}", j.name));
        return;
    }
    let log = Mutex::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out_dir.join("queue_rust.log"))
            .unwrap(),
    );
    let note = |s: String| {
        use std::io::Write;
        let _ = writeln!(log.lock().unwrap(), "{s}");
        println!("{s}");
    };
    let t_all = Instant::now();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .unwrap();
    pool.install(|| {
        todo.par_iter().with_max_len(1).for_each(|j| {
            let t0 = Instant::now();
            let f = ComplexField2D::new(n, l, 1.0, 0.01);
            let eps = if j.control {
                0.0
            } else {
                eps_from_av(wavenumber(l, j.m), j.av)
            };
            let run = run_wave_pair(&f, &WaveParams::new(j.m, eps, j.dir, t_max, j.d0));
            let secs = (t0.elapsed().as_secs_f64() * 10.0).round() / 10.0;
            std::fs::write(out_dir.join(format!("{}.csv", j.name)), run.csv()).unwrap();
            std::fs::write(
                out_dir.join(format!("{}.json", j.name)),
                run.meta_json(secs),
            )
            .unwrap();
            note(format!(
                "end   {} ended={} {secs}s",
                j.name,
                run.ended.name()
            ));
        });
    });
    note(format!(
        "queue done ({:.0}s wall)",
        t_all.elapsed().as_secs_f64()
    ));

    // analysis
    let mut summary = String::from("{\n \"by_d0\": {\n");
    for (di, &d0) in d0s.iter().enumerate() {
        let c_name = job_name(d0, follow_up, 4, 0.0, 1, true);
        let ctl = load(&out_dir, &c_name).and_then(|r| drift_slopes(&r, l, 50.0));
        let Some(c) = ctl else {
            eprintln!("control missing/lost for d0={d0}");
            continue;
        };
        let mut by: BTreeMap<(usize, i64), BTreeMap<i32, Slopes>> = BTreeMap::new();
        for j in all.iter().filter(|j| j.d0 == d0 && !j.control) {
            match load(&out_dir, &j.name).and_then(|r| drift_slopes(&r, l, 50.0)) {
                Some(s) => {
                    by.entry((j.m, (j.av * 1000.0).round() as i64))
                        .or_default()
                        .insert(j.dir, s);
                }
                None => eprintln!("dropped (vortex lost or short): {}", j.name),
            }
        }
        let mut pts: Vec<(usize, i64, Sigma)> = Vec::new();
        for ((m, av), dd) in &by {
            if let (Some(p), Some(q)) = (dd.get(&1), dd.get(&-1)) {
                pts.push((*m, *av, sigma_from_pair(p, q, (c.sdx, c.syc))));
            }
        }
        println!(
            "d0 = {d0}: control slope(d_x) {:+.3e}, slope(y_c) {:+.3e}, d change {:+.3}",
            c.sdx, c.syc, c.d_change
        );
        let mut rows = Vec::new();
        for (m, av, s) in &pts {
            println!(
                "  m={m:2} av={:.2} k={:.3} sigma_par {:.3} (+{:.3} -{:.3}) sigma_perp {:.3} odd {}",
                *av as f64 / 1000.0,
                s.k,
                s.sigma_par,
                s.sp,
                s.sm,
                s.sigma_perp,
                s.odd
            );
            rows.push(format!(
                "   {{\"m\": {m}, \"av\": {}, \"k\": {}, \"j\": {}, \"sigma_par\": {}, \"sp\": {}, \"sm\": {}, \"sigma_perp\": {}, \"tp\": {}, \"tm\": {}, \"odd\": {}, \"odd_strict\": {}, \"d_dx_p\": {}, \"d_dx_m\": {}}}",
                *av as f64 / 1000.0, jnum(s.k), jnum(s.j), jnum(s.sigma_par), jnum(s.sp), jnum(s.sm), jnum(s.sigma_perp), jnum(s.tp), jnum(s.tm), s.odd, s.odd_strict, jnum(s.d_dx_p), jnum(s.d_dx_m)
            ));
        }
        summary += &format!(
            "  \"{d0}\": {{\n   \"control\": {{\"sdx\": {}, \"syc\": {}, \"d_change\": {}}},\n   \"points\": [\n{}\n   ]\n  }}{}\n",
            jnum(c.sdx),
            jnum(c.syc),
            jnum(c.d_change),
            rows.join(",\n"),
            if di + 1 < d0s.len() { "," } else { "" }
        );
    }
    summary += " }\n}\n";
    std::fs::write(out_dir.join("wave_scan_rust_results.json"), summary).unwrap();
}
