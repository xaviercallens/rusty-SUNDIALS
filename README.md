# rusty-SUNDIALS

**Production-quality ODE solver in pure Rust** — a SpecToRust translation of [SUNDIALS CVODE](https://computing.llnl.gov/projects/sundials) from Lawrence Livermore National Laboratory.

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD--3--Clause-blue.svg)](LICENSE)

## What is this?

rusty-SUNDIALS is an idiomatic Rust implementation of the CVODE solver from the SUNDIALS suite (~44K LOC C → Rust). CVODE solves initial value problems for ordinary differential equation (ODE) systems using:

- **BDF methods** (orders 1-5) for stiff problems
- **Adams-Moulton methods** (orders 1-12) for non-stiff problems
- **Newton iteration** with direct or iterative linear solvers for implicit methods
- **Functional iteration** for explicit methods
- **Root-finding** capability during integration
- **Adaptive step size control** with error-based order selection

## Why Rust?

The Rust ecosystem lacks a production-quality, general-purpose ODE solver at the level of SUNDIALS. This project fills that gap with:

- **Memory safety** without garbage collection
- **Zero-cost abstractions** for numerical types
- **Fearless concurrency** for parallel vector operations
- **No unsafe code** in the public API
- **Type-safe error handling** via `Result<T, CvodeError>`

## Architecture

```
rusty-sundials/
├── crates/
│   ├── sundials-core/    # Core types: Real, Context, error handling
│   ├── nvector/          # N_Vector trait + serial implementation
│   └── cvode/           # CVODE solver (BDF + Adams methods)
└── examples/            # Usage examples
```

## Quick Start

```rust
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

fn main() -> Result<(), cvode::CvodeError> {
    // dy/dt = -0.04*y1 + 1e4*y2*y3
    // Robertson chemical kinetics (stiff system)
    let rhs = |t: f64, y: &[f64], ydot: &mut [f64]| {
        ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        ydot[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        ydot[2] = 3e7 * y[1] * y[1];
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0, 0.0, 0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-4)
        .atol(1e-8)
        .build(rhs, 0.0, y0)?;

    let (t, y) = solver.solve(0.4, Task::Normal)?;
    println!("t = {t:.4e}, y = [{:.4e}, {:.4e}, {:.4e}]", y[0], y[1], y[2]);
    Ok(())
}
```

## Translation Methodology

This project was generated using the **SpecToRust** pipeline from the [SocrateAgora](https://github.com/xaviercallens/socrateagora) platform:

1. **Specification extraction** — Public API contracts extracted from C headers
2. **Idiomatic Rust design** — Ownership model mapped (void* → typed structs, manual memory → RAII)
3. **Implementation** — Core algorithms translated preserving numerical behavior
4. **Verification** — Equivalence testing against reference SUNDIALS outputs

## Status

| Component | Status | Notes |
|-----------|--------|-------|
| Core types (Real, Context) | ✅ | Complete |
| N_Vector trait | ✅ | Serial implementation |
| CVODE BDF solver | ✅ | Orders 1-5 |
| CVODE Adams solver | ✅ | Orders 1-12 |
| Step size control | ✅ | Adaptive with error estimation |
| Root finding | ✅ | Event detection during integration |
| Dense linear solver | ✅ | Direct dense solve |
| Band linear solver | 🟡 | In progress |
| Iterative solvers (GMRES) | 🔴 | Planned |
| Parallel N_Vector | 🔴 | Planned |

## License

BSD-3-Clause — same as the original SUNDIALS.

## Acknowledgments

Based on [SUNDIALS](https://computing.llnl.gov/projects/sundials) by Lawrence Livermore National Laboratory.
Original authors: Scott D. Cohen, Alan C. Hindmarsh, Radu Serban, and contributors.
