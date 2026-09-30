//! Driver for QuantumFluids' PGPE round-3 finite-size study (amendment R3-A3, `docs/designs/
//! PGPE_R3_PREREG.md` in SocrateAI-Scientific-QuantumFluids), reimplementing `run_r3.py`'s "C3"
//! timing loop on top of this crate's `ComplexField2D` instead of the Python solver, for speed
//! and to checkpoint (the Python run had no checkpointing at all and was lost, cause
//! undiagnosable without kernel-log access, after ~2 days of compute).
//!
//! This binary does **not** compute the round's block-averaged observables (vortex detection,
//! `g1` radial correlation, condensate fraction, current correlators, the energy budget, the
//! vortex thermometer) — those are existing, already-validated Python
//! (`exploration/pgpe/{round2,observables,vortex_thermometer_canonical}.py`) that this port does
//! not attempt to reproduce here. Instead: this binary does the expensive part (integrating
//! ~700,000 IF-RK4 steps per configuration) and writes raw field snapshots every 10 time units
//! during the 1000-unit observation window, plus periodic checkpoints throughout, for a separate
//! Python pass to run the existing analysis on.
//!
//! Checkpointing: every `CHECKPOINT_EVERY_STEPS` steps, `<name>.checkpoint.raw` (the field `c`,
//! raw complex128, numpy-loadable via `np.fromfile(path, dtype=complex128).reshape(n, n)`) and
//! `<name>.checkpoint.meta` (`steps_done t_sim`, plain text) are written; on startup, a config
//! resumes from its checkpoint if one exists, rather than rebuilding `random_state` from scratch.
//! A crash loses at most one checkpoint interval of compute, not the whole run.
//!
//! `random_state` is not in this crate (only `pgpe.py`'s core ODE was ported); it is implemented
//! here as a free function using the crate's own public `k2`/`mask`/`l` fields and `norm`/
//! `energy` methods, so it needs no crate changes. **Its random phase does not use the same RNG
//! as numpy** — matching numpy's bit stream was not attempted, since the physics this measures is
//! ensemble statistics at a target energy, not a specific trajectory; each `(e, seed)` label is
//! independently reproducible under this program, not literally identical to a Python run with
//! the same numbers. Documented here so nobody mistakes one for the other.
//!
//! 3-way concurrency (not 6, to reduce peak memory after the unexplained loss of the 6-way run)
//! via a `rayon` thread pool.

use num_complex::Complex64;
use qf_pgpe::ComplexField2D;
use rayon::ThreadPoolBuilder;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Overridable via env vars for smoke-testing at a tiny scale (`QF_R3_N=16 QF_R3_L=8
/// QF_R3_T_TR=1.0 QF_R3_T_END=2.0 QF_R3_SNAPSHOT_EVERY_T=0.2 QF_R3_CHECKPOINT_EVERY_STEPS=10`);
/// the real run uses the defaults below, matching `run_r3.py`'s C3 spec exactly.
fn env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn n() -> usize {
    env_or("QF_R3_N", 384)
}
fn l() -> f64 {
    env_or("QF_R3_L", 192.0)
}
const G: f64 = 1.0;
const DT: f64 = 0.01;
fn t_tr() -> f64 {
    env_or("QF_R3_T_TR", 6000.0)
}
fn t_end() -> f64 {
    env_or("QF_R3_T_END", 7000.0)
}
fn snapshot_every_t() -> f64 {
    env_or("QF_R3_SNAPSHOT_EVERY_T", 10.0)
}
fn checkpoint_every_steps() -> u64 {
    env_or("QF_R3_CHECKPOINT_EVERY_STEPS", 20_000)
}
fn concurrency() -> usize {
    env_or("QF_R3_CONCURRENCY", 3)
}

struct Config {
    e: f64,
    seed: u64,
    name: String,
}

fn configs() -> Vec<Config> {
    let mut out = Vec::new();
    for &e in &[1.00, 1.10, 1.20] {
        for &seed in &[11u64, 12u64] {
            out.push(Config {
                e,
                seed,
                name: format!("C3_L192_e{e:.2}_s{seed}"),
            });
        }
    }
    out
}

/// A xorshift64* generator, seeded per-config; matches this program's random_state to a
/// reproducible-but-not-numpy-bit-identical stream (see the module docs).
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xD1B54A32D192ED03)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn uniform01(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Random projected field with norm `n0 * L^2` and energy per particle `e_target`, by rescaling a
/// random-phase spectrum `|c_k| ~ exp(-k^2/(2 s^2))` and bisecting on `s` -- the same algorithm as
/// `pgpe.py`'s `PGPE.random_state`, using only this crate's public fields/methods.
fn random_state(
    field: &ComplexField2D,
    n0: f64,
    e_target: f64,
    seed: u64,
    iters: usize,
) -> Vec<Complex64> {
    let mut rng = Rng::new(seed);
    let n2 = field.n * field.n;
    let phase: Vec<Complex64> = (0..n2)
        .map(|_| Complex64::from_polar(1.0, 2.0 * std::f64::consts::PI * rng.uniform01()))
        .collect();
    let ntot = n0 * field.l * field.l;

    let make = |s: f64| -> Vec<Complex64> {
        let mut c: Vec<Complex64> = field
            .k2
            .iter()
            .zip(&field.mask)
            .zip(&phase)
            .map(|((&k2v, &m), &ph)| {
                if m {
                    (-k2v / (2.0 * s * s)).exp() * ph
                } else {
                    Complex64::new(0.0, 0.0)
                }
            })
            .collect();
        let scale = (ntot / field.norm(&c)).sqrt();
        for v in c.iter_mut() {
            *v *= scale;
        }
        c
    };

    let (mut lo, mut hi) = (0.02, field.kcut);
    for _ in 0..iters {
        let mid = 0.5 * (lo + hi);
        let e = field.energy(&make(mid)) / ntot;
        if e < e_target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    make(0.5 * (lo + hi))
}

fn write_complex_raw(path: &Path, c: &[Complex64]) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(c.len() * 16);
    for z in c {
        buf.extend_from_slice(&z.re.to_le_bytes());
        buf.extend_from_slice(&z.im.to_le_bytes());
    }
    let tmp = path.with_extension("raw.tmp");
    fs::write(&tmp, &buf)?;
    fs::rename(&tmp, path) // atomic on the same filesystem: never leaves a half-written checkpoint
}

fn read_complex_raw(path: &Path, n2: usize) -> std::io::Result<Vec<Complex64>> {
    let mut f = fs::File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    assert_eq!(buf.len(), n2 * 16, "checkpoint size mismatch for {path:?}");
    Ok((0..n2)
        .map(|i| {
            let re = f64::from_le_bytes(buf[i * 16..i * 16 + 8].try_into().unwrap());
            let im = f64::from_le_bytes(buf[i * 16 + 8..i * 16 + 16].try_into().unwrap());
            Complex64::new(re, im)
        })
        .collect())
}

fn checkpoint_paths(out_dir: &Path, name: &str) -> (PathBuf, PathBuf) {
    (
        out_dir.join(format!("{name}.checkpoint.raw")),
        out_dir.join(format!("{name}.checkpoint.meta")),
    )
}

fn save_checkpoint(out_dir: &Path, name: &str, c: &[Complex64], steps_done: u64, t_sim: f64) {
    let (raw, meta) = checkpoint_paths(out_dir, name);
    write_complex_raw(&raw, c).expect("write checkpoint raw");
    let meta_tmp = meta.with_extension("meta.tmp");
    fs::write(&meta_tmp, format!("{steps_done} {t_sim}\n")).expect("write checkpoint meta");
    fs::rename(&meta_tmp, &meta).expect("rename checkpoint meta");
}

/// `Some((c, steps_done, t_sim))` if a checkpoint exists for this config.
fn load_checkpoint(out_dir: &Path, name: &str, n2: usize) -> Option<(Vec<Complex64>, u64, f64)> {
    let (raw, meta) = checkpoint_paths(out_dir, name);
    let meta_text = fs::read_to_string(&meta).ok()?;
    let mut it = meta_text.split_whitespace();
    let steps_done: u64 = it.next()?.parse().ok()?;
    let t_sim: f64 = it.next()?.parse().ok()?;
    let c = read_complex_raw(&raw, n2).ok()?;
    Some((c, steps_done, t_sim))
}

fn run_one(cfg: &Config, out_dir: &Path) {
    let (n, l, t_tr, t_end, snapshot_every_t, checkpoint_every_steps) = (
        n(),
        l(),
        t_tr(),
        t_end(),
        snapshot_every_t(),
        checkpoint_every_steps(),
    );
    let field = ComplexField2D::new(n, l, G, DT);
    let n2 = n * n;
    let total_steps = (t_end / DT).round() as u64;
    let warmup_steps = (t_tr / DT).round() as u64;
    let snapshot_every_steps = (snapshot_every_t / DT).round() as u64;

    let t0 = Instant::now();
    let (mut c, mut steps_done) = match load_checkpoint(out_dir, &cfg.name, n2) {
        Some((c, steps, t)) => {
            println!(
                "{}: resuming from checkpoint at step {steps} (t={t:.1})",
                cfg.name
            );
            (c, steps)
        }
        None => {
            println!(
                "{}: no checkpoint, building random_state(n0=1.0, e_target={})",
                cfg.name, cfg.e
            );
            (random_state(&field, 1.0, cfg.e, cfg.seed, 60), 0)
        }
    };
    let e_initial = field.energy(&c);

    while steps_done < total_steps {
        c = field.step(&c);
        steps_done += 1;
        let t_sim = steps_done as f64 * DT;

        if steps_done > warmup_steps
            && (steps_done - warmup_steps).is_multiple_of(snapshot_every_steps)
        {
            let psi = field.psi(&c);
            let path = out_dir.join(format!("{}_sample_t{:07.1}.raw", cfg.name, t_sim));
            write_complex_raw(&path, &psi).expect("write snapshot");
        }

        if steps_done.is_multiple_of(checkpoint_every_steps) || steps_done == total_steps {
            save_checkpoint(out_dir, &cfg.name, &c, steps_done, t_sim);
        }
    }

    let e_final = field.energy(&c);
    let meta = format!(
        "{{\"name\": \"{}\", \"e_target\": {}, \"seed\": {}, \"N\": {n}, \"L\": {l}, \"g\": {G}, \"dt\": {DT}, \
         \"t_tr\": {t_tr}, \"t_end\": {t_end}, \"snapshot_every_t\": {snapshot_every_t}, \
         \"energy_initial\": {e_initial}, \"energy_final\": {e_final}, \
         \"drift_E\": {}, \"steps\": {steps_done}, \"wall_seconds\": {:.1}}}\n",
        cfg.name,
        cfg.e,
        cfg.seed,
        ((e_final - e_initial) / e_initial).abs(),
        t0.elapsed().as_secs_f64()
    );
    fs::write(out_dir.join(format!("{}_meta.json", cfg.name)), meta).expect("write meta json");
    println!(
        "{}: done. steps={steps_done} E {e_initial:.3}->{e_final:.3} drift={:.1e} wall={:.1}s",
        cfg.name,
        ((e_final - e_initial) / e_initial).abs(),
        t0.elapsed().as_secs_f64()
    );
}

/// A standalone check of `random_state`'s bisection on a small, fast grid: does it converge to
/// the requested energy-per-particle target, and does the resulting field have the requested
/// norm? Not a comparison against Python's numbers (the RNG streams differ by design -- see the
/// module docs), just a check that this implementation's own bisection does what it claims.
fn validate() {
    let field = ComplexField2D::new(32, 16.0, 1.0, 0.01);
    let mut worst_e = 0.0f64;
    let mut worst_n = 0.0f64;
    for &e_target in &[0.5, 1.0, 1.5, 2.0] {
        for seed in [1u64, 2, 3] {
            let c = random_state(&field, 1.0, e_target, seed, 60);
            let ntot = 1.0 * field.l * field.l;
            let e_rel = (field.energy(&c) / ntot - e_target).abs() / e_target;
            let n_rel = (field.norm(&c) - ntot).abs() / ntot;
            worst_e = worst_e.max(e_rel);
            worst_n = worst_n.max(n_rel);
            println!(
                "e_target={e_target} seed={seed}: energy/N rel err {e_rel:.2e}, norm rel err {n_rel:.2e}"
            );
        }
    }
    let ok = worst_e < 1e-3 && worst_n < 1e-9;
    println!("worst energy rel err: {worst_e:.2e}, worst norm rel err: {worst_n:.2e}");
    println!("VALIDATE: {}", if ok { "PASS" } else { "FAIL" });
    std::process::exit(if ok { 0 } else { 1 });
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("validate") {
        validate();
        return;
    }
    let out_dir_arg = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let out_dir = PathBuf::from(out_dir_arg);
    fs::create_dir_all(&out_dir).expect("create output directory");

    let cfgs = configs();
    let pool = ThreadPoolBuilder::new()
        .num_threads(concurrency())
        .build()
        .expect("build thread pool");
    pool.install(|| {
        use rayon::prelude::*;
        cfgs.par_iter().for_each(|cfg| run_one(cfg, &out_dir));
    });
    println!("all configurations complete");
}
