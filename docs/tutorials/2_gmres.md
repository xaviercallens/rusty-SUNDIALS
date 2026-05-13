# Tutorial 2: Preconditioned GMRES for Large Systems

For large systems (N > 10,000), dense and banded linear solvers become memory-bound and extremely slow. The Generalised Minimal RESidual (GMRES) method is an iterative Krylov subspace solver that scales beautifully to massive systems.

In `rusty-SUNDIALS`, we can use Jacobian-Free Newton-Krylov (JFNK) combined with GMRES to solve systems without ever explicitly assembling the Jacobian matrix.

## 1. Setup the Problem
Let's consider a large 2D Heat Equation on a 200x200 grid (40,000 ODEs).

```rust
use cvode::{Cvode, Method, Task};
use nvector::ParallelVector; // Use multi-threading for large N
use sundials_core::gmres::GmresSolver;

fn main() -> Result<(), cvode::CvodeError> {
    let nx = 200;
    let ny = 200;
    let n = nx * ny;
    
    // Initial condition
    let y0 = ParallelVector::new(n, 1.0);
    
    // Right hand side (finite difference Laplacian)
    let rhs = move |t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        // ... (compute 5-point stencil)
        Ok(())
    };

    // Configure GMRES linear solver with restart = 20
    let mut ls = GmresSolver::new(n, 20);
    
    // Build CVODE with GMRES
    let mut solver = Cvode::builder(Method::Bdf)
        .linear_solver(ls)
        .rtol(1e-5)
        .atol(1e-8)
        .build(rhs, 0.0, y0)?;

    let (t, y) = solver.solve(10.0, Task::Normal)?;
    println!("Solved to t={}", t);
    
    Ok(())
}
```

## 2. Preconditioning
GMRES often stalls without a good preconditioner. `rusty-SUNDIALS` allows you to provide a `SUNPreconditioner` to accelerate convergence.

*(For detailed examples of ILU or AI-surrogate preconditioning, see the advanced SciML tutorials).*
