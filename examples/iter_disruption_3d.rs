//! "3D toroidal ITER disruption" — CLOSED-FORM EVALUATION, NOT A SIMULATION.
//!
//! AUDIT NOTE 2026-09-27 (docs/audit/fusion-2026-09-27/README.md; reports A §0.2,
//! B §2, C §3):
//!   * No ODE is solved. The state at each output time is the prescribed closed
//!     form (Te ∝ e^{-3t}(1 + island(t)) + edge term, j ∝ (1-0.6t)(1+0.4t·redist),
//!     vessel ∝ 4t e^{-2t}) evaluated directly; the same prescribed trajectory
//!     the 2-D example feeds to CVODE as a y-independent RHS. It is not a
//!     physics model: no MHD, no circuit equations, no toroidal coupling
//!     beyond the cos(2θ-φ) factor in the prescribed island shape.
//!   * The earlier version imported cvode but never constructed a solver, and
//!     printed "Injecting Neural-FGMRES", "Loading FNO/DeepONet/MPNN weights",
//!     "Offloading SpMV to H100 Tensor Cores (Target: 157x speedup)" and a
//!     "Newton residual proxy -> FP8/FP16/FP32" schedule computed from
//!     exp(-5t). The environment knobs RUSTY_SUNDIALS_GPU_ABLATION,
//!     _ADAPTIVE_PRECISION and _ARCHITECTURE had no effect on the output
//!     (report B: all 119 CSV md5 sums identical with the flags flipped), and
//!     the closing banner hardcoded "GPU Ablation=ON, AdaptivePrec=ON". Those
//!     prints were removed; the knobs are still read and echoed, with a note
//!     that they do nothing.
//!   * "DOF" below counts output values, not unknowns of a solved system.
//!
//! For a state-dependent model (a 0-D current-quench circuit whose RHS depends
//! on y) see examples/iter_current_quench_0d.rs.

use std::fs::File;
use std::io::Write;
use std::time::Instant;
use sundials_core::Real;

// ═══════════════════════════════════════════════════════════════
// 3D Toroidal Grid Parameters
// ═══════════════════════════════════════════════════════════════
const N_RHO: usize = 100; // radial
const N_THETA: usize = 200; // poloidal
const N_PHI: usize = 16; // toroidal slices
const N_PLASMA_2D: usize = N_RHO * N_THETA;
const N_PLASMA_3D: usize = N_RHO * N_THETA * N_PHI;

const N_R_VESSEL: usize = 10;
const N_THETA_VESSEL: usize = 200;
const N_PHI_VESSEL: usize = 16;
const N_VESSEL: usize = N_R_VESSEL * N_THETA_VESSEL * N_PHI_VESSEL;

const TE0: f64 = 25000.0;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  rusty-SUNDIALS: 3D ITER disruption CLOSED-FORM evaluation  ║");
    println!("║  (no ODE solve, no physics model; see file header)          ║");
    println!(
        "║  Grid: {}×{}×{} = {} plasma DOF               ║",
        N_RHO, N_THETA, N_PHI, N_PLASMA_3D
    );
    println!(
        "║  Total system DOF: {}                              ║",
        N_PLASMA_3D * 2 + N_VESSEL
    );
    println!("╚══════════════════════════════════════════════════════════════╝");

    let start_setup = Instant::now();

    // Precompute 3D spatial profiles for Plasma (ρ, θ, φ)
    let mut rho_p = vec![0.0; N_PLASMA_3D];
    let mut theta_p = vec![0.0; N_PLASMA_3D];
    let mut phi_p = vec![0.0; N_PLASMA_3D];
    let mut te_base = vec![0.0; N_PLASMA_3D];
    let mut island_shape = vec![0.0; N_PLASMA_3D];
    let mut edge_shape = vec![0.0; N_PLASMA_3D];
    let mut j_base = vec![0.0; N_PLASMA_3D];
    let mut j_redist_shape = vec![0.0; N_PLASMA_3D];

    for ip in 0..N_PHI {
        let ph = (ip as f64) * 2.0 * std::f64::consts::PI / (N_PHI as f64);
        for ir in 0..N_RHO {
            let r = 0.01 + (ir as f64) / (N_RHO as f64 - 1.0) * 0.99;
            for it in 0..N_THETA {
                let th = (it as f64) * 2.0 * std::f64::consts::PI / (N_THETA as f64);
                let idx = ip * N_PLASMA_2D + ir * N_THETA + it;

                rho_p[idx] = r;
                theta_p[idx] = th;
                phi_p[idx] = ph;
                te_base[idx] = TE0 * (1.0 - r * r).powi(2);

                let rs = 0.45;
                // n=1, m=2 helical tearing mode: cos(m*θ - n*φ)
                island_shape[idx] =
                    (-(r - rs).powi(2) / 0.08_f64.powi(2)).exp() * (2.0 * th - ph).cos();
                edge_shape[idx] = (-(r - 0.85).powi(2) / 0.1_f64.powi(2)).exp();

                j_base[idx] = 1.2e6 * (1.0 - r * r).powf(1.5);
                j_redist_shape[idx] = (-(r - 0.7).powi(2) / 0.15_f64.powi(2)).exp();
            }
        }
    }

    // Precompute 3D spatial profiles for Vessel
    let mut poloidal_var = vec![0.0; N_VESSEL];
    let mut skin_factor = vec![0.0; N_VESSEL];

    for ip in 0..N_PHI_VESSEL {
        for ir in 0..N_R_VESSEL {
            let r = (ir as f64) / ((N_R_VESSEL - 1) as f64);
            for it in 0..N_THETA_VESSEL {
                let th = (it as f64) * 2.0 * std::f64::consts::PI / ((N_THETA_VESSEL - 1) as f64);
                let idx = ip * (N_R_VESSEL * N_THETA_VESSEL) + ir * N_THETA_VESSEL + it;

                poloidal_var[idx] = 1.0 + 0.4 * th.cos() - 0.2 * (2.0 * th).cos();
                skin_factor[idx] = (-r / 0.3).exp();
            }
        }
    }

    // State vector layout (3D):
    // [0 .. N_PLASMA_3D]                       : Te (electron temperature)
    // [N_PLASMA_3D .. 2*N_PLASMA_3D]           : j_phi (toroidal current density)
    // [2*N_PLASMA_3D .. 2*N_PLASMA_3D+N_VESSEL]: j_induced (vessel eddy currents)
    let neq = 2 * N_PLASMA_3D + N_VESSEL;
    println!("  [Setup] Total DOF: {} ({:.1}M)", neq, neq as f64 / 1e6);

    // Initial Conditions
    let mut y0_vec = vec![0.0; neq];
    for i in 0..N_PLASMA_3D {
        let island_width = 0.05;
        let island = island_width * island_shape[i];
        y0_vec[i] = te_base[i] * (1.0 + island);
        y0_vec[N_PLASMA_3D + i] = j_base[i];
    }
    for i in 0..N_VESSEL {
        y0_vec[2 * N_PLASMA_3D + i] = 1.4e-8;
    }

    println!("  [Solver] None. States are evaluated from the prescribed closed form.");

    let gpu_ablation =
        std::env::var("RUSTY_SUNDIALS_GPU_ABLATION").unwrap_or_else(|_| "1".to_string()) == "1";
    let adaptive_precision = std::env::var("RUSTY_SUNDIALS_ADAPTIVE_PRECISION")
        .unwrap_or_else(|_| "1".to_string())
        == "1";
    let architecture =
        std::env::var("RUSTY_SUNDIALS_ARCHITECTURE").unwrap_or_else(|_| "MPNN".to_string());

    println!("  [Setup] Grid initialization: {:?}", start_setup.elapsed());

    // AUDIT: these knobs never changed the computation. They are echoed so that
    // scripts which set them still see them, but no weights are loaded, no GPU
    // is used and no precision schedule is applied.
    println!(
        "  [Env knobs] ARCHITECTURE={}, GPU_ABLATION={}, ADAPTIVE_PRECISION={} (no effect: no neural net, GPU or precision control exists here)",
        architecture, gpu_ablation, adaptive_precision
    );

    let start = Instant::now();

    let out_times: Vec<Real> = vec![0.0, 0.3, 0.4, 0.5, 0.7, 0.9, 1.0];
    std::fs::create_dir_all("data/fusion/rust_sim_output_3d").unwrap();

    for &t_out in &out_times {
        let y_curr = if t_out == 0.0 {
            y0_vec.clone()
        } else {
            // Closed-form evaluation (no solver; see file header).
            let mut y_slice = vec![0.0; neq];
            for i in 0..N_PLASMA_3D {
                let island_width = 0.05 + 0.35 * t_out;
                let quench_factor = (-3.0 * t_out).exp();
                let island = island_width * island_shape[i];
                let te = te_base[i] * quench_factor * (1.0 + island)
                    + TE0 * 0.15 * t_out * edge_shape[i];
                y_slice[i] = te;

                let j_phi =
                    j_base[i] * (1.0 - 0.6 * t_out) * (1.0 + 0.4 * t_out * j_redist_shape[i]);
                y_slice[N_PLASMA_3D + i] = j_phi;
            }
            let current_quench = 4.0 * t_out * (-2.0 * t_out).exp();
            for i in 0..N_VESSEL {
                y_slice[2 * N_PLASMA_3D + i] =
                    3.3e5 * current_quench * poloidal_var[i] * skin_factor[i];
            }
            y_slice
        };

        // Save per-toroidal-slice CSV files
        for ip in 0..N_PHI {
            let filename = format!(
                "data/fusion/rust_sim_output_3d/iter_3d_t{:.2}_phi{:02}.csv",
                t_out, ip
            );
            let mut file = File::create(&filename).unwrap();
            writeln!(file, "domain,ir,itheta,iphi,Te,j_phi").unwrap();

            for ir in 0..N_RHO {
                for it in 0..N_THETA {
                    let idx = ip * N_PLASMA_2D + ir * N_THETA + it;
                    let mut te = y_curr[idx];
                    if te < 2.0 {
                        te = 2.0;
                    }
                    let j_phi = y_curr[N_PLASMA_3D + idx];
                    writeln!(file, "plasma,{},{},{},{},{}", ir, it, ip, te, j_phi).unwrap();
                }
            }
        }

        // Also save a consolidated file per time-step
        let filename = format!("data/fusion/rust_sim_output_3d/iter_3d_t{:.2}.csv", t_out);
        let mut file = File::create(&filename).unwrap();
        writeln!(file, "domain,ir,itheta,iphi,Te,j_phi").unwrap();

        for ip in 0..N_PHI {
            for ir in 0..N_RHO {
                for it in 0..N_THETA {
                    let idx = ip * N_PLASMA_2D + ir * N_THETA + it;
                    let mut te = y_curr[idx];
                    if te < 2.0 {
                        te = 2.0;
                    }
                    let j_phi = y_curr[N_PLASMA_3D + idx];
                    writeln!(file, "plasma,{},{},{},{},{}", ir, it, ip, te, j_phi).unwrap();
                }
            }
        }

        println!(
            "  [t={:.2}] Saved 3D state ({} DOF) in {:?}",
            t_out,
            neq,
            start.elapsed()
        );
    }

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!(
        "║  3D closed-form evaluation + CSV write complete in {:?}  ║",
        start.elapsed()
    );
    println!(
        "║  Total DOF: {} ({:.2}M)                          ║",
        neq,
        neq as f64 / 1e6
    );
    println!(
        "║  Toroidal slices: {} | Mode: m=2, n=1               ║",
        N_PHI
    );
    println!(
        "║  Env knobs (no effect): GPU_ABLATION={}, ADAPTIVE_PRECISION={}, ARCH={} ║",
        gpu_ablation, adaptive_precision, architecture
    );
    println!("╚══════════════════════════════════════════════════════════════╝");
}
