//! A library must not write to stdout: `solver.rs` used to `println!("ERROR FAIL …")` on its
//! failure paths, which corrupted stdio protocols built on top of it (crates/sundials-mcp). The
//! diagnostics now go to stderr. This test proves it end to end by re-running this test binary
//! as a child process on a solve that deterministically reaches the "ERROR FAIL 3" path, and
//! capturing both of the child's streams.
//!
//! The fixture: y' = step(t - 0.5) with `min_step = 0.1`, rtol 1e-10, atol 1e-14. The step that
//! straddles the discontinuity has a local error estimate ~h/atol regardless of h, and the
//! minimum step size stops the controller from shrinking past it, so the local error test fails
//! MAX_ERR_TEST_FAILS times in one step and the solver returns `ErrTestFailure`.

use std::process::{Command, Stdio};

use cvode::{Cvode, CvodeError, Method, Task};
use nvector::SerialVector;

fn jump(t: f64, _y: &[f64], yd: &mut [f64]) -> Result<(), String> {
    yd[0] = if t < 0.5 { 0.0 } else { 1.0 };
    Ok(())
}

fn run_to_error_test_failure(method: Method) -> Result<(f64, Vec<f64>), CvodeError> {
    let mut cv = Cvode::builder(method)
        .rtol(1e-10)
        .atol(1e-14)
        .min_step(0.1)
        .max_steps(10_000)
        .build(jump, 0.0, SerialVector::from_slice(&[1.0]))
        .unwrap();
    cv.solve(1.0, Task::Normal).map(|(t, y)| (t, y.to_vec()))
}

/// The fixture reaches the error-test-failure path for both methods (this is also the child
/// process of the test below).
#[test]
fn child_reaches_error_fail_path() {
    for method in [Method::Bdf, Method::Adams] {
        let r = run_to_error_test_failure(method);
        assert!(
            matches!(
                r,
                Err(CvodeError::Solver(
                    sundials_core::SundialsError::ErrTestFailure
                ))
            ),
            "{method:?}: expected ErrTestFailure, got {r:?}"
        );
    }
}

#[test]
fn error_fail_diagnostic_goes_to_stderr_not_stdout() {
    let exe = std::env::current_exe().expect("test binary path");
    let out = Command::new(exe)
        .args(["child_reaches_error_fail_path", "--exact", "--nocapture"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn child test");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "child failed:\n{stdout}\n{stderr}");
    assert!(
        stderr.contains("ERROR FAIL 3"),
        "the diagnostic should appear on stderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("ERROR FAIL"),
        "the diagnostic leaked to stdout:\n{stdout}"
    );
    // The child really ran the test (the harness reports it on stdout).
    assert!(stdout.contains("test result: ok. 1 passed"), "{stdout}");
}
