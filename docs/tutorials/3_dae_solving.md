# Tutorial 3: DAE Solving with IDA

Differential-Algebraic Equations (DAEs) involve both differential equations (like ODEs) and algebraic constraints. `rusty-SUNDIALS` includes the `ida` crate, translated from SUNDIALS IDA, specifically designed for index-1 DAEs.

## 1. The Pendulum (Index-1 DAE)

Consider a pendulum in Cartesian coordinates (x, y) with velocity (u, v) and tension $\lambda$.
The algebraic constraint is that the length is constant: $x^2 + y^2 - L^2 = 0$.

```rust
use ida::{Ida, Task};
use nvector::SerialVector;

fn main() -> Result<(), ida::IdaError> {
    // State vector: [x, y, u, v, lambda]
    // Derivative vector: [dx, dy, du, dv, dlambda]
    
    let res = |t: f64, y: &[f64], yp: &[f64], r: &mut [f64]| -> Result<(), String> {
        let l = 1.0;
        let g = 9.81;
        
        // Differential parts: x' = u, y' = v
        r[0] = yp[0] - y[2];
        r[1] = yp[1] - y[3];
        
        // Momentum parts
        r[2] = yp[2] + y[0]*y[4];       // u' = -x*lambda
        r[3] = yp[3] + y[1]*y[4] + g;   // v' = -y*lambda - g
        
        // Algebraic constraint: x^2 + y^2 - L^2 = 0
        r[4] = y[0]*y[0] + y[1]*y[1] - l*l;
        
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0, 0.0, 0.0, 0.0, 0.0]);
    let yp0 = SerialVector::from_slice(&[0.0, 0.0, 0.0, -9.81, 0.0]);

    // IDA uses BDF methods implicitly
    let mut solver = Ida::builder()
        .rtol(1e-6)
        .atol(1e-8)
        .build(res, 0.0, y0, yp0)?;

    // Integrate
    for i in 1..=5 {
        let (t, y, _yp) = solver.solve(i as f64, Task::Normal)?;
        println!("t={:.1} x={:.3} y={:.3}", t, y[0], y[1]);
    }
    
    Ok(())
}
```

DAE solvers are powerful tools for constrained mechanical systems, chemical equilibria, and electrical circuits.
