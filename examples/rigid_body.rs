//! Euler Rigid Body — Conservation test. Hairer, Nørsett & Wanner.
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let i1=0.5; let i2=1.0; let i3=2.0;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        f[0]=((1.0/i3)-(1.0/i2))*y[1]*y[2];
        f[1]=((1.0/i1)-(1.0/i3))*y[0]*y[2];
        f[2]=((1.0/i2)-(1.0/i1))*y[0]*y[1];
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[1.0,0.0,0.9]);
    let e0=0.5*(1.0/i1+0.81/i3);
    let l0=(1.0+0.81f64).sqrt();
    let mut solver = Cvode::builder(Method::Adams)
        .rtol(1e-3).atol(1e-5).max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("Euler Rigid Body (I=[{i1},{i2},{i3}])");
    println!("{:>8} {:>12} {:>12} {:>12} {:>14} {:>14}","t","ω₁","ω₂","ω₃","ΔE/E₀","ΔL/L₀");
    println!("{}", "-".repeat(78));
    for i in 1..=20 {
        let (t,y) = solver.solve(0.5*i as f64, Task::Normal)?;
        let e=0.5*(y[0]*y[0]/i1+y[1]*y[1]/i2+y[2]*y[2]/i3);
        let l=(y[0]*y[0]+y[1]*y[1]+y[2]*y[2]).sqrt();
        println!("{t:8.2} {:12.6} {:12.6} {:12.6} {:14.4e} {:14.4e}",y[0],y[1],y[2],(e-e0)/e0,(l-l0)/l0);
    }
    println!("\nSteps: {}, RHS evals: {}", solver.num_steps(), solver.num_rhs_evals());
    Ok(())
}
