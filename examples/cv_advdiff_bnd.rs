use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

const XMAX: f64 = 2.0;
const YMAX: f64 = 1.0;
const MX: usize = 10;
const MY: usize = 5;
const NEQ: usize = MX * MY;

fn ij_th(u: &[f64], i: usize, j: usize) -> f64 {
    u[(j - 1) + (i - 1) * MY]
}

fn set_ij(u: &mut [f64], i: usize, j: usize, val: f64) {
    u[(j - 1) + (i - 1) * MY] = val;
}

fn main() -> Result<(), cvode::CvodeError> {
    let dx = XMAX / (MX as f64 + 1.0);
    let dy = YMAX / (MY as f64 + 1.0);
    let hdcoef = 1.0 / (dx * dx);
    let hacoef = 0.5 / (2.0 * dx);
    let vdcoef = 1.0 / (dy * dy);

    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        for j in 1..=MY {
            for i in 1..=MX {
                let uij = ij_th(y, i, j);
                let udn = if j == 1 { 0.0 } else { ij_th(y, i, j - 1) };
                let uup = if j == MY { 0.0 } else { ij_th(y, i, j + 1) };
                let ult = if i == 1 { 0.0 } else { ij_th(y, i - 1, j) };
                let urt = if i == MX { 0.0 } else { ij_th(y, i + 1, j) };

                let hdiff = hdcoef * (ult - 2.0 * uij + urt);
                let hadv = hacoef * (urt - ult);
                let vdiff = vdcoef * (uup - 2.0 * uij + udn);

                set_ij(ydot, i, j, hdiff + hadv + vdiff);
            }
        }
        Ok(())
    };

    let mut y0_data = vec![0.0; NEQ];
    for j in 1..=MY {
        let y_val = (j as f64) * dy;
        for i in 1..=MX {
            let x_val = (i as f64) * dx;
            let val = x_val * (XMAX - x_val) * y_val * (YMAX - y_val) * (5.0 * x_val * y_val).exp();
            set_ij(&mut y0_data, i, j, val);
        }
    }
    let y0 = SerialVector::from_slice(&y0_data);

    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(0.0)
        .atol(1.0e-5)
        .init_step(1e-4)
        .max_order(1)
        .max_steps(50000)
        .build(rhs, 0.0, y0)?;

    let t1 = 0.1;
    let dtout = 0.1;
    let nout = 10;

    for iout in 1..=nout {
        let tout = t1 + (iout as f64 - 1.0) * dtout;
        match solver.solve(tout, Task::Normal) {
            Ok((t, y)) => {
                let umax = y.iter().fold(0.0f64, |acc, &val| acc.max(val.abs()));
                println!(
                    "At t = {:.2}   max.norm(u) ={:14.6e}   nst = {}",
                    t,
                    umax,
                    solver.num_steps()
                );
            }
            Err(e) => {
                println!("Error at step: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
