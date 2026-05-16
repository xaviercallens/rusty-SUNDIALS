//! Robertson chemical kinetics — classic stiff ODE test problem.
//!
//! dy1/dt = -0.04*y1 + 1e4*y2*y3
//! dy2/dt =  0.04*y1 - 1e4*y2*y3 - 3e7*y2^2
//! dy3/dt =  3e7*y2^2
//!
//! y(0) = [1, 0, 0], t ∈ [0, 4e10]
//!
//! This is the standard SUNDIALS CVODE example (cvRoberts_dns).
//!
//! v11: Added `--output-csv` flag for machine-readable benchmark output.
//!
//! Usage:
//!   cargo run --example robertson_csv -- --output-csv results.csv
//!   cargo run --example robertson_csv -- --output-csv -  # stdout CSV

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};

/// Parsed CLI options for the Robertson runner.
struct Options {
    /// Path to CSV output file, or "-" for stdout. `None` = table mode.
    output_csv: Option<String>,
    /// Relative tolerance.
    rtol: f64,
    /// Absolute tolerance.
    atol: f64,
}

impl Options {
    fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut output_csv = None;
        let mut rtol = 1e-4_f64;
        let mut atol = 1e-8_f64;
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--output-csv" => {
                    i += 1;
                    output_csv = args.get(i).cloned();
                }
                "--rtol" => {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        rtol = v.parse().unwrap_or(1e-4);
                    }
                }
                "--atol" => {
                    i += 1;
                    if let Some(v) = args.get(i) {
                        atol = v.parse().unwrap_or(1e-8);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Options {
            output_csv,
            rtol,
            atol,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Options::parse();

    let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        ydot[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        ydot[2] = 3e7 * y[1] * y[1];
        Ok(())
    };

    let y0 = SerialVector::from_slice(&[1.0, 0.0, 0.0]);
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(opts.rtol)
        .atol(opts.atol)
        .max_steps(50000)
        .build(rhs, 0.0, y0)?;

    let times = [
        0.4, 4.0, 40.0, 400.0, 4000.0, 40000.0, 4e5, 4e6, 4e7, 4e8, 4e9, 4e10,
    ];

    // Decide output writer
    let use_csv = opts.output_csv.is_some();

    let stdout = io::stdout();
    let mut writer: Box<dyn Write> = match &opts.output_csv {
        None => Box::new(BufWriter::new(stdout.lock())),
        Some(path) if path == "-" => Box::new(BufWriter::new(stdout.lock())),
        Some(path) => Box::new(BufWriter::new(
            File::create(path).expect("Cannot create CSV output file"),
        )),
    };

    // Header
    if use_csv {
        writeln!(
            writer,
            "t,y1,y2,y3,steps,rhs_evals,order,conservation_error"
        )?;
    } else {
        writeln!(writer, "Robertson Chemical Kinetics (stiff system)")?;
        writeln!(writer, "BDF method with adaptive step size")?;
        writeln!(writer, "{:>12} {:>14} {:>14} {:>14}", "t", "y1", "y2", "y3")?;
        writeln!(writer, "{}", "-".repeat(58))?;
    }

    for &tout in &times {
        let (t, y) = solver.solve(tout, Task::Normal)?;
        let y0_val = y[0];
        let y1_val = y[1];
        let y2_val = y[2];
        let conservation = (y0_val + y1_val + y2_val - 1.0).abs();
        // Drop the borrow on y before accessing solver stats
        drop(y);
        if use_csv {
            let steps = solver.num_steps();
            let rhs = solver.num_rhs_evals();
            let order = solver.order();
            writeln!(
                writer,
                "{t:.6e},{y0_val:.10e},{y1_val:.10e},{y2_val:.10e},{steps},{rhs},{order},{conservation:.4e}",
            )?;
        } else {
            writeln!(
                writer,
                "{t:12.4e} {y0_val:14.6e} {y1_val:14.6e} {y2_val:14.6e}",
            )?;
        }
    }

    if !use_csv {
        let y = solver.y();
        let sum = y[0] + y[1] + y[2];
        writeln!(writer, "\nSolver statistics:")?;
        writeln!(writer, "  Steps: {}", solver.num_steps())?;
        writeln!(writer, "  RHS evals: {}", solver.num_rhs_evals())?;
        writeln!(writer, "  Final order: {}", solver.order())?;
        writeln!(
            writer,
            "  Conservation (y1+y2+y3): {sum:.15e} (should be 1.0)"
        )?;
    }

    Ok(())
}
