//! Thermal base state generator (port of `make_friction_bases.py` / `make_transport_bases.py`): heat a base field to a
//! target energy per particle, equilibrate, measure (`T`, `n_s/n`, raw vortex count), and write the final field.
//!
//!     cargo run --release -p qf-pgpe --example thermal_base -- --n 128 --l 64 --kcut-frac 0.5 --e 0.60 \
//!         --t-tr 1000 --t-end 1500 --seed 1 [--from BASE.raw --from-n 128] --out PREFIX
//!
//! Without `--from` the start is the vortex-free uniform condensate (energy per particle 0.5 at unit density).
//! With `--from`, `BASE.raw` (complex128 little endian, `from-n` squared values, projected amplitudes) is
//! Fourier-resampled to `--n` on the same box, re-projected under the new cutoff and renormalised to the original
//! norm, exactly as `make_friction_bases.py`. Writes `PREFIX.raw` (the layout `vortex_transport` reads) and
//! `PREFIX.json`. Heating noise comes from `thermal::Rng` (not numpy-identical).
use qf_pgpe::ComplexField2D;
use qf_pgpe::thermal::{make_base, read_raw, resample, write_raw};
use qf_pgpe::vortex::uniform_condensate;
use std::path::Path;

struct Args {
    n: usize,
    l: f64,
    kcut_frac: f64,
    e: f64,
    t_tr: f64,
    t_end: f64,
    seed: u64,
    from: Option<String>,
    from_n: usize,
    out: String,
}

fn parse() -> Args {
    let a: Vec<String> = std::env::args().collect();
    let mut r = Args {
        n: 128,
        l: 64.0,
        kcut_frac: 0.5,
        e: 0.60,
        t_tr: 1000.0,
        t_end: 1500.0,
        seed: 1,
        from: None,
        from_n: 128,
        out: String::new(),
    };
    let mut i = 1;
    while i < a.len() {
        let v = a.get(i + 1).cloned().unwrap_or_default();
        match a[i].as_str() {
            "--n" => r.n = v.parse().unwrap(),
            "--l" => r.l = v.parse().unwrap(),
            "--kcut-frac" => r.kcut_frac = v.parse().unwrap(),
            "--e" => r.e = v.parse().unwrap(),
            "--t-tr" => r.t_tr = v.parse().unwrap(),
            "--t-end" => r.t_end = v.parse().unwrap(),
            "--seed" => r.seed = v.parse().unwrap(),
            "--from" => r.from = Some(v),
            "--from-n" => r.from_n = v.parse().unwrap(),
            "--out" => r.out = v,
            other => panic!("unknown argument {other}"),
        }
        i += 2;
    }
    assert!(!r.out.is_empty(), "--out PREFIX is required");
    r
}

fn main() {
    let a = parse();
    let t0 = std::time::Instant::now();
    let f = ComplexField2D::with_kcut_frac(a.n, a.l, 1.0, 0.01, a.kcut_frac);
    let c0 = match &a.from {
        None => uniform_condensate(&f),
        Some(p) => {
            let f0 = ComplexField2D::new(a.from_n, a.l, 1.0, 0.01);
            let c_in = read_raw(Path::new(p), a.from_n * a.from_n).expect("read base");
            let psi = resample(&f0.psi(&c_in), a.n);
            let mut c = f.modes(&psi);
            let s = (f0.norm(&c_in) / f.norm(&c)).sqrt();
            c.iter_mut().for_each(|v| *v *= s);
            c
        }
    };
    let b = make_base(&f, &c0, a.e, a.t_tr, a.t_end, a.seed).expect("make_base");
    let s = &b.summary;
    write_raw(Path::new(&format!("{}.raw", a.out)), &b.c).expect("write raw");
    let blocks: Vec<String> = s
        .blocks
        .iter()
        .map(|k| {
            format!(
                "{{\"t0\":{},\"T\":{},\"cond\":{},\"n_v\":{},\"JL\":{},\"JT\":{},\"ns_over_n\":{}}}",
                k.t0, k.t_thermo, k.cond, k.n_v, k.jl, k.jt, k.ns_over_n
            )
        })
        .collect();
    let json = format!(
        "{{\n \"n\":{},\"L\":{},\"kcut_frac\":{},\"kcut\":{},\"e\":{},\"e_start\":{},\"E_per_particle\":{},\"drift_E\":{},\n \"t_tr\":{},\"t_end\":{},\"seed\":{},\n \"T\":{},\"T_lowwindow\":{},\"ns_over_n\":{},\"n_v\":{},\"cond\":{},\"JL\":{},\"JT\":{},\"admitted\":{},\n \"seconds\":{:.1},\n \"blocks\":[{}]\n}}\n",
        a.n,
        a.l,
        a.kcut_frac,
        f.kcut,
        a.e,
        b.e_start,
        b.e_per_particle,
        b.drift_e,
        a.t_tr,
        a.t_end,
        a.seed,
        s.t_thermo,
        s.t_lowwindow,
        s.ns_over_n,
        s.n_v,
        s.cond,
        s.jl,
        s.jt,
        s.n_v < 0.5,
        t0.elapsed().as_secs_f64(),
        blocks.join(",")
    );
    std::fs::write(format!("{}.json", a.out), json).expect("write json");
    println!(
        "{}: T = {:.4}, n_s/n = {:.4}, raw n_v = {:.2}, e_start {:.3}, drift {:.1e}, {:.1} s",
        a.out,
        s.t_thermo,
        s.ns_over_n,
        s.n_v,
        b.e_start,
        b.drift_e,
        t0.elapsed().as_secs_f64()
    );
}
