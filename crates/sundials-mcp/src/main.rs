//! stdio entry point. Two modes:
//!
//! * default — the MCP protocol process. Reads newline-delimited JSON-RPC on stdin, writes responses
//!   on stdout, and **never runs solver code itself**: each solver tool call is delegated to a worker
//!   subprocess (see below), so nothing a solver prints can reach the protocol stream.
//! * `--worker <tool> <arguments-json> <result-path>` — runs one tool and writes the outcome as JSON
//!   to `<result-path>`. Its stdout is a duplicate of the parent's stderr handle, so solver
//!   diagnostics (e.g. `cvode/src/solver.rs`'s `println!("ERROR FAIL …")`) land in the client's log,
//!   not in the protocol.
//!
//! `SUNDIALS_MCP_TIMEOUT_SECS` (default 120; `SUNDIALS_MCP_TIMEOUT_MS` overrides it, for tests)
//! bounds each worker; on timeout the worker is killed and
//! the call returns `ran: false`. `SUNDIALS_MCP_NO_ISOLATION=1` runs solvers in the protocol process
//! instead — **test-only**, it exists so the isolation test has a control that must fail.
#![forbid(unsafe_code)]

use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sundials_mcp::{ToolOutcome, handle_message, is_inline_tool, run_tool};

static CALLS: AtomicU64 = AtomicU64::new(0);

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() >= 2 && argv[1] == "--worker" {
        std::process::exit(worker(&argv[2..]));
    }
    serve();
}

fn worker(args: &[String]) -> i32 {
    let [tool, arguments, result_path] = args else {
        eprintln!("usage: sundials-mcp --worker <tool> <arguments-json> <result-path>");
        return 2;
    };
    let parsed: Value = serde_json::from_str(arguments).unwrap_or(json!({}));
    let outcome = run_tool(tool, &parsed);
    match std::fs::write(result_path, outcome.to_json().to_string()) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("sundials-mcp worker: cannot write result: {e}");
            3
        }
    }
}

fn timeout() -> Duration {
    if let Some(ms) = std::env::var("SUNDIALS_MCP_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    {
        return Duration::from_millis(ms.max(1));
    }
    let secs = std::env::var("SUNDIALS_MCP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(120);
    Duration::from_secs(secs.max(1))
}

/// A duplicate of our stderr, to become the worker's stdout.
#[cfg(unix)]
fn dup_stderr() -> std::io::Result<Stdio> {
    use std::os::fd::AsFd;
    Ok(Stdio::from(std::io::stderr().as_fd().try_clone_to_owned()?))
}

/// A duplicate of our stderr, to become the worker's stdout.
#[cfg(windows)]
fn dup_stderr() -> std::io::Result<Stdio> {
    use std::os::windows::io::AsHandle;
    Ok(Stdio::from(
        std::io::stderr().as_handle().try_clone_to_owned()?,
    ))
}

/// No handle duplication here: refuse rather than silently lose the stdout isolation.
#[cfg(not(any(unix, windows)))]
fn dup_stderr() -> std::io::Result<Stdio> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "stderr duplication is not supported on this platform",
    ))
}

/// Run a solver tool in an isolated worker subprocess.
fn run_isolated(tool: &str, args: &Value) -> ToolOutcome {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            return ToolOutcome::Err {
                ran: false,
                message: format!("cannot locate own executable: {e}"),
            };
        }
    };
    let n = CALLS.fetch_add(1, Ordering::Relaxed);
    let result_path =
        std::env::temp_dir().join(format!("sundials-mcp-{}-{n}.json", std::process::id()));
    // The worker's stdout is a duplicate of our stderr: solver prints go to the log, never to the protocol.
    let stdout_for_worker = match dup_stderr() {
        Ok(stdio) => stdio,
        Err(e) => {
            return ToolOutcome::Err {
                ran: false,
                message: format!("cannot duplicate stderr: {e}"),
            };
        }
    };
    let mut child = match Command::new(exe)
        .arg("--worker")
        .arg(tool)
        .arg(args.to_string())
        .arg(&result_path)
        .stdin(Stdio::null())
        .stdout(stdout_for_worker)
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return ToolOutcome::Err {
                ran: false,
                message: format!("cannot spawn worker: {e}"),
            };
        }
    };
    let deadline = Instant::now() + timeout();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&result_path);
                return ToolOutcome::Err {
                    ran: false,
                    message: format!(
                        "timed out after {} ms; the worker was killed and no result exists",
                        timeout().as_millis()
                    ),
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                return ToolOutcome::Err {
                    ran: false,
                    message: format!("cannot wait on worker: {e}"),
                };
            }
        }
    };
    let outcome = match std::fs::read_to_string(&result_path) {
        Ok(s) => match serde_json::from_str::<Value>(&s) {
            Ok(v) => ToolOutcome::from_json(&v),
            Err(e) => ToolOutcome::Err {
                ran: false,
                message: format!("worker wrote unparseable result: {e}"),
            },
        },
        Err(_) => ToolOutcome::Err {
            ran: false,
            message: format!("worker exited ({status}) without a result; treat as not run"),
        },
    };
    let _ = std::fs::remove_file(&result_path);
    outcome
}

fn execute(tool: &str, args: &Value) -> ToolOutcome {
    if is_inline_tool(tool) || std::env::var_os("SUNDIALS_MCP_NO_ISOLATION").is_some() {
        run_tool(tool, args)
    } else {
        run_isolated(tool, args)
    }
}

fn serve() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle_message(&msg, &execute),
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null,
                                  "error": {"code": -32700, "message": format!("parse error: {e}")}})),
        };
        if let Some(resp) = response {
            let mut out = stdout.lock();
            if writeln!(out, "{resp}").and_then(|_| out.flush()).is_err() {
                break;
            }
        }
    }
}
