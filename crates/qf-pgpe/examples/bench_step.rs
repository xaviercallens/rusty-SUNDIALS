//! Micro-benchmark of the IF-RK4 step: `cargo run --release -p qf-pgpe --example bench_step -- [N] [steps] [reps]`.
//!
//! Prints one JSON line: the best and the median time per step over `reps` repetitions of `steps` steps (best-of-n is
//! the right statistic on a shared machine), the thread count (always 1: the step is single-threaded) and a checksum `sum |Re c| + |Im c|` of the final state, so that optimisations can be checked for
//! bit-level regressions and compared with the Python engines (`exploration/pgpe/bench_engines.py` reproduces the
//! same initial state and checksum).
//!
//! Initial state: `c[0] = N^2` (uniform condensate), `c[1] = (0.3, 0.1) N`, `c[N] = (-0.2, 0.2) N`; `L = N/2`
//! (`dx = 0.5`), `g = 1`, `dt = 0.01`, cutoff `k_max/2`.
use num_complex::Complex64;
use qf_pgpe::ComplexField2D;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = a.get(1).map_or(128, |s| s.parse().unwrap());
    let steps: usize = a.get(2).map_or(200, |s| s.parse().unwrap());
    let reps: usize = a.get(3).map_or(3, |s| s.parse().unwrap());
    let f = ComplexField2D::new(n, n as f64 / 2.0, 1.0, 0.01);
    let mut c0 = vec![Complex64::new(0.0, 0.0); n * n];
    c0[0] = Complex64::new((n * n) as f64, 0.0);
    c0[1] = Complex64::new(0.3 * n as f64, 0.1 * n as f64);
    c0[n] = Complex64::new(-0.2 * n as f64, 0.2 * n as f64);
    let c0 = f.modes(&f.psi(&c0)); // projected (and a warm-up of the plans)
    let mut times = Vec::new();
    let mut last = c0.clone();
    for _ in 0..reps {
        let t0 = Instant::now();
        last = f.run(&c0, steps as f64 * f.dt);
        times.push(t0.elapsed().as_secs_f64() / steps as f64);
    }
    times.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let sum: f64 = last.iter().map(|v| v.re.abs() + v.im.abs()).sum();
    let threads = 1;
    println!(
        "{{\"engine\":\"qf-pgpe\",\"n\":{n},\"steps\":{steps},\"reps\":{reps},\"us_per_step_best\":{:.1},\"us_per_step_median\":{:.1},\"threads\":{threads},\"checksum\":{sum:.12e},\"norm\":{:.12e}}}",
        1e6 * times[0],
        1e6 * times[times.len() / 2],
        f.norm(&last)
    );
}
