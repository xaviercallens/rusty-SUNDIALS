//! `sundials-mcp` — a Model Context Protocol (stdio, JSON-RPC 2.0) server that lets AI agents and
//! scientists run rusty-SUNDIALS solvers: named CVODE problems with independent known answers, and
//! the `qf-pgpe` projected Gross–Pitaevskii solver.
//!
//! # Honesty conventions
//!
//! * Every tool result carries `ran`. `ran: false` means the computation did not execute (bad
//!   arguments, timeout, worker crash) and is **never** a pass.
//! * A solver failure is reported as an MCP tool error (`isError: true`) with the solver's own
//!   message and **no partial trajectory** — a half-finished run is not a result.
//! * Known-answer checks compare against a value fixed independently of this code (a closed-form
//!   solution, a conserved quantity, or the LLNL SUNDIALS reference output), and the reference is
//!   named in the result, so a reader can tell a check from a restatement.
//!
//! # Process isolation (why the solver never shares the protocol stream)
//!
//! `cvode/src/solver.rs` prints diagnostics to **stdout** on some failure paths
//! (`println!("ERROR FAIL …")`). On a stdio protocol that corrupts the stream. Rather than
//! redirecting file descriptors (which needs `unsafe`, and this repository has none), the protocol
//! process never calls solver code: each tool call runs in a worker subprocess
//! (`sundials-mcp --worker`) whose stdout is a duplicate of the parent's *stderr* handle and whose
//! result comes back through a temporary file. The parent can therefore also enforce a wall-clock
//! timeout by killing the worker. All of this uses safe `std` APIs only.
#![forbid(unsafe_code)]

use cvode::{Cvode, CvodeError, Method, Task};
use num_complex::Complex64;
use nvector::SerialVector;
use qf_cmb_cascade::{r_bound_2sigma, sigma_cv, sigma_ell};
use qf_pgpe::ComplexField2D;
use serde_json::{Value, json};

pub const SERVER_NAME: &str = "sundials-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol versions this server's handshake implements. A client asking for one of these gets
/// it back; any other request is answered with the latest one here (the client decides whether
/// to continue, per the MCP version-negotiation rules).
pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 4] =
    ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];

/// Outcome of a tool call. `Err` is an honest failure with the reason; `ran` tells whether the
/// computation actually executed before failing.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    Ok(Value),
    Err { ran: bool, message: String },
}

impl ToolOutcome {
    pub fn to_json(&self) -> Value {
        match self {
            ToolOutcome::Ok(v) => json!({"status": "ok", "value": v}),
            ToolOutcome::Err { ran, message } => {
                json!({"status": "err", "ran": ran, "message": message})
            }
        }
    }

    pub fn from_json(v: &Value) -> ToolOutcome {
        match v.get("status").and_then(Value::as_str) {
            Some("ok") => ToolOutcome::Ok(v.get("value").cloned().unwrap_or(Value::Null)),
            Some("err") => ToolOutcome::Err {
                ran: v.get("ran").and_then(Value::as_bool).unwrap_or(false),
                message: v
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            },
            _ => ToolOutcome::Err {
                ran: false,
                message: format!("malformed worker result: {v}"),
            },
        }
    }
}

fn bad_args(msg: impl Into<String>) -> ToolOutcome {
    ToolOutcome::Err {
        ran: false,
        message: msg.into(),
    }
}

// ------------------------------------------------------------------------------------------------
// Tool catalogue
// ------------------------------------------------------------------------------------------------

/// The `tools/list` payload.
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "about",
            "description": "Server scope, honesty conventions (ran / isError / known-answer references), \
                            and what is deliberately not exposed yet.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "list_problems",
            "description": "Named CVODE problems this server can solve, with their parameters, bounds, \
                            and the independent reference each result is checked against.",
            "inputSchema": {"type": "object", "properties": {}}
        },
        {
            "name": "solve",
            "description": "Solve a named CVODE problem (exponential, robertson, vanderpol, lorenz, domain_exit) and \
                            return the trajectory at the requested output times plus solver statistics \
                            and known-answer checks. A solver failure is returned as an error with no \
                            partial trajectory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "problem": {"type": "string", "enum": ["exponential", "robertson", "vanderpol", "lorenz", "domain_exit"]},
                    "t_out": {"type": "array", "items": {"type": "number"},
                              "description": "Increasing positive output times (at most 50). Defaults per problem."},
                    "rtol": {"type": "number", "description": "Relative tolerance, 1e-12..1e-2."},
                    "atol": {"type": "number", "description": "Absolute tolerance, 1e-16..1e-2."},
                    "max_steps": {"type": "integer", "description": "Step budget, 1..1000000."},
                    "mu": {"type": "number", "description": "Van der Pol stiffness parameter (vanderpol only), 0..1000."}
                },
                "required": ["problem"]
            }
        },
        {
            "name": "pgpe_run",
            "description": "Run the qf-pgpe projected Gross-Pitaevskii solver (2D periodic, IF-RK4) on a \
                            small grid and report norm/energy/momentum before and after, their drifts, and \
                            for a plane-wave initial state the max error against the exact solution.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "initial": {"type": "string", "enum": ["plane_wave", "seeded"]},
                    "n": {"type": "integer", "enum": [16, 32, 64]},
                    "l": {"type": "number", "description": "Box side, 1..1000 (default 32)."},
                    "g": {"type": "number", "description": "Nonlinearity, -100..100 (default 1)."},
                    "dt": {"type": "number", "description": "Time step, 1e-5..0.1 (default 0.005)."},
                    "t_end": {"type": "number", "description": "End time; at most 4000 steps (default 5)."},
                    "mode": {"type": "integer", "description": "Plane-wave mode number m, k = 2 pi m / l (default 3)."}
                },
                "required": ["initial"]
            }
        },
        {
            "name": "cmb_bound",
            "description": "Compute the Koren-Tsai-Wang 2-sigma CMB bound on a late-time first-order \
                            dark-energy phase transition, using the exact bubble-completion-time power \
                            spectrum from Elor et al. (arXiv:2311.16222) checked against the real \
                            Planck 2018 TT data. Returns r_2sigma (the bubble nucleation rate per Hubble \
                            volume per Hubble time at which the model is excluded at 2 sigma) and the \
                            dominant multipole ell_peak. Each call takes 10-30 seconds at typical \
                            parameters; a 120-second worker timeout applies.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "beta_over_h": {
                        "type": "number",
                        "description": "Bubble nucleation rate normalized to Hubble, beta/H* (1..1000, default 100)."
                    },
                    "zpt": {
                        "type": "number",
                        "description": "Phase-transition redshift (0.01..0.9, default 0.1)."
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["planck", "cosmic_variance"],
                        "description": "Noise model: 'planck' (Planck 2018 TT bars, default) or \
                                       'cosmic_variance' (cosmic-variance floor — the best any \
                                       temperature-only CMB experiment could do)."
                    }
                },
                "required": []
            }
        }
    ])
}

/// Run a tool in-process. The protocol process never calls this for solver tools; the worker does.
pub fn run_tool(name: &str, args: &Value) -> ToolOutcome {
    match name {
        "about" => ToolOutcome::Ok(about()),
        "list_problems" => ToolOutcome::Ok(list_problems()),
        "solve" => solve(args),
        "pgpe_run" => pgpe_run(args),
        "cmb_bound" => cmb_bound(args),
        other => bad_args(format!("unknown tool {other:?}")),
    }
}

/// Tools cheap and solver-free enough to answer in the protocol process itself.
pub fn is_inline_tool(name: &str) -> bool {
    matches!(name, "about" | "list_problems")
}

fn about() -> Value {
    json!({
        "server": SERVER_NAME,
        "version": SERVER_VERSION,
        "conventions": {
            "ran": "false means the computation did not execute; never read it as a pass",
            "errors": "a solver failure is an MCP tool error (isError: true) with no partial trajectory",
            "known_answers": "checks compare with an independent reference named in each result",
            "isolation": "solvers run in a worker subprocess whose stdout is routed to stderr, so solver \
                          diagnostics cannot corrupt the protocol stream; the parent enforces a timeout"
        },
        "exposed": {
            "qf-cmb-cascade": "cmb_bound tool is live (PR #57 merged). Computes the Koren-Tsai-Wang \
                               2-sigma CMB bound on late-time dark-energy phase transitions, checked \
                               against Planck 2018 TT data. Each call typically takes 10-30 seconds."
        }
    })
}

fn list_problems() -> Value {
    json!([
        {"name": "exponential", "equation": "y' = -y, y(0) = 1", "dim": 1, "method": "BDF",
         "default_t_out": [0.1, 0.5, 1.0, 2.0, 5.0, 10.0], "max_t": 100.0,
         "reference": "closed form y(t) = exp(-t); reported as max_abs_error"},
        {"name": "robertson", "equation": "Robertson stiff chemical kinetics (3 species)", "dim": 3,
         "method": "BDF order 5 with analytical Jacobian, init_step 1e-4 (LLNL cvRoberts_dns configuration)",
         "default_t_out": [0.4, 4.0, 40.0, 400.0, 4000.0, 40000.0], "max_t": 4.0e10,
         "reference": "mass conservation y1+y2+y3 = 1, and the LLNL SUNDIALS cvRoberts_dns published output at t = 0.4 (y1 = 9.8517e-01, y2 = 3.3864e-05, y3 = 1.4794e-02)"},
        {"name": "vanderpol", "equation": "y0' = y1, y1' = mu (1 - y0^2) y1 - y0", "dim": 2, "method": "BDF",
         "default_t_out": [5.0, 10.0, 15.0, 20.0, 25.0], "max_t": 3000.0, "params": {"mu": "default 10"},
         "reference": "none closed-form; the result reports the trajectory and statistics only"},
        {"name": "lorenz", "equation": "Lorenz 1963, sigma = 10, rho = 28, beta = 8/3", "dim": 3, "method": "BDF",
         "default_t_out": [1.0, 2.0, 3.0, 4.0, 5.0], "max_t": 50.0,
         "reference": "none closed-form; chaotic, so trajectories are tolerance-sensitive beyond a few Lyapunov times"}
    ])
}

// ------------------------------------------------------------------------------------------------
// CVODE problems
// ------------------------------------------------------------------------------------------------

fn get_f64(args: &Value, key: &str, default: f64, lo: f64, hi: f64) -> Result<f64, ToolOutcome> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let x = v
                .as_f64()
                .ok_or_else(|| bad_args(format!("{key} must be a number")))?;
            if !x.is_finite() || x < lo || x > hi {
                return Err(bad_args(format!(
                    "{key} = {x} outside the allowed range [{lo}, {hi}]"
                )));
            }
            Ok(x)
        }
    }
}

fn get_usize(
    args: &Value,
    key: &str,
    default: usize,
    lo: usize,
    hi: usize,
) -> Result<usize, ToolOutcome> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let x = v
                .as_u64()
                .ok_or_else(|| bad_args(format!("{key} must be a non-negative integer")))?
                as usize;
            if x < lo || x > hi {
                return Err(bad_args(format!(
                    "{key} = {x} outside the allowed range [{lo}, {hi}]"
                )));
            }
            Ok(x)
        }
    }
}

fn get_times(args: &Value, default: &[f64], max_t: f64) -> Result<Vec<f64>, ToolOutcome> {
    let Some(v) = args.get("t_out") else {
        return Ok(default.to_vec());
    };
    let arr = v
        .as_array()
        .ok_or_else(|| bad_args("t_out must be an array of numbers"))?;
    if arr.is_empty() || arr.len() > 50 {
        return Err(bad_args("t_out must contain between 1 and 50 times"));
    }
    let mut out = Vec::with_capacity(arr.len());
    let mut prev = 0.0;
    for x in arr {
        let t = x
            .as_f64()
            .ok_or_else(|| bad_args("t_out entries must be numbers"))?;
        if !t.is_finite() || t <= prev || t > max_t {
            return Err(bad_args(format!(
                "t_out must be strictly increasing, positive and at most {max_t}; got {t}"
            )));
        }
        out.push(t);
        prev = t;
    }
    Ok(out)
}

fn cvode_err(e: CvodeError, problem: &str) -> ToolOutcome {
    ToolOutcome::Err {
        ran: true,
        message: format!("CVODE failed on {problem}: {e}. No partial trajectory is returned."),
    }
}

struct Integrated {
    traj: Vec<(f64, Vec<f64>)>,
    steps: usize,
    rhs_evals: usize,
}

fn integrate<F>(
    mut solver: Cvode<F>,
    times: &[f64],
    problem: &str,
) -> Result<Integrated, ToolOutcome>
where
    F: FnMut(f64, &[f64], &mut [f64]) -> Result<(), String> + Send + Sync,
{
    let mut traj = Vec::with_capacity(times.len());
    for &tout in times {
        let (t, y) = solver
            .solve(tout, Task::Normal)
            .map_err(|e| cvode_err(e, problem))?;
        traj.push((t, y.to_vec()));
    }
    Ok(Integrated {
        traj,
        steps: solver.num_steps(),
        rhs_evals: solver.num_rhs_evals(),
    })
}

fn traj_json(traj: &[(f64, Vec<f64>)]) -> Value {
    Value::Array(traj.iter().map(|(t, y)| json!({"t": t, "y": y})).collect())
}

fn solve(args: &Value) -> ToolOutcome {
    match solve_inner(args) {
        Ok(v) | Err(v) => v,
    }
}

fn solve_inner(args: &Value) -> Result<ToolOutcome, ToolOutcome> {
    let problem = args
        .get("problem")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_args("problem is required"))?;
    let max_steps = get_usize(args, "max_steps", 500_000, 1, 1_000_000)?;
    match problem {
        "exponential" => {
            let times = get_times(args, &[0.1, 0.5, 1.0, 2.0, 5.0, 10.0], 100.0)?;
            let rtol = get_f64(args, "rtol", 1e-6, 1e-12, 1e-2)?;
            let atol = get_f64(args, "atol", 1e-10, 1e-16, 1e-2)?;
            let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
                ydot[0] = -y[0];
                Ok(())
            };
            let solver = Cvode::builder(Method::Bdf)
                .rtol(rtol)
                .atol(atol)
                .max_steps(max_steps)
                .build(rhs, 0.0, SerialVector::from_slice(&[1.0]))
                .map_err(|e| cvode_err(e, problem))?;
            let r = integrate(solver, &times, problem)?;
            let max_err = r
                .traj
                .iter()
                .map(|(t, y)| (y[0] - (-t).exp()).abs())
                .fold(0.0_f64, f64::max);
            Ok(ToolOutcome::Ok(json!({
                "ran": true, "problem": problem, "rtol": rtol, "atol": atol,
                "trajectory": traj_json(&r.traj), "steps": r.steps, "rhs_evals": r.rhs_evals,
                "checks": {"max_abs_error_vs_exp_minus_t": max_err,
                           "reference": "closed form y(t) = exp(-t)"}
            })))
        }
        "domain_exit" => {
            // y' = -sqrt(y), y(0) = 1 has the exact solution (1 - t/2)^2 for t <= 2, where it
            // reaches y = 0. Past t = 2 an integrator overshoots into y < 0 where the RHS is NaN
            // (outside its real domain), so the local error test fails repeatedly. This is a
            // genuine, documented failure case (used by the stdio isolation tests).
            let times = get_times(args, &[0.5, 1.0, 1.5, 3.0], 100.0)?;
            let rtol = get_f64(args, "rtol", 1e-6, 1e-12, 1e-2)?;
            let atol = get_f64(args, "atol", 1e-10, 1e-16, 1e-2)?;
            let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
                ydot[0] = -y[0].sqrt();
                Ok(())
            };
            // Past t = 2 the NaN surfaces as a Newton convergence failure (BDF and Adams alike).
            let solver = Cvode::builder(Method::Bdf)
                .rtol(rtol)
                .atol(atol)
                .max_steps(max_steps)
                .build(rhs, 0.0, SerialVector::from_slice(&[1.0]))
                .map_err(|e| cvode_err(e, problem))?;
            let r = integrate(solver, &times, problem)?;
            let max_err = r
                .traj
                .iter()
                .map(|(t, y)| (y[0] - (1.0 - t / 2.0).max(0.0).powi(2)).abs())
                .fold(0.0_f64, f64::max);
            Ok(ToolOutcome::Ok(json!({
                "ran": true, "problem": problem, "rtol": rtol, "atol": atol,
                "trajectory": traj_json(&r.traj), "steps": r.steps, "rhs_evals": r.rhs_evals,
                "checks": {"max_abs_error_vs_closed_form": max_err,
                           "reference": "closed form y(t) = (1 - t/2)^2 for t <= 2"}
            })))
        }
        "robertson" => {
            let times = get_times(args, &[0.4, 4.0, 40.0, 400.0, 4000.0, 40000.0], 4.0e10)?;
            let rtol = get_f64(args, "rtol", 1e-4, 1e-12, 1e-2)?;
            let atol = get_f64(args, "atol", 1e-8, 1e-16, 1e-2)?;
            let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
                ydot[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
                ydot[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
                ydot[2] = 3e7 * y[1] * y[1];
                Ok(())
            };
            let solver = Cvode::builder(Method::Bdf)
                .rtol(rtol)
                .atol(atol)
                .max_order(5)
                .init_step(1e-4)
                .max_steps(max_steps)
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
                .build(rhs, 0.0, SerialVector::from_slice(&[1.0, 0.0, 0.0]))
                .map_err(|e| cvode_err(e, problem))?;
            let r = integrate(solver, &times, problem)?;
            let mass_dev = r
                .traj
                .iter()
                .map(|(_, y)| (y[0] + y[1] + y[2] - 1.0).abs())
                .fold(0.0_f64, f64::max);
            let llnl = [9.8517e-01, 3.3864e-05, 1.4794e-02];
            let at04 = r.traj.iter().find(|(t, _)| (*t - 0.4).abs() < 1e-12);
            let llnl_check = at04.map(|(_, y)| {
                let rel: Vec<f64> = y
                    .iter()
                    .zip(llnl)
                    .map(|(a, b)| ((a - b) / b).abs())
                    .collect();
                json!({"t": 0.4, "llnl_reference": llnl, "computed": y, "max_rel_dev": rel.iter().cloned().fold(0.0, f64::max),
                       "note": "LLNL reference is printed to 5 significant figures, so agreement is meaningful to ~1e-4 relative"})
            });
            Ok(ToolOutcome::Ok(json!({
                "ran": true, "problem": problem, "rtol": rtol, "atol": atol,
                "trajectory": traj_json(&r.traj), "steps": r.steps, "rhs_evals": r.rhs_evals,
                "checks": {"max_mass_conservation_deviation": mass_dev,
                           "llnl_cvRoberts_dns_t0.4": llnl_check.unwrap_or(Value::Null)}
            })))
        }
        "vanderpol" => {
            let times = get_times(args, &[5.0, 10.0, 15.0, 20.0, 25.0], 3000.0)?;
            let rtol = get_f64(args, "rtol", 1e-6, 1e-12, 1e-2)?;
            let atol = get_f64(args, "atol", 1e-8, 1e-16, 1e-2)?;
            let mu = get_f64(args, "mu", 10.0, 0.0, 1000.0)?;
            let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
                ydot[0] = y[1];
                ydot[1] = mu * (1.0 - y[0] * y[0]) * y[1] - y[0];
                Ok(())
            };
            let solver = Cvode::builder(Method::Bdf)
                .rtol(rtol)
                .atol(atol)
                .max_steps(max_steps)
                .build(rhs, 0.0, SerialVector::from_slice(&[2.0, 0.0]))
                .map_err(|e| cvode_err(e, problem))?;
            let r = integrate(solver, &times, problem)?;
            Ok(ToolOutcome::Ok(json!({
                "ran": true, "problem": problem, "mu": mu, "rtol": rtol, "atol": atol,
                "trajectory": traj_json(&r.traj), "steps": r.steps, "rhs_evals": r.rhs_evals,
                "checks": {"reference": "none closed-form"}
            })))
        }
        "lorenz" => {
            let times = get_times(args, &[1.0, 2.0, 3.0, 4.0, 5.0], 50.0)?;
            let rtol = get_f64(args, "rtol", 1e-6, 1e-12, 1e-2)?;
            let atol = get_f64(args, "atol", 1e-8, 1e-16, 1e-2)?;
            let rhs = |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
                ydot[0] = 10.0 * (y[1] - y[0]);
                ydot[1] = y[0] * (28.0 - y[2]) - y[1];
                ydot[2] = y[0] * y[1] - (8.0 / 3.0) * y[2];
                Ok(())
            };
            let solver = Cvode::builder(Method::Bdf)
                .rtol(rtol)
                .atol(atol)
                .max_steps(max_steps)
                .build(rhs, 0.0, SerialVector::from_slice(&[1.0, 1.0, 1.0]))
                .map_err(|e| cvode_err(e, problem))?;
            let r = integrate(solver, &times, problem)?;
            Ok(ToolOutcome::Ok(json!({
                "ran": true, "problem": problem, "rtol": rtol, "atol": atol,
                "trajectory": traj_json(&r.traj), "steps": r.steps, "rhs_evals": r.rhs_evals,
                "checks": {"reference": "none closed-form; chaotic, tolerance-sensitive beyond a few Lyapunov times"}
            })))
        }
        other => Err(bad_args(format!(
            "unknown problem {other:?}; see list_problems"
        ))),
    }
}

// ------------------------------------------------------------------------------------------------
// qf-pgpe
// ------------------------------------------------------------------------------------------------

fn invariants(f: &ComplexField2D, c: &[Complex64]) -> Value {
    let (px, py) = f.momentum(c);
    json!({"norm": f.norm(c), "energy": f.energy(c), "momentum": [px, py]})
}

fn pgpe_run(args: &Value) -> ToolOutcome {
    match pgpe_inner(args) {
        Ok(v) | Err(v) => v,
    }
}

fn pgpe_inner(args: &Value) -> Result<ToolOutcome, ToolOutcome> {
    let initial = args
        .get("initial")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_args("initial is required (plane_wave | seeded)"))?;
    let n = get_usize(args, "n", 64, 16, 64)?;
    if ![16, 32, 64].contains(&n) {
        return Err(bad_args("n must be 16, 32 or 64"));
    }
    let l = get_f64(args, "l", 32.0, 1.0, 1000.0)?;
    let g = get_f64(args, "g", 1.0, -100.0, 100.0)?;
    let dt = get_f64(args, "dt", 0.005, 1e-5, 0.1)?;
    let t_end = get_f64(args, "t_end", 5.0, 0.0, 1.0e6)?;
    let steps = (t_end / dt).round();
    if steps > 4000.0 {
        return Err(bad_args(format!(
            "t_end/dt = {steps} steps exceeds the bound of 4000; shorten t_end or enlarge dt"
        )));
    }
    let field = ComplexField2D::new(n, l, g, dt);
    let x = field.coords();
    let (psi0, exact_omega, k) = match initial {
        "plane_wave" => {
            let m = args.get("mode").and_then(Value::as_i64).unwrap_or(3);
            let k = 2.0 * std::f64::consts::PI * m as f64 / l;
            if k.abs() > field.kcut {
                return Err(bad_args(format!(
                    "mode {m} gives |k| = {:.4} beyond the projection cutoff k_cut = {:.4}; the projected \
                     state would be zero, so this would not test anything",
                    k.abs(),
                    field.kcut
                )));
            }
            let psi: Vec<Complex64> = (0..n * n)
                .map(|idx| Complex64::new(0.0, k * x[idx / n]).exp())
                .collect();
            (psi, Some(0.5 * k * k + g), Some(k))
        }
        "seeded" => {
            let k1 = 2.0 * std::f64::consts::PI / l;
            let k2 = 2.0 * k1;
            let psi: Vec<Complex64> = (0..n * n)
                .map(|idx| {
                    let (xi, yj) = (x[idx / n], x[idx % n]);
                    Complex64::new(1.0, 0.0)
                        + 0.1 * Complex64::new(0.0, k1 * xi + k2 * yj).exp()
                        + 0.05 * Complex64::new(0.0, -k2 * xi + k1 * yj).exp()
                })
                .collect();
            (psi, None, None)
        }
        other => return Err(bad_args(format!("unknown initial state {other:?}"))),
    };
    let c0 = field.modes(&psi0);
    let before = invariants(&field, &c0);
    let c1 = field.run(&c0, t_end);
    let after = invariants(&field, &c1);
    let n0 = field.norm(&c0);
    let e0 = field.energy(&c0);
    let (p0x, p0y) = field.momentum(&c0);
    let (p1x, p1y) = field.momentum(&c1);
    let mut checks = json!({
        "norm_rel_drift": (field.norm(&c1) - n0).abs() / n0,
        "energy_rel_drift": (field.energy(&c1) - e0).abs() / e0.abs().max(1e-300),
        "momentum_abs_drift": (p1x - p0x).abs().max((p1y - p0y).abs()),
        "reference": "norm and momentum are exact invariants of the projected GPE; the IF-RK4 energy \
                      drift is O(dt^4) (qf-pgpe criteria: norm 1e-8, momentum 1e-10 over t=20 at dt=0.005)"
    });
    if let (Some(w), Some(k)) = (exact_omega, k) {
        let psi1 = field.psi(&c1);
        let t = steps * dt;
        let phase = Complex64::new(0.0, -w * t).exp();
        let err = psi0
            .iter()
            .zip(&psi1)
            .map(|(p0, p1)| (p1 - p0 * phase).norm())
            .fold(0.0_f64, f64::max);
        checks["plane_wave_max_error_vs_exact"] = json!(err);
        checks["exact_solution"] = json!(format!(
            "psi = exp(i(k x - w t)), k = {k:.6}, w = k^2/2 + g = {w:.6}"
        ));
    }
    Ok(ToolOutcome::Ok(json!({
        "ran": true, "initial": initial, "n": n, "l": l, "g": g, "dt": dt, "t_end": steps * dt,
        "steps": steps as u64, "before": before, "after": after, "checks": checks
    })))
}

// ------------------------------------------------------------------------------------------------
// CMB dark-energy phase-transition bound
// ------------------------------------------------------------------------------------------------

fn cmb_bound(args: &Value) -> ToolOutcome {
    let beta_over_h = match get_f64(args, "beta_over_h", 100.0, 1.0, 1000.0) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let zpt = match get_f64(args, "zpt", 0.1, 0.01, 0.9) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("planck");
    if mode != "planck" && mode != "cosmic_variance" {
        return bad_args(format!(
            "mode must be 'planck' or 'cosmic_variance', got {mode:?}"
        ));
    }
    let (r, ell_peak) = if mode == "cosmic_variance" {
        r_bound_2sigma(zpt, beta_over_h, 3.84, 0.1, sigma_cv)
    } else {
        r_bound_2sigma(zpt, beta_over_h, 3.84, 0.1, sigma_ell)
    };
    if r.is_nan() || !r.is_finite() {
        return ToolOutcome::Err {
            ran: true,
            message: "r_bound_2sigma returned non-finite; probe value did not converge".to_string(),
        };
    }
    ToolOutcome::Ok(json!({
        "ran": true,
        "r_2sigma": r,
        "ell_peak": ell_peak,
        "inputs": {
            "beta_over_h": beta_over_h,
            "zpt": zpt,
            "mode": mode
        },
        "reference": "Koren-Tsai-Wang (arXiv:2509.07076), exact spectrum from Elor et al. \
                      (arXiv:2311.16222), checked against Planck 2018 TT power spectrum. \
                      See qf-cmb-cascade crate docs for derivation, approximations, and \
                      agreement caveats.",
        "note": "This is a 2-sigma chi^2 exclusion bound at fixed (beta/H*, zpt). It does not \
                 claim the discrete-cascade model is preferred or disfavoured, and it does not \
                 map any quantum-fluid/PGPE simulation to model parameters."
    }))
}

// ------------------------------------------------------------------------------------------------
// JSON-RPC / MCP dispatch
// ------------------------------------------------------------------------------------------------

fn rpc_result(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn rpc_error(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Convert a tool outcome to an MCP `CallToolResult`.
pub fn call_tool_result(outcome: &ToolOutcome) -> Value {
    match outcome {
        ToolOutcome::Ok(v) => json!({
            "content": [{"type": "text", "text": v.to_string()}],
            "structuredContent": v,
            "isError": false
        }),
        ToolOutcome::Err { ran, message } => {
            let body = json!({"ran": ran, "error": message});
            json!({
                "content": [{"type": "text", "text": body.to_string()}],
                "structuredContent": body,
                "isError": true
            })
        }
    }
}

/// Handle one JSON-RPC message. `exec` runs a tool (in the real server: in a worker subprocess).
/// Returns `None` for notifications, which get no response.
pub fn handle_message(msg: &Value, exec: &dyn Fn(&str, &Value) -> ToolOutcome) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let Some(id) = id else {
        return None; // notification (e.g. notifications/initialized)
    };
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    Some(match method {
        "initialize" => {
            let requested = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("");
            let version = if SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
                requested
            } else {
                SUPPORTED_PROTOCOL_VERSIONS[SUPPORTED_PROTOCOL_VERSIONS.len() - 1]
            };
            rpc_result(
                &id,
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
                    "instructions": "rusty-SUNDIALS solvers for agents. `ran: false` is never a pass; a \
                                     solver failure is an error with no partial result; each known-answer \
                                     check names its independent reference."
                }),
            )
        }
        "ping" => rpc_result(&id, json!({})),
        "tools/list" => rpc_result(&id, json!({"tools": tool_definitions()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let known = tool_definitions()
                .as_array()
                .is_some_and(|ts| ts.iter().any(|t| t["name"] == name));
            if !known {
                rpc_error(&id, -32602, &format!("unknown tool {name:?}"))
            } else {
                rpc_result(&id, call_tool_result(&exec(name, &args)))
            }
        }
        other => rpc_error(&id, -32601, &format!("method not found: {other}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(o: ToolOutcome) -> Value {
        match o {
            ToolOutcome::Ok(v) => v,
            ToolOutcome::Err { message, .. } => panic!("expected Ok, got error: {message}"),
        }
    }

    #[test]
    fn exponential_matches_closed_form() {
        let v = ok(run_tool("solve", &json!({"problem": "exponential"})));
        let err = v["checks"]["max_abs_error_vs_exp_minus_t"]
            .as_f64()
            .unwrap();
        assert!(err < 1e-4, "max error {err}");
        assert_eq!(v["trajectory"].as_array().unwrap().len(), 6);
    }

    #[test]
    fn robertson_conserves_mass_and_matches_llnl_reference() {
        let v = ok(run_tool("solve", &json!({"problem": "robertson"})));
        let dev = v["checks"]["max_mass_conservation_deviation"]
            .as_f64()
            .unwrap();
        assert!(dev < 1e-6, "mass deviation {dev}");
        let rel = v["checks"]["llnl_cvRoberts_dns_t0.4"]["max_rel_dev"]
            .as_f64()
            .unwrap();
        assert!(rel < 2e-3, "relative deviation from LLNL reference {rel}");
    }

    #[test]
    fn planted_defect_llnl_check_can_fail() {
        // Control: at a looser tolerance the t=0.4 comparison must move — the check reads the
        // solver's numbers, not a stored constant. (atol = 1e-2 is not usable here: y2 ~ 1e-5, and
        // CVODE then exhausts its step budget by t ~ 0.014, which `solve` reports as an error.)
        let tight = ok(run_tool("solve", &json!({"problem": "robertson"})));
        let loose = ok(run_tool(
            "solve",
            &json!({"problem": "robertson", "rtol": 1e-2, "atol": 1e-6}),
        ));
        let a = tight["checks"]["llnl_cvRoberts_dns_t0.4"]["max_rel_dev"]
            .as_f64()
            .unwrap();
        let b = loose["checks"]["llnl_cvRoberts_dns_t0.4"]["max_rel_dev"]
            .as_f64()
            .unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn solver_failure_is_an_error_without_partial_result() {
        let o = run_tool(
            "solve",
            &json!({"problem": "vanderpol", "mu": 1000.0, "max_steps": 5,
                                          "t_out": [100.0]}),
        );
        match o {
            ToolOutcome::Err { ran, message } => {
                assert!(ran, "the solver did run before failing");
                assert!(message.contains("No partial trajectory"), "{message}");
            }
            ToolOutcome::Ok(v) => panic!("a 5-step budget cannot reach t=100 on stiff VdP: {v}"),
        }
    }

    #[test]
    fn out_of_range_arguments_do_not_run() {
        for bad in [
            json!({"problem": "exponential", "rtol": 0.5}),
            json!({"problem": "lorenz", "t_out": [1.0, 0.5]}),
            json!({"problem": "lorenz", "t_out": [60.0]}),
            json!({"problem": "nope"}),
        ] {
            match run_tool("solve", &bad) {
                ToolOutcome::Err { ran, .. } => assert!(!ran, "{bad}"),
                ToolOutcome::Ok(v) => panic!("accepted bad arguments {bad}: {v}"),
            }
        }
    }

    #[test]
    fn pgpe_plane_wave_is_exact_and_invariants_hold() {
        let v = ok(run_tool(
            "pgpe_run",
            &json!({"initial": "plane_wave", "n": 32, "t_end": 2.0}),
        ));
        let err = v["checks"]["plane_wave_max_error_vs_exact"]
            .as_f64()
            .unwrap();
        assert!(err < 1e-9, "plane wave error {err}");
        assert!(v["checks"]["norm_rel_drift"].as_f64().unwrap() < 1e-10);
    }

    #[test]
    fn pgpe_seeded_conserves_norm_and_momentum() {
        let v = ok(run_tool(
            "pgpe_run",
            &json!({"initial": "seeded", "n": 32, "t_end": 2.0}),
        ));
        assert!(v["checks"]["norm_rel_drift"].as_f64().unwrap() < 1e-8);
        assert!(v["checks"]["momentum_abs_drift"].as_f64().unwrap() < 1e-10);
    }

    #[test]
    fn pgpe_rejects_mode_beyond_cutoff_and_oversized_runs() {
        for bad in [
            json!({"initial": "plane_wave", "n": 16, "mode": 8}),
            json!({"initial": "seeded", "t_end": 100.0, "dt": 0.005}),
            json!({"initial": "seeded", "n": 48}),
        ] {
            match run_tool("pgpe_run", &bad) {
                ToolOutcome::Err { ran, .. } => assert!(!ran, "{bad}"),
                ToolOutcome::Ok(v) => panic!("accepted {bad}: {v}"),
            }
        }
    }

    #[test]
    fn dispatch_handshake_tools_and_errors() {
        let exec = |n: &str, a: &Value| run_tool(n, a);
        let init = handle_message(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2025-06-18"}}),
            &exec,
        )
        .unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
        let odd = handle_message(
            &json!({"jsonrpc": "2.0", "id": 2, "method": "initialize",
                    "params": {"protocolVersion": "1999-01-01"}}),
            &exec,
        )
        .unwrap();
        assert_eq!(odd["result"]["protocolVersion"], "2025-11-25");
        assert!(
            handle_message(
                &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
                &exec
            )
            .is_none()
        );
        let list = handle_message(
            &json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"}),
            &exec,
        )
        .unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 5);
        let unk = handle_message(
            &json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "rm_rf"}}),
            &exec,
        )
        .unwrap();
        assert_eq!(unk["error"]["code"], -32602);
        let err = handle_message(
            &json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call",
                    "params": {"name": "solve", "arguments": {"problem": "nope"}}}),
            &exec,
        )
        .unwrap();
        assert_eq!(err["result"]["isError"], true);
        assert_eq!(err["result"]["structuredContent"]["ran"], false);
    }

    #[test]
    fn outcome_roundtrips_through_worker_json() {
        for o in [
            ToolOutcome::Ok(json!({"ran": true, "x": 1})),
            ToolOutcome::Err {
                ran: true,
                message: "boom".into(),
            },
        ] {
            assert_eq!(ToolOutcome::from_json(&o.to_json()), o);
        }
        assert!(matches!(
            ToolOutcome::from_json(&json!({"garbage": 1})),
            ToolOutcome::Err { ran: false, .. }
        ));
    }
}
