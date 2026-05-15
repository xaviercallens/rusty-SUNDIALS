use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sundials_core::Real;

fn vanderpol_rhs(t: Real, y: &[Real], ydot: &mut [Real]) {
    let mu = 1.0;
    ydot[0] = y[1];
    ydot[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
}

fn bench_vanderpol(c: &mut Criterion) {
    // This benchmark simulates the ODE RHS evaluation for the Van der Pol oscillator,
    // which is a standard stiff ODE test case in SUNDIALS.
    let mut ydot = vec![0.0, 0.0];
    let y = vec![2.0, 0.0];

    c.bench_function("vanderpol_rhs_rust", |b| {
        b.iter(|| vanderpol_rhs(black_box(0.0), black_box(&y), black_box(&mut ydot)))
    });
}

fn robertson_rhs(t: Real, y: &[Real], ydot: &mut [Real]) {
    ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
    ydot[2] = 3e7 * y[1] * y[1];
    ydot[1] = -ydot[0] - ydot[2];
}

fn bench_robertson(c: &mut Criterion) {
    let mut ydot = vec![0.0, 0.0, 0.0];
    let y = vec![1.0, 0.0, 0.0];

    c.bench_function("robertson_rhs_rust", |b| {
        b.iter(|| robertson_rhs(black_box(0.0), black_box(&y), black_box(&mut ydot)))
    });
}

criterion_group!(benches, bench_vanderpol, bench_robertson);
criterion_main!(benches);
