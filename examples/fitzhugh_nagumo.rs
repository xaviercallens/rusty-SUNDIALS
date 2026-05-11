//! FitzHugh-Nagumo — FitzHugh (1961), Nagumo et al. (1962).
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let a=0.7; let b=0.8; let tau=12.5; let i_ext=0.5;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        f[0]=y[0]-y[0].powi(3)/3.0-y[1]+i_ext;
        f[1]=(y[0]+a-b*y[1])/tau;
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[-1.0, 1.0]);
    let mut solver = Cvode::builder(Method::Adams)
        .rtol(1e-3).atol(1e-5).max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("FitzHugh-Nagumo (I_ext={i_ext})");
    println!("{:>8} {:>14} {:>14}","t","v","w");
    println!("{}", "-".repeat(38));
    for i in 1..=20 {
        let (t,y) = solver.solve(5.0*i as f64, Task::Normal)?;
        println!("{t:8.1} {:14.8} {:14.8}",y[0],y[1]);
    }
    println!("\nSteps: {}, RHS evals: {}", solver.num_steps(), solver.num_rhs_evals());
    Ok(())
}
