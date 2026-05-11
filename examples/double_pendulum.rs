//! Double Pendulum — Chaotic mechanics. Shinbrot et al. (1992).
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let g = 9.81;
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        let (th1,w1,th2,w2) = (y[0],y[1],y[2],y[3]);
        let d = th1-th2; let cd=d.cos(); let sd=d.sin();
        let den = 2.0-cd*cd;
        f[0]=w1;
        f[1]=(-w1*w1*sd*cd+g*th2.sin()*cd-w2*w2*sd-2.0*g*th1.sin())/den;
        f[2]=w2;
        f[3]=(w2*w2*sd*cd+2.0*(g*th1.sin()*cd+w1*w1*sd-g*th2.sin()))/den;
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[1.5,0.0,1.0,0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-3).atol(1e-5).max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("Double Pendulum (chaotic)");
    println!("{:>8} {:>12} {:>12} {:>12} {:>12}","t","θ₁","ω₁","θ₂","ω₂");
    println!("{}", "-".repeat(60));
    for i in 1..=20 {
        let (t,y) = solver.solve(0.25*i as f64, Task::Normal)?;
        println!("{t:8.2} {:12.6} {:12.6} {:12.6} {:12.6}",y[0],y[1],y[2],y[3]);
    }
    println!("\nSteps: {}, RHS evals: {}", solver.num_steps(), solver.num_rhs_evals());
    Ok(())
}
