//! Tests for the opt-in atol-dominance monitor (issue 17).

use cvode::{AtolMonitorReport, Cvode, CvodeError, Method, Task};
use nvector::SerialVector;

const TOUTS: [f64; 6] = [0.4, 4.0, 40.0, 400.0, 4000.0, 40000.0];

/// Outputs (as raw bits), step count, RHS count, and the monitor report.
type RunOut = (Vec<u64>, usize, usize, Option<AtolMonitorReport>);

fn robertson(rtol: f64, atol: f64, monitor: bool) -> Result<RunOut, CvodeError> {
    let rhs = |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
        f[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        f[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        f[2] = 3e7 * y[1] * y[1];
        Ok(())
    };
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(rtol)
        .atol(atol)
        .max_steps(100_000)
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
        .build(rhs, 0.0, SerialVector::from_slice(&[1.0, 0.0, 0.0]))?;
    if monitor {
        solver.enable_atol_monitor();
    }
    let mut bits = Vec::new();
    for &tout in &TOUTS {
        let (_, y) = solver.solve(tout, Task::Normal)?;
        bits.extend(y.iter().map(|v| v.to_bits()));
    }
    Ok((
        bits,
        solver.num_steps(),
        solver.num_rhs_evals(),
        solver.atol_monitor_report(),
    ))
}

#[test]
fn monitor_is_off_by_default() -> Result<(), CvodeError> {
    let (_, _, _, report) = robertson(1e-4, 1e-7, false)?;
    assert!(report.is_none());
    Ok(())
}

#[test]
fn flags_y2_on_robertson_at_published_tolerance() -> Result<(), CvodeError> {
    let (_, steps, _, report) = robertson(1e-4, 1e-7, true)?;
    let r = report.expect("monitor enabled");
    assert_eq!(r.components[1].steps_monitored, steps);
    assert_eq!(r.flagged_components(), vec![1]);
    Ok(())
}

#[test]
fn does_not_flag_y2_when_atol_is_small() -> Result<(), CvodeError> {
    let (_, _, _, report) = robertson(1e-4, 1e-13, true)?;
    let r = report.expect("monitor enabled");
    assert!(!r.components[1].flagged);
    assert!(r.components[1].steps_hit < r.components[1].steps_monitored / 2);
    Ok(())
}

#[test]
fn enabling_monitor_leaves_solution_bit_identical() -> Result<(), CvodeError> {
    for &(rtol, atol) in &[(1e-4, 1e-7), (1e-8, 1e-11), (1e-4, 1e-13)] {
        let off = robertson(rtol, atol, false)?;
        let on = robertson(rtol, atol, true)?;
        assert_eq!(off.0, on.0, "solution bits differ at rtol={rtol}");
        assert_eq!(off.1, on.1, "step counts differ");
        assert_eq!(off.2, on.2, "RHS counts differ");
    }
    Ok(())
}

#[test]
fn adams_run_is_bit_identical_and_cannot_flag() -> Result<(), CvodeError> {
    let run = |monitor: bool| -> Result<(Vec<u64>, Option<AtolMonitorReport>), CvodeError> {
        let rhs = |_t: f64, y: &[f64], f: &mut [f64]| -> Result<(), String> {
            f[0] = -y[0];
            f[1] = 1e-6 * y[0] - y[1];
            Ok(())
        };
        let mut solver = Cvode::builder(Method::Adams).rtol(1e-4).atol(1e-7).build(
            rhs,
            0.0,
            SerialVector::from_slice(&[1.0, 0.0]),
        )?;
        if monitor {
            solver.enable_atol_monitor();
        }
        let mut bits = Vec::new();
        for k in 1..=10 {
            let (_, y) = solver.solve(k as f64, Task::Normal)?;
            bits.extend(y.iter().map(|v| v.to_bits()));
        }
        Ok((bits, solver.atol_monitor_report()))
    };
    let (a, _) = run(false)?;
    let (b, report) = run(true)?;
    assert_eq!(a, b);
    let r = report.expect("monitor enabled");
    assert!(!r.any_flagged());
    assert!(r.components[1].steps_atol_dominated > 0);
    Ok(())
}
