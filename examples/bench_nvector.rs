//! Benchmark: SIMD vs Serial N_Vector on representative operations.
//!
//! Run with:
//!   RUSTFLAGS="-C target-cpu=native" cargo run --release --example bench_nvector
//!
//! This demonstrates the hardware-level throughput gains on modern
//! Apple Silicon (NEON) and x86_64 (AVX-512) processors.

use nvector::{NVector, ParallelVector, SerialVector, SimdVector};

fn wall_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as u64
}

fn bench<F: FnMut()>(label: &str, reps: usize, mut f: F) {
    let t0 = std::time::Instant::now();
    for _ in 0..reps {
        f();
    }
    let dt = t0.elapsed().as_micros();
    println!("  {label:<40} {dt:>8} µs  ({} reps)", reps);
}

fn main() {
    let n = 1_000_000usize; // 1M state variables — typical large PDE mesh
    let reps = 20;

    println!("=== N_Vector Benchmark: N = {n}, reps = {reps} ===\n");

    // ── Serial baseline ────────────────────────────────────────────────────────
    {
        let x = SerialVector::filled(n, 1.0);
        let y = SerialVector::filled(n, 2.0);
        let mut z = SerialVector::new(n);
        let w = SerialVector::filled(n, 0.5);

        println!("[ SerialVector ]");
        bench("linear_sum(3·x + 4·y)", reps, || {
            SerialVector::linear_sum(3.0, &x, 4.0, &y, &mut z);
        });
        bench("dot(x, y)", reps, || {
            let _ = x.dot(&y);
        });
        bench("wrms_norm(x, w)", reps, || {
            let _ = x.wrms_norm(&w);
        });
        bench("scale(0.5, x, z)", reps, || {
            SerialVector::scale(0.5, &x, &mut z);
        });
        println!();
    }

    // ── SIMD (auto-vectorised chunks) ──────────────────────────────────────────
    {
        let x = SimdVector::filled(n, 1.0);
        let y = SimdVector::filled(n, 2.0);
        let mut z = SimdVector::new(n);
        let w = SimdVector::filled(n, 0.5);

        println!("[ SimdVector (LANE=8, target-cpu=native) ]");
        bench("linear_sum(3·x + 4·y)", reps, || {
            SimdVector::linear_sum(3.0, &x, 4.0, &y, &mut z);
        });
        bench("dot(x, y)", reps, || {
            let _ = x.dot(&y);
        });
        bench("wrms_norm(x, w)", reps, || {
            let _ = x.wrms_norm(&w);
        });
        bench("scale(0.5, x, z)", reps, || {
            SimdVector::scale(0.5, &x, &mut z);
        });
        println!();
    }

    // ── Parallel (Rayon, all CPU cores) ───────────────────────────────────────
    {
        let x = ParallelVector::filled(n, 1.0);
        let y = ParallelVector::filled(n, 2.0);
        let mut z = ParallelVector::new(n);
        let w = ParallelVector::filled(n, 0.5);

        println!(
            "[ ParallelVector (rayon, {} threads) ]",
            rayon::current_num_threads()
        );
        bench("linear_sum(3·x + 4·y)", reps, || {
            ParallelVector::linear_sum(3.0, &x, 4.0, &y, &mut z);
        });
        bench("dot(x, y)", reps, || {
            let _ = x.dot(&y);
        });
        bench("wrms_norm(x, w)", reps, || {
            let _ = x.wrms_norm(&w);
        });
        bench("scale(0.5, x, z)", reps, || {
            ParallelVector::scale(0.5, &x, &mut z);
        });
        println!();
    }

    println!("Done. Compile with RUSTFLAGS=\"-C target-cpu=native\" for maximum vectorisation.");
}
