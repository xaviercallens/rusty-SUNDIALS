//! Detection-noise dependence of the G0 estimators: `cargo run --release -p qf-pgpe --example g0_scan -- [n_seeds]`.
//! Synthetic tracks with known alpha = 0.02, alpha' = 0.10, eta = 2e-3 (the registered gate G0), detection noise swept over
//! {0, 0.05, 0.1, 0.2} per coordinate, mean and spread of the energy estimator and of the regression over the seeds.
//! With `--eta-scan` the detection noise is fixed at 0 and the diffusion `eta` is swept over {0, 5e-4, 1e-3, 2e-3}.
use qf_pgpe::transport::{Options, Track, analyse_tracks, langevin, standard_normal};
use qf_pgpe::vortex::{Lcg, antiparallel};

fn main() {
    let ns: u64 = std::env::args().nth(1).map_or(8, |s| s.parse().unwrap());
    let eta_scan = std::env::args().any(|a| a == "--eta-scan");
    let cases: Vec<(f64, f64)> = if eta_scan {
        [0.0, 5e-4, 1e-3, 2e-3].iter().map(|&e| (0.0, e)).collect()
    } else {
        [0.0, 0.05, 0.1, 0.2].iter().map(|&n| (n, 2e-3)).collect()
    };
    let l = 64.0;
    println!(
        "noise  alpha_energy (mean +- sd)   alpha_regression   1-alpha'   eta   [truth 0.02, 0.90, 2e-3]"
    );
    for (noise, eta) in cases {
        let mut v: Vec<[f64; 4]> = Vec::new();
        for seed in 0..ns {
            let mut rng = Lcg::new(7919 * (seed + 1));
            let mut tracks = Vec::new();
            for _ in 0..8 {
                let (pos, q) = antiparallel(l, 10.0, &mut rng);
                let (t, mut r) = langevin(
                    &pos, &q, l, 0.02, 0.10, eta, 2000.0, &mut rng, 0.05, 1.0, 2.0,
                );
                for row in r.iter_mut() {
                    for p in row.iter_mut() {
                        p.0 += noise * standard_normal(&mut rng);
                        p.1 += noise * standard_normal(&mut rng);
                    }
                }
                tracks.push(Track { t, r, q });
            }
            let e = analyse_tracks(&tracks, l, &Options::default()).unwrap();
            v.push([
                e.alpha_energy,
                e.alpha_regression,
                e.one_minus_alpha_prime,
                e.eta,
            ]);
        }
        let stat = |k: usize| {
            let m = v.iter().map(|x| x[k]).sum::<f64>() / v.len() as f64;
            let sd =
                (v.iter().map(|x| (x[k] - m).powi(2)).sum::<f64>() / (v.len() as f64 - 1.0)).sqrt();
            (m, sd)
        };
        let (a, b, c, d) = (stat(0), stat(1), stat(2), stat(3));
        println!(
            "noise {noise:4.2} eta {eta:.0e}   {:.4} +- {:.4}   {:.4} +- {:.4}   {:.4}   {:.2e}",
            a.0, a.1, b.0, b.1, c.0, d.0
        );
    }
}
