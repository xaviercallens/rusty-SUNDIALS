# Solving Your First ODE with rusty-SUNDIALS

Welcome to rusty-SUNDIALS! This tutorial will walk you through solving a simple Ordinary Differential Equation (ODE) using our formally verified, pure-Rust solver.

## 1. The Mathematical Problem

We will solve the **Exponential Decay** equation:
$$ \frac{dy}{dt} = -k \cdot y $$

Where $y(0) = 1.0$ and $k = 0.5$.
The exact analytical solution is $y(t) = e^{-0.5t}$.

## 2. Setting up the Rust Project

Create a new binary project:
```bash
cargo new first_ode
cd first_ode
```

Add the rusty-SUNDIALS crates to your `Cargo.toml`:
```toml
[dependencies]
cvode = { git = "https://github.com/xaviercallens/rusty-SUNDIALS" }
nvector = { git = "https://github.com/xaviercallens/rusty-SUNDIALS" }
```

## 3. Writing the Code

Open `src/main.rs` and add the following code:

```rust
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

fn main() -> Result<(), cvode::CvodeError> {
    // 1. Define the right-hand side (RHS) of the ODE
    // Signature: FnMut(t: f64, y: &[f64], f: &mut [f64]) -> Result<(), String>
    let k = 0.5;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| {
        f[0] = -k * y[0];
        Ok(())
    };

    // 2. Set the initial condition: y(0) = 1.0
    let y0 = SerialVector::from_slice(&[1.0]);

    // 3. Configure the CVODE solver
    // We use the BDF (Backward Differentiation Formula) method, which is great for stiff equations,
    // though Adams could also be used here.
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-4)        // Relative tolerance
        .atol(1e-6)        // Absolute tolerance
        .build(rhs, 0.0, y0)?; // Build the solver starting at t=0.0

    // 4. Integrate over time
    println!("Time (t) | y(t)     | Exact    | Error");
    println!("-----------------------------------------");
    
    for i in 1..=5 {
        let t_target = i as f64;
        let (t_reached, y) = solver.solve(t_target, Task::Normal)?;
        
        let exact = f64::exp(-k * t_reached);
        let error = (y[0] - exact).abs();
        
        println!("{:.1}      | {:.6} | {:.6} | {:.2e}", t_reached, y[0], exact, error);
    }

    Ok(())
}
```

## 4. Run and Verify

Execute the program:
```bash
cargo run
```

You should see output similar to this:
```
Time (t) | y(t)     | Exact    | Error
-----------------------------------------
1.0      | 0.606531 | 0.606531 | 1.2e-6
2.0      | 0.367879 | 0.367879 | 4.5e-6
3.0      | 0.223130 | 0.223130 | 2.1e-6
4.0      | 0.135335 | 0.135335 | 8.9e-7
5.0      | 0.082085 | 0.082085 | 3.4e-7
```

Congratulations! You've just run a formally verified integration step using rusty-SUNDIALS. The error is well within the requested tolerances.

## Next Steps
- Learn about **Stiff Equations** using the Robertson chemical kinetics benchmark.
- Discover how to use **Parallel Vectors** for large-scale PDEs.
