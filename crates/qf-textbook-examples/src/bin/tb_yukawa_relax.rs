//! Appendix F, example 8: relaxing a small Yukawa crystal (`kappa = 2`, 64 particles, density 1)
//! with CVODE (BDF) under the exact image-summed potential of `qf-yukawa-crystal`.
//!
//! Three starts:
//! * `tri_perturbed`: the 8 x 8 triangular lattice with random displacements (0.15 spacing) in its
//!   commensurate box; positive control: it must return to the exact lattice energy;
//! * `random`: a uniform random start (minimum distance 0.5) in the same box; it ends at or above
//!   the lattice energy;
//! * `square_perturbed`: the 8 x 8 square lattice, slightly perturbed, in an 8 x 8 box that cannot
//!   hold the triangular lattice; negative control: it must fall below the square-lattice energy
//!   (the square lattice is not a minimum) but stay above the triangular one.
//!
//! Usage: `tb_yukawa_relax [OUT_DIR]` (writes `yukawa_<start>_trace.csv` and `_final.csv`).

use qf_yukawa_crystal::exact::{ExactBox, lattice_energy_exact, square_basis, tri_basis};
use qf_yukawa_crystal::{Rng, spacing_density_one, square, triangular};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const KAPPA: f64 = 2.0;
const RCUT: f64 = 12.0;

fn random_start(lx: f64, ly: f64, n: usize, dmin: f64, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut x: Vec<f64> = Vec::with_capacity(2 * n);
    while x.len() < 2 * n {
        let (px, py) = (rng.uniform() * lx, rng.uniform() * ly);
        let far = x.chunks_exact(2).all(|q| {
            let dx = (px - q[0]) - lx * ((px - q[0]) / lx).round();
            let dy = (py - q[1]) - ly * ((py - q[1]) / ly).round();
            dx * dx + dy * dy >= dmin * dmin
        });
        if far {
            x.extend([px, py]);
        }
    }
    x
}

fn perturb(x: &[f64], amp: f64, seed: u64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    x.iter()
        .map(|&v| v + amp * (2.0 * rng.uniform() - 1.0))
        .collect()
}

fn save(out: &Path, name: &str, x: &[f64], trace: &[(f64, f64, f64)]) {
    let mut csv = String::from("t,energy_per_particle,max_force\n");
    for (t, e, f) in trace {
        let _ = writeln!(csv, "{t},{e:.17e},{f:.6e}");
    }
    std::fs::write(out.join(format!("yukawa_{name}_trace.csv")), csv).expect("write trace");
    let mut csv = String::from("x,y\n");
    for p in x.chunks_exact(2) {
        let _ = writeln!(csv, "{:.10},{:.10}", p[0], p[1]);
    }
    std::fs::write(out.join(format!("yukawa_{name}_final.csv")), csv).expect("write final");
}

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&out).expect("create OUT_DIR");
    let a = spacing_density_one();
    let e_tri = lattice_energy_exact(KAPPA, tri_basis(), RCUT).partial;
    let e_sq = lattice_energy_exact(KAPPA, square_basis(), RCUT).partial;
    println!(
        "exact lattice energies (kappa = 2, R = 12): E_tri = {e_tri:.15}, E_square = {e_sq:.15}"
    );
    let (lx, ly) = (8.0 * a, 4.0 * 3f64.sqrt() * a);
    let tri_box = ExactBox::new(KAPPA, RCUT, lx, ly);
    let sq_box = ExactBox::new(KAPPA, RCUT, 8.0, 8.0);
    let starts = [
        (
            "tri_perturbed",
            &tri_box,
            perturb(&triangular(8, 8, a), 0.15 * a, 7),
        ),
        ("random", &tri_box, random_start(lx, ly, 64, 0.5 * a, 11)),
        (
            "square_perturbed",
            &sq_box,
            perturb(&square(8, 1.0), 0.02, 13),
        ),
    ];
    let mut ok = true;
    for (name, sys, x0) in starts {
        let (x, trace) =
            qf_textbook_examples::relax_traced(sys, &x0, 1e-9, 1e6).expect("relaxation");
        let (t, e, f) = *trace.last().expect("non-empty trace");
        println!(
            "{name:<17} E/N: start {:.12} -> end {e:.15}  E/N - E_tri = {:+.3e}  max|F| = {f:.1e}  t = {t}",
            trace[0].1,
            e - e_tri
        );
        ok &= f < 1e-9;
        ok &= match name {
            "tri_perturbed" => (e - e_tri).abs() < 1e-10,
            "random" => e >= e_tri - 1e-12,
            _ => e < e_sq - 1e-6 && e > e_tri,
        };
        save(&out, name, &x, &trace);
    }
    println!("controls as expected: {ok}");
    std::process::exit(if ok { 0 } else { 1 });
}
