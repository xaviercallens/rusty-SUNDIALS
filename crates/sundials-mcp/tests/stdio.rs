//! End-to-end tests of the real binary over stdio. The central one reproduces the pitfall this
//! server is built around: `cvode/src/solver.rs` prints `ERROR FAIL 3: …` to stdout when the local
//! error test fails, which on a stdio protocol corrupts the stream. With isolation the protocol
//! stream stays pure JSON-RPC; the control (isolation disabled) must show the corruption, so the
//! test demonstrably can fail.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

const BIN: &str = env!("CARGO_BIN_EXE_sundials-mcp");

/// A failing solve: exponential decay at rtol 1e-9 / atol 1e-12 trips CVODE's error-test failure
/// path (verified against the solver when this test was written), which prints to stdout.
fn failing_solve(id: i64) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
           "params": {"name": "solve", "arguments": {"problem": "exponential", "rtol": 1e-9, "atol": 1e-12}}})
}

fn session(messages: &[Value], env: &[(&str, &str)]) -> (String, String) {
    let mut cmd = Command::new(BIN);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("spawn sundials-mcp");
    {
        let stdin = child.stdin.as_mut().unwrap();
        for m in messages {
            writeln!(stdin, "{m}").unwrap();
        }
    }
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn is_pure_jsonrpc(stdout: &str) -> bool {
    stdout
        .lines()
        .all(|l| serde_json::from_str::<Value>(l).is_ok_and(|v| v["jsonrpc"] == "2.0"))
}

fn handshake() -> Vec<Value> {
    vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                          "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    ]
}

#[test]
fn solver_stdout_never_reaches_the_protocol_stream() {
    let mut msgs = handshake();
    msgs.push(failing_solve(2));
    msgs.push(json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                     "params": {"name": "solve", "arguments": {"problem": "exponential"}}}));
    let (stdout, stderr) = session(&msgs, &[]);
    assert!(
        is_pure_jsonrpc(&stdout),
        "protocol stream corrupted:\n{stdout}"
    );
    let resps: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(resps.len(), 3, "{stdout}");
    let fail = &resps[1]["result"];
    assert_eq!(fail["isError"], true);
    assert_eq!(fail["structuredContent"]["ran"], true);
    assert_eq!(resps[2]["result"]["isError"], false);
    assert!(
        stderr.contains("ERROR FAIL"),
        "the solver's diagnostic should be routed to stderr: {stderr}"
    );
}

#[test]
fn control_without_isolation_the_stream_is_corrupted() {
    let mut msgs = handshake();
    msgs.push(failing_solve(2));
    let (stdout, _) = session(&msgs, &[("SUNDIALS_MCP_NO_ISOLATION", "1")]);
    assert!(
        !is_pure_jsonrpc(&stdout),
        "control failed to reproduce the corruption; the isolation test would then prove nothing:\n{stdout}"
    );
    assert!(stdout.contains("ERROR FAIL"));
}

#[test]
fn timeout_kills_worker_and_reports_not_run() {
    let mut msgs = handshake();
    msgs.push(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                     "params": {"name": "pgpe_run", "arguments": {"initial": "seeded", "n": 64, "t_end": 20.0}}}));
    let (stdout, _) = session(&msgs, &[("SUNDIALS_MCP_TIMEOUT_MS", "1")]);
    let resps: Vec<Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let r = &resps[1]["result"];
    assert_eq!(r["isError"], true);
    assert_eq!(r["structuredContent"]["ran"], false);
    assert!(
        r["structuredContent"]["error"]
            .as_str()
            .unwrap()
            .contains("timed out")
    );
}

#[test]
fn malformed_input_gets_a_parse_error_not_a_crash() {
    let (stdout, _) = session(&[json!("not an object")], &[]);
    // a bare JSON string is valid JSON but not a request; send raw garbage too
    let mut child = Command::new(BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    writeln!(child.stdin.as_mut().unwrap(), "{{this is not json").unwrap();
    drop(child.stdin.take());
    let out = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
    let v: Value = serde_json::from_str(out.lines().next().unwrap()).unwrap();
    assert_eq!(v["error"]["code"], -32700);
    assert!(
        stdout.is_empty(),
        "a message without an id is a notification and gets no reply: {stdout}"
    );
}
