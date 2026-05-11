//! Lotka-Volterra Predator-Prey — Lotka (1925), Volterra (1926).
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let a=1.5; let b=1.0; let d=1.0; let g=3.0;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        f[0]=a*y[0]-b*y[0]*y[1]; f[1]=d*y[0]*y[1]-g*y[1]; Ok(())
    };
    let x0=10.0; let y0v=5.0;
    let h0 = d*x0-g*x0.ln()+b*y0v-a*y0v.ln();
    let y0 = SerialVector::from_slice(&[x0, y0v]);
    let mut solver = Cvode::builder(Method::Adams)
        .rtol(1e-3).atol(1e-5).max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("Lotka-Volterra (α={a}, β={b}, δ={d}, γ={g})");
    println!("{:>8} {:>12} {:>12} {:>14}","t","prey","predator","ΔH/H₀");
    println!("{}", "-".repeat(50));
    for i in 1..=20 {
        let (t,y) = solver.solve(0.5*i as f64, Task::Normal)?;
        let h = d*y[0]-g*y[0].ln()+b*y[1]-a*y[1].ln();
        println!("{t:8.2} {:12.4} {:12.4} {:14.4e}",y[0],y[1],(h-h0)/h0);
    }
    println!("\nSteps: {}, RHS evals: {}", solver.num_steps(), solver.num_rhs_evals());
    Ok(())
}
