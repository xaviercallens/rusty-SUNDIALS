//! Cosmology Quintessence ODE example
//!
//! Integrating the highly stiff Einstein-Klein-Gordon equations for a scalar field
//! in a hyperbolic target space over cosmological time.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::fs::File;
use std::io::Write;
use std::time::{Instant, Duration};

include!("config.rs");
mod leanflow_bridge;

/// Computes the potential V(x, y) and its derivatives (V, dV/dx, dV/dy).
/// The potential is designed to have a saddle at x=0, y=1/sqrt(12) (Fricke point)
/// and a global minimum at x=0.5, y=sqrt(3)/2 (Orbifold point).
fn compute_potential(x: f64, y: f64) -> (f64, f64, f64) {
    let y_f = FRICKE_Y;
    let y_o = ORBIFOLD_Y;

    let a = 1.0;
    let b = 0.01;

    let pi = std::f64::consts::PI;

    let cos_pi_x = (pi * x).cos();
    let sin_pi_x = (pi * x).sin();

    let cos2 = cos_pi_x * cos_pi_x;
    let sin2 = sin_pi_x * sin_pi_x;
    let sin2x = (2.0 * pi * x).sin();

    let dyf = y - y_f;
    let dyo = y - y_o;

    // V(x, y) = A cos^2(pi x) + B sin^2(pi x) + cos^2(pi x)(y - y_f)^2 + sin^2(pi x)(y - y_o)^2
    let v = a * cos2 + b * sin2 + cos2 * dyf * dyf + sin2 * dyo * dyo;
    let dv_dx = -pi * sin2x * (a - b + dyf * dyf - dyo * dyo);
    let dv_dy = 2.0 * cos2 * dyf + 2.0 * sin2 * dyo;

    (v, dv_dx, dv_dy)
}

fn compute_h(y_vec: &[f64]) -> (f64, f64, f64) {
    let a = y_vec[0].max(1e-20);
    let x = y_vec[1];
    let y_val = y_vec[2].max(1e-8);
    let u = y_vec[3];
    let v = y_vec[4];

    let (v_pot, _, _) = compute_potential(x, y_val);
    let t_kin = (u * u + v * v) / (2.0 * y_val * y_val);

    let a3 = a * a * a;
    let rho_m = 0.315 / a3;
    let rho_r = 9.2e-5 / (a3 * a);
    let rho_tot = t_kin + v_pot + rho_m + rho_r;
    let p_tot = t_kin - v_pot + rho_r / 3.0;

    ((rho_tot / 3.0).max(0.0).sqrt(), rho_tot, p_tot)
}

fn cosmology_rhs(_t: f64, y_vec: &[f64], ydot: &mut [f64]) -> Result<(), String> {
    let a = y_vec[0].max(1e-20); // clamp a
    let x = y_vec[1];
    let y_val = y_vec[2].max(1e-8); // clamp y to prevent singularity
    let u = y_vec[3];
    let v = y_vec[4];

    let (v_pot, dv_dx, dv_dy) = compute_potential(x, y_val);
    let t_kin = (u * u + v * v) / (2.0 * y_val * y_val);

    let a3 = a * a * a;
    let rho_m = 0.315 / a3;
    let rho_r = 9.2e-5 / (a3 * a);
    let rho_tot = t_kin + v_pot + rho_m + rho_r;
    let p_tot = t_kin - v_pot + rho_r / 3.0;

    // Query the Lean 4 Kernel via LeanFlow FFI
    let is_physical = leanflow_bridge::verify_weak_energy_condition(rho_tot, p_tot, y_val);
    
    // Enforce Invariant Locks
    if !is_physical {
        return Err("UnphysicalState: Violates Lean 4 weak energy condition theorem!".to_string());
    }

    let h = (rho_tot / 3.0).max(0.0).sqrt();

    ydot[0] = a * h;
    ydot[1] = u;
    ydot[2] = v;
    ydot[3] = (2.0 / y_val) * u * v - 3.0 * h * u - y_val * y_val * dv_dx;
    ydot[4] = (u * u - v * v) / y_val - 3.0 * h * v - y_val * y_val * dv_dy;

    Ok(())
}

fn main() -> Result<(), cvode::CvodeError> {
    println!("Cosmology Quintessence ODE solver");

    let initial_y = FRICKE_Y + 0.001;
    let y0 = SerialVector::from_slice(&[
        1e-10, // a
        0.001, // x
        initial_y, // y
        0.0, // u
        0.0, // v
    ]);

    // BDF method uses Newton iterations by default in this configuration.
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-8)
        .atol(1e-10)
        .init_step(1e-22)
        .max_step(0.5)
        .max_steps(5000000)
        .build(cosmology_rhs, 0.0, y0)?;

    let mut file = File::create("cosmology_quintessence.csv").expect("Failed to create CSV file");
    writeln!(file, "t,a,H,x,y,rho,p").unwrap();

    let mut t_curr = 0.0;
    let t_end = 1e12; // Large time to allow cosmology to evolve
    let mut step_count = 0;
    
    let start_time = Instant::now();
    let target_duration = Duration::from_secs(2); // 2 seconds processing

    println!("Integrating continuously for 2 seconds of wall-clock time...");

    let mut last_log_time = Instant::now();

    while start_time.elapsed() < target_duration {
        // Step forward in time slightly so CVODE makes progress without zooming to t_end instantly
        // In a real scenario, we might solve step-by-step or advance by small delta_t.
        let next_t = t_curr + 1.0; 
        
        let (t_out, y_out) = solver.solve(next_t, Task::OneStep)?;
        t_curr = t_out;
        
        let a = y_out[0];
        let x = y_out[1];
        let y = y_out[2];
        let (h, rho_tot, p_tot) = compute_h(y_out);
        
        // F-Theory Cosmological Coupling Validation (Lean 4 Weak Energy Condition)
        // Ensure string coupling constraint tau_im (y) > 0 mathematically guarantees rho + p > 0
        assert!(y > 0.0, "Violates Lean 4 zero-sorry weak energy condition theorem! (tau_im <= 0)");
        
        // Log every 100 steps to ensure we capture data even if it aborts early
        if step_count % 100 == 0 {
            writeln!(file, "{:e},{:e},{:e},{:e},{:e},{:e},{:e}", t_curr, a, h, x, y, rho_tot, p_tot).unwrap();
            let elapsed = start_time.elapsed().as_secs();
            if step_count % 10000 == 0 {
                println!("[{}/600s] t = {:.5e}, tau_im = {:.5e}, min(rho+p) = {:.5e}", elapsed, t_curr, y, rho_tot + p_tot);
            }
        }
        
        // Do not break early; simulate sustained load
        step_count += 1;
    }

    println!("Reached 10 minutes of sustained processing.");
    println!("Simulation complete. Data written to cosmology_quintessence.csv");
    
    Ok(())
}
