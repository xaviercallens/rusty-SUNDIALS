//! Three-Body Gravitational Problem — Hairer et al.
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
fn main() -> Result<(), cvode::CvodeError> {
    let rhs = move |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        let (x1,y1,x2,y2,x3,y3) = (y[0],y[1],y[2],y[3],y[4],y[5]);
        let r12=((x1-x2).powi(2)+(y1-y2).powi(2)).sqrt().max(1e-10);
        let r13=((x1-x3).powi(2)+(y1-y3).powi(2)).sqrt().max(1e-10);
        let r23=((x2-x3).powi(2)+(y2-y3).powi(2)).sqrt().max(1e-10);
        let (r12_3,r13_3,r23_3) = (r12.powi(3),r13.powi(3),r23.powi(3));
        f[0]=y[6]; f[1]=y[7]; f[2]=y[8]; f[3]=y[9]; f[4]=y[10]; f[5]=y[11];
        f[6] =-(x1-x2)/r12_3-(x1-x3)/r13_3;
        f[7] =-(y1-y2)/r12_3-(y1-y3)/r13_3;
        f[8] =-(x2-x1)/r12_3-(x2-x3)/r23_3;
        f[9] =-(y2-y1)/r12_3-(y2-y3)/r23_3;
        f[10]=-(x3-x1)/r13_3-(x3-x2)/r23_3;
        f[11]=-(y3-y1)/r13_3-(y3-y2)/r23_3;
        Ok(())
    };
    let y0 = SerialVector::from_slice(&[
        -0.5,0.0, 0.5,0.0, 0.0,0.866,
        0.0,0.5, 0.0,-0.5, -0.5,0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-3).atol(1e-5).max_steps(50_000_000)
        .build(rhs, 0.0, y0)?;
    println!("Three-Body Problem (equal masses)");
    println!("{:>6} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}","t","x1","y1","x2","y2","x3","y3");
    println!("{}", "-".repeat(68));
    for i in 1..=20 {
        let (t,y) = solver.solve(0.25*i as f64, Task::Normal)?;
        println!("{t:6.2} {:10.4} {:10.4} {:10.4} {:10.4} {:10.4} {:10.4}",
            y[0],y[1],y[2],y[3],y[4],y[5]);
    }
    println!("\nSteps: {}, RHS evals: {}", solver.num_steps(), solver.num_rhs_evals());
    Ok(())
}
