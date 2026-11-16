//! Collects provenance at build time. Deterministic: no timestamps, no host data.
//! Same checkout + same toolchain + same env => same emitted strings.

#[path = "src/gitinfo.rs"]
mod gitinfo;
#[path = "src/sha256.rs"]
mod sha256;

use std::path::{Path, PathBuf};
use std::process::Command;

fn emit(key: &str, value: &str) {
    // cargo:rustc-env values must be single-line.
    let v: String = value
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    println!("cargo:rustc-env=RUSTY_PROV_{key}={v}");
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn git_out(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let o = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    o.status.success().then_some(o.stdout)
}

fn rerun_triggers(dir: &Path) {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RUSTY_PREREG_FILE");
    let Some(gd) = git_out(dir, &["rev-parse", "--absolute-git-dir"]) else {
        return;
    };
    let gd = PathBuf::from(String::from_utf8_lossy(&gd).trim());
    for name in ["HEAD", "index", "packed-refs"] {
        let p = gd.join(name);
        if p.exists() {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    }
    // A new commit moves the branch ref file, not HEAD itself.
    if let Some(r) = git_out(dir, &["symbolic-ref", "-q", "HEAD"]) {
        let r = String::from_utf8_lossy(&r).trim().to_string();
        if let Some(common) = git_out(dir, &["rev-parse", "--git-common-dir"]) {
            let common = PathBuf::from(String::from_utf8_lossy(&common).trim());
            let common = if common.is_absolute() {
                common
            } else {
                dir.join(common)
            };
            let p = common.join(&r);
            if p.exists() {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
    // Editing a tracked file does not touch .git/index until `git add`, so watch
    // every tracked file so that a dirty change triggers a rebuild too.
    if let (Some(top), Some(list)) = (
        git_out(dir, &["rev-parse", "--show-toplevel"]),
        git_out(dir, &["ls-files", "-z"]),
    ) {
        let top = PathBuf::from(String::from_utf8_lossy(&top).trim());
        for f in list.split(|&b| b == 0).filter(|f| !f.is_empty()) {
            let f = String::from_utf8_lossy(f);
            if !f.contains('\n') {
                println!("cargo:rerun-if-changed={}", top.join(f.as_ref()).display());
            }
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env("CARGO_MANIFEST_DIR"));
    let g = gitinfo::collect(&manifest_dir);
    emit("GIT_SHA", &g.sha);
    emit("GIT_STATUS", g.status.as_str());
    emit("GIT_DIRTY", if g.dirty { "true" } else { "false" });
    if let Some(h) = &g.dirty_diff_sha256 {
        emit("DIRTY_DIFF_SHA256", h);
    }
    emit("UNTRACKED_COUNT", &g.untracked_count.to_string());

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let verbose = Command::new(&rustc)
        .args(["--version", "--verbose"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let field = |name: &str| -> String {
        verbose
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    };
    emit("RUSTC_RELEASE", &field("release:"));
    emit("RUSTC_COMMIT_HASH", &field("commit-hash:"));
    emit("TARGET", &env("TARGET"));
    emit("PROFILE", &env("PROFILE"));
    emit("OPT_LEVEL", &env("OPT_LEVEL"));

    let prereg = std::env::var("RUSTY_PREREG_FILE").unwrap_or_default();
    if !prereg.is_empty() {
        match std::fs::read(&prereg) {
            Ok(bytes) => {
                emit("PREREG_PATH", &prereg);
                emit("PREREG_SHA256", &sha256::hex_digest(&bytes));
                println!("cargo:rerun-if-changed={prereg}");
            }
            // Fail loudly: silently dropping the hash would fake an unregistered build.
            Err(e) => panic!("RUSTY_PREREG_FILE={prereg} is set but unreadable: {e}"),
        }
    }
    rerun_triggers(&manifest_dir);
}
