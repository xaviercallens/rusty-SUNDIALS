//! Build-time provenance for rusty-SUNDIALS binaries and JSON outputs.
//!
//! # What is recorded
//! Git commit SHA of the checkout this crate was compiled from, whether tracked
//! files differed from HEAD (and a SHA-256 of `git diff HEAD --binary` if so),
//! the count of untracked files, the `rustc` release and commit hash, target
//! triple, profile, opt-level, this crate's version, and optionally the path and
//! SHA-256 of a preregistration file named by the env var `RUSTY_PREREG_FILE`
//! at build time. Everything is deterministic: no timestamps.
//!
//! # What is NOT recorded
//! * Untracked files are only counted, never hashed.
//! * Versions of third-party dependencies (see `Cargo.lock`).
//! * The build host, user, or environment variables other than the above.
//! * Anything about a dirty or non-git build beyond the flags below: such a
//!   build is **not pinned** ([`Provenance::is_pinned`] returns `false`).
//!
//! # Downstream crates that depend on rusty-SUNDIALS by path
//! The build script runs in *this crate's own directory*, so the values describe
//! the rusty-SUNDIALS checkout, not the downstream project. A downstream binary
//! that has `sundials-provenance` as a path dependency therefore gets the
//! dependency checkout's SHA by calling [`build_info`]; record that next to the
//! downstream crate's own git SHA.

pub mod gitinfo;
pub mod sha256;

use gitinfo::{GitStatus, is_pinned_state};
use std::fmt::Write as _;
use std::io::{self, Read};
use std::path::Path;

/// Provenance of the rusty-SUNDIALS checkout a binary was built from.
#[derive(Debug, Clone, PartialEq)]
pub struct Provenance {
    /// 40-hex commit SHA, or `"unknown"`.
    pub git_sha: String,
    /// How git behaved at build time.
    pub git_status: GitStatus,
    /// Tracked files differed from HEAD.
    pub dirty: bool,
    /// SHA-256 of `git diff HEAD --binary` when dirty.
    pub dirty_diff_sha256: Option<String>,
    /// Untracked, non-ignored file count (counted, not hashed).
    pub untracked_count: usize,
    /// `CARGO_PKG_VERSION` of this crate (the workspace version).
    pub crate_version: String,
    /// `rustc` release, e.g. `1.90.0`.
    pub rustc_release: String,
    /// `rustc` commit hash.
    pub rustc_commit_hash: String,
    /// Target triple.
    pub target: String,
    /// Cargo profile.
    pub profile: String,
    /// Optimisation level.
    pub opt_level: String,
    /// Preregistration file path given at build time.
    pub prereg_path: Option<String>,
    /// SHA-256 of that file's bytes at build time.
    pub prereg_sha256: Option<String>,
}

/// Provenance baked in at compile time.
pub fn build_info() -> Provenance {
    Provenance {
        git_sha: env!("RUSTY_PROV_GIT_SHA").to_string(),
        git_status: GitStatus::parse(env!("RUSTY_PROV_GIT_STATUS")),
        dirty: env!("RUSTY_PROV_GIT_DIRTY") == "true",
        dirty_diff_sha256: option_env!("RUSTY_PROV_DIRTY_DIFF_SHA256").map(str::to_string),
        untracked_count: env!("RUSTY_PROV_UNTRACKED_COUNT").parse().unwrap_or(0),
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        rustc_release: env!("RUSTY_PROV_RUSTC_RELEASE").to_string(),
        rustc_commit_hash: env!("RUSTY_PROV_RUSTC_COMMIT_HASH").to_string(),
        target: env!("RUSTY_PROV_TARGET").to_string(),
        profile: env!("RUSTY_PROV_PROFILE").to_string(),
        opt_level: env!("RUSTY_PROV_OPT_LEVEL").to_string(),
        prereg_path: option_env!("RUSTY_PROV_PREREG_PATH").map(str::to_string),
        prereg_sha256: option_env!("RUSTY_PROV_PREREG_SHA256").map(str::to_string),
    }
}

/// SHA-256 (lowercase hex) of a file, streamed; for runtime use.
pub fn hash_file(path: &Path) -> io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = sha256::Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize_hex())
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn json_opt(out: &mut String, s: &Option<String>) {
    match s {
        Some(v) => json_str(out, v),
        None => out.push_str("null"),
    }
}

impl Provenance {
    /// True only if git status is `Ok`, the SHA is 40 lowercase hex and the
    /// tracked tree was clean. A dirty or unknown build is NOT pinned.
    pub fn is_pinned(&self) -> bool {
        is_pinned_state(self.git_status, &self.git_sha, self.dirty)
    }

    /// Valid JSON with a stable key order.
    pub fn to_json(&self) -> String {
        let mut o = String::from("{");
        let key = |o: &mut String, k: &str, first: bool| {
            if !first {
                o.push(',');
            }
            json_str(o, k);
            o.push(':');
        };
        key(&mut o, "git_sha", true);
        json_str(&mut o, &self.git_sha);
        key(&mut o, "git_status", false);
        json_str(&mut o, self.git_status.as_str());
        key(&mut o, "dirty", false);
        o.push_str(if self.dirty { "true" } else { "false" });
        key(&mut o, "dirty_diff_sha256", false);
        json_opt(&mut o, &self.dirty_diff_sha256);
        key(&mut o, "untracked_count", false);
        let _ = write!(o, "{}", self.untracked_count);
        key(&mut o, "pinned", false);
        o.push_str(if self.is_pinned() { "true" } else { "false" });
        key(&mut o, "crate_version", false);
        json_str(&mut o, &self.crate_version);
        key(&mut o, "rustc_release", false);
        json_str(&mut o, &self.rustc_release);
        key(&mut o, "rustc_commit_hash", false);
        json_str(&mut o, &self.rustc_commit_hash);
        key(&mut o, "target", false);
        json_str(&mut o, &self.target);
        key(&mut o, "profile", false);
        json_str(&mut o, &self.profile);
        key(&mut o, "opt_level", false);
        json_str(&mut o, &self.opt_level);
        key(&mut o, "prereg_path", false);
        json_opt(&mut o, &self.prereg_path);
        key(&mut o, "prereg_sha256", false);
        json_opt(&mut o, &self.prereg_sha256);
        o.push('}');
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [&str; 14] = [
        "git_sha",
        "git_status",
        "dirty",
        "dirty_diff_sha256",
        "untracked_count",
        "pinned",
        "crate_version",
        "rustc_release",
        "rustc_commit_hash",
        "target",
        "profile",
        "opt_level",
        "prereg_path",
        "prereg_sha256",
    ];

    fn sample() -> Provenance {
        Provenance {
            git_sha: "0123456789abcdef0123456789abcdef01234567".into(),
            git_status: GitStatus::Ok,
            dirty: false,
            dirty_diff_sha256: None,
            untracked_count: 2,
            crate_version: "6.4.0".into(),
            rustc_release: "1.90.0".into(),
            rustc_commit_hash: "abc".into(),
            target: "x86_64-unknown-linux-gnu".into(),
            profile: "debug".into(),
            opt_level: "0".into(),
            prereg_path: None,
            prereg_sha256: None,
        }
    }

    #[test]
    fn to_json_parses_and_has_every_key() {
        let v: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        for k in KEYS {
            assert!(v.get(k).is_some(), "missing key {k}");
        }
        assert_eq!(v["untracked_count"], 2);
        assert_eq!(v["pinned"], true);
        assert!(v["prereg_sha256"].is_null());
    }

    #[test]
    fn build_info_json_roundtrips_and_matches_struct() {
        let p = build_info();
        let v: serde_json::Value = serde_json::from_str(&p.to_json()).unwrap();
        assert_eq!(v["git_sha"], p.git_sha.as_str());
        assert_eq!(v["pinned"], p.is_pinned());
        assert_eq!(v["crate_version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn json_escapes_quote_backslash_and_control() {
        let mut p = sample();
        p.prereg_path = Some("C:\\dir\\\"q\".md\n\u{1}".into());
        let v: serde_json::Value = serde_json::from_str(&p.to_json()).unwrap();
        assert_eq!(v["prereg_path"], "C:\\dir\\\"q\".md\n\u{1}");
    }

    #[test]
    fn dirty_or_unknown_is_not_pinned() {
        let mut p = sample();
        assert!(p.is_pinned());
        p.dirty = true;
        assert!(!p.is_pinned());
        p.dirty = false;
        p.git_status = GitStatus::NotARepo;
        assert!(!p.is_pinned());
        p.git_status = GitStatus::Ok;
        p.git_sha = "unknown".into();
        assert!(!p.is_pinned());
    }

    #[test]
    fn hash_file_matches_known_digest() {
        let path = std::env::temp_dir().join(format!("sundials-prov-hash-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        let h = hash_file(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(hash_file(Path::new("/nonexistent/definitely/missing")).is_err());
    }
}
