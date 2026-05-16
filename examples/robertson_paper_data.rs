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
        let (t_ret, _flag) = solver.solve(t_final, Task::OneStep)?;
        step_count += 1;

        // Collect all data after solve() returns (no mutable borrow conflict)
        let h = solver.step_size();
        let q = solver.order();
        let nfe = solver.num_rhs_evals();
        let y0_val = solver.y()[0];
        let y1_val = solver.y()[1];
        let y2_val = solver.y()[2];
        let cons_err = (y0_val + y1_val + y2_val - 1.0).abs();

        // Output at log-spaced intervals to keep CSV manageable
        let emit = step_count <= 100
            || (step_count <= 500 && step_count % 10 == 0)
            || step_count % 50 == 0;

        if emit {
            println!(
                "{step_count},{t_ret:.6e},{h:.6e},{q},{nfe},{y0_val:.10e},{y1_val:.10e},{y2_val:.10e},{cons_err:.2e}"
            );
        }
        t_current = t_ret;
    }

    // Final summary line
    let steps = solver.num_steps();
    let rhs_evals = solver.num_rhs_evals();
    let y_final = solver.y();
    let cons = (y_final[0] + y_final[1] + y_final[2] - 1.0).abs();
    #[cfg(feature = "experimental-nls-v2")]
    let nni = solver.num_newton_iters();
    #[cfg(not(feature = "experimental-nls-v2"))]
    let nni = 0usize;

    let ni_per_step = nni as f64 / steps as f64;
    eprintln!(
        "# Summary: steps={steps} rhs={rhs_evals} nni={nni} \
         ni_per_step={ni_per_step:.2} conservation={cons:.2e}"
    );

    Ok(())
}
