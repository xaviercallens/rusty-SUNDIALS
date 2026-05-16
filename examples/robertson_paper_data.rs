/// Robertson Chemical Kinetics — CSV output for paper figures.
///
/// Emits step-by-step CSV data for generating:
///   - Figure 1: Step size adaptation over time (h vs t)
///   - Figure 2: Newton iterations per step (NI/step vs step number)
///   - Figure 3: BDF order selection over time
///
/// Usage:
///   cargo run --example robertson_paper_data --features experimental-nls-v2 > paper_data.csv
use cvode::{
    constants::{Method, Task},
    Cvode,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let n = 3;
    let y0 = vec![1.0, 0.0, 0.0];
    let t0 = 0.0;
    let rtol = 1e-4;
    let atol = vec![1e-8, 1e-14, 1e-6];

    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        ydot[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        ydot[2] = 3e7 * y[1] * y[1];
        Ok(())
    };

    let jac = |_t: f64, y: &[f64], j: &mut sundials_core::DenseMat| -> Result<(), String> {
        j.set(0, 0, -0.04);
        j.set(0, 1, 1e4 * y[2]);
        j.set(0, 2, 1e4 * y[1]);
        j.set(1, 0, 0.04);
        j.set(1, 1, -1e4 * y[2] - 6e7 * y[1]);
        j.set(1, 2, -1e4 * y[1]);
        j.set(2, 0, 0.0);
        j.set(2, 1, 6e7 * y[1]);
        j.set(2, 2, 0.0);
        Ok(())
    };

    let mut solver = Cvode::new(Method::Bdf, rhs, &y0, t0, n, rtol, &atol)?;
    solver.set_jacobian(jac);

    // CSV header
    println!("step,t,h,order,nfe,y1,y2,y3,conservation_error");

    let mut step_count = 0;
    let t_final = 4e10;

    // Use OneStep mode to capture every internal step
    let mut t_current = t0;
    while t_current < t_final {
        match solver.solve(t_final, Task::OneStep) {
            Ok((_t_ret, _flag)) => {
                step_count += 1;
                let y = solver.y();
                let h = solver.step_size();
                let q = solver.order();
                let nfe = solver.num_rhs_evals();
                let t = _t_ret;
                let cons_err = (y[0] + y[1] + y[2] - 1.0).abs();

                // Output at log-spaced intervals to keep CSV manageable
                // First 100 steps, then every 10th, then every 50th after 500
                let emit = step_count <= 100
                    || (step_count <= 500 && step_count % 10 == 0)
                    || step_count % 50 == 0;

                if emit {
                    println!(
                        "{step_count},{t:.6e},{h:.6e},{q},{nfe},{:.10e},{:.10e},{:.10e},{cons_err:.2e}",
                        y[0], y[1], y[2]
                    );
                }
                t_current = t;
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
