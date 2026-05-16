/// Robertson Chemical Kinetics — CSV output for paper figures.
///
/// Emits step-by-step CSV data for generating:
///   - Figure 1: Step size adaptation over time (h vs t)
///   - Figure 2: BDF order selection over time
///
/// Usage:
///   cargo run --example robertson_paper_data --features experimental-nls-v2 > paper/data_exp.csv
use cvode::{Cvode, Method, Task};
use nvector::SerialVector;

fn main() -> Result<(), cvode::CvodeError> {
    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        ydot[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        ydot[2] = 3e7 * y[1] * y[1];
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0, 0.0, 0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-4)
        .atol(1e-8)
        .max_order(5)
        .init_step(1e-4)
        .max_steps(50000)
        .jacobian(|_t, y, j| {
            j.cols[0][0] = -0.04;
            j.cols[0][1] = 0.04;
            j.cols[0][2] = 0.0;
            j.cols[1][0] = 1e4 * y[2];
            j.cols[1][1] = -1e4 * y[2] - 6e7 * y[1];
            j.cols[1][2] = 6e7 * y[1];
            j.cols[2][0] = 1e4 * y[1];
            j.cols[2][1] = -1e4 * y[1];
            j.cols[2][2] = 0.0;
            Ok(())
        })
        .build(rhs, 0.0, y0)?;

    // CSV header
    println!("step,t,h,order,nfe,y1,y2,y3,conservation_error");

    let mut step_count: usize = 0;
    let t_final = 4e10;
    let mut t_current = 0.0_f64;

    while t_current < t_final {
        match solver.solve(t_final, Task::OneStep) {
            Ok((t_ret, _flag)) => {
                step_count += 1;
                let y = solver.y();
                let h = solver.step_size();
                let q = solver.order();
                let nfe = solver.num_rhs_evals();
                let cons_err = (y[0] + y[1] + y[2] - 1.0).abs();

                // Output at log-spaced intervals to keep CSV manageable
                let emit = step_count <= 100
                    || (step_count <= 500 && step_count % 10 == 0)
                    || step_count % 50 == 0;

                if emit {
                    println!(
                        "{step_count},{t_ret:.6e},{h:.6e},{q},{nfe},{:.10e},{:.10e},{:.10e},{cons_err:.2e}",
                        y[0], y[1], y[2]
                    );
                }
                t_current = t_ret;
            }
            Err(e) => {
                eprintln!("Solver error at step {step_count}: {e}");
                break;
            }
        }
    }

    // Final summary line
    let y = solver.y();
    let steps = solver.num_steps();
    let rhs_evals = solver.num_rhs_evals();
    #[cfg(feature = "experimental-nls-v2")]
    let nni = solver.num_newton_iters();
    #[cfg(not(feature = "experimental-nls-v2"))]
    let nni = 0usize;
    let cons = (y[0] + y[1] + y[2] - 1.0).abs();

    eprintln!("# Summary: steps={steps} rhs={rhs_evals} nni={nni} ni_per_step={:.2} conservation={cons:.2e}",
        nni as f64 / steps as f64);

    Ok(())
}
