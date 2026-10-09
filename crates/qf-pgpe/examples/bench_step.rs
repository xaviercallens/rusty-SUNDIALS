//! Micro-benchmark of the IF-RK4 step: `cargo run --release -p qf-pgpe --example bench_step -- [N] [steps]`.
//! Prints CPU-time per step (user time of this thread is what matters on a shared machine) and the final norm so
//! that optimisations can be checked for bit-level regressions with `--check`.
use num_complex::Complex64;
use qf_pgpe::ComplexField2D;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = a.get(1).map_or(128, |s| s.parse().unwrap());
    let steps: usize = a.get(2).map_or(400, |s| s.parse().unwrap());
    let f = ComplexField2D::new(n, n as f64 / 2.0, 1.0, 0.01);
    let mut c = vec![Complex64::new(0.0, 0.0); n * n];
    c[0] = Complex64::new((n * n) as f64, 0.0);
    c[1] = Complex64::new(0.3 * n as f64, 0.1 * n as f64);
    c[n] = Complex64::new(-0.2 * n as f64, 0.2 * n as f64);
    let t0 = Instant::now();
    c = f.run(&c, steps as f64 * f.dt);
    let dt = t0.elapsed().as_secs_f64();
    let sum: f64 = c.iter().map(|v| v.re.abs() + v.im.abs()).sum();
    println!(
        "N = {n}: {:.1} us/step ({steps} steps, {dt:.2} s wall); checksum {sum:.12e}; norm {:.12e}",
        1e6 * dt / steps as f64,
        f.norm(&c)
    );
}
