//! Git state collection, shared by `build.rs` and the library through `#[path]`.
//!
//! Limits, stated plainly: tracked-file changes are hashed (through the bytes of
//! `git diff HEAD --binary`); untracked files are only COUNTED, never hashed, so
//! two trees that differ only in untracked file contents look identical here.
#![allow(dead_code)]

use crate::sha256;
use std::path::Path;
use std::process::Command;

/// Outcome of querying git.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitStatus {
    /// Git answered and HEAD resolved to a commit.
    Ok,
    /// The `git` binary could not be run.
    GitUnavailable,
    /// The directory is not inside a git work tree (e.g. a crates.io tarball).
    NotARepo,
    /// A repository exists but has no commit yet.
    NoCommits,
}

impl GitStatus {
    /// Stable machine-readable name.
    pub fn as_str(self) -> &'static str {
        match self {
            GitStatus::Ok => "ok",
            GitStatus::GitUnavailable => "git_unavailable",
            GitStatus::NotARepo => "not_a_repo",
            GitStatus::NoCommits => "no_commits",
        }
    }

    /// Inverse of [`GitStatus::as_str`]; unknown names map to `GitUnavailable`.
    pub fn parse(s: &str) -> GitStatus {
        match s {
            "ok" => GitStatus::Ok,
            "not_a_repo" => GitStatus::NotARepo,
            "no_commits" => GitStatus::NoCommits,
            _ => GitStatus::GitUnavailable,
        }
    }
}

/// Snapshot of a git checkout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitInfo {
    /// 40 lowercase hex characters, or `"unknown"`.
    pub sha: String,
    /// Whether tracked files differ from HEAD (index or work tree).
    pub dirty: bool,
    /// SHA-256 of `git diff HEAD --binary` when dirty; `None` if clean or git failed.
    pub dirty_diff_sha256: Option<String>,
    /// Number of untracked, non-ignored paths (counted, NOT hashed).
    pub untracked_count: usize,
    /// Whether the values above can be trusted.
    pub status: GitStatus,
}

/// True only when git worked, the SHA is a full 40-hex id, and the tree is clean.
pub fn is_pinned_state(status: GitStatus, sha: &str, dirty: bool) -> bool {
    status == GitStatus::Ok
        && !dirty
        && sha.len() == 40
        && sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl GitInfo {
    /// See [`is_pinned_state`].
    pub fn is_pinned(&self) -> bool {
        is_pinned_state(self.status, &self.sha, self.dirty)
    }

    fn failed(status: GitStatus) -> GitInfo {
        GitInfo {
            sha: "unknown".to_string(),
            dirty: false,
            dirty_diff_sha256: None,
            untracked_count: 0,
            status,
        }
    }
}

/// Run `git args...` in `dir`. `Err(true)` = binary missing, `Err(false)` = nonzero exit.
fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>, bool> {
    match Command::new("git").args(args).current_dir(dir).output() {
        Ok(o) if o.status.success() => Ok(o.stdout),
        Ok(_) => Err(false),
        Err(_) => Err(true),
    }
}

/// Arguments for the dirty-tree diff whose SHA-256 is recorded. Every option that can change what `git diff`
/// prints is pinned here (flags, plus `-c` overrides that older git versions simply ignore), so the same
/// tracked change hashes the same under different user or repository diff settings (external diff tools,
/// prefixes, colour, rename detection, algorithm, context size, relative paths, text conversion).
/// `core.autocrlf` and `core.eol` are NOT overridden: they change what a working-tree file means and stay part of
/// the repository state.
const DIFF_ARGS: &[&str] = &[
    "-c",
    "core.quotepath=off",
    "-c",
    "color.ui=false",
    "-c",
    "diff.relative=false",
    "-c",
    "diff.noprefix=false",
    "-c",
    "diff.mnemonicPrefix=false",
    "-c",
    "diff.renames=false",
    "-c",
    "diff.algorithm=myers",
    "-c",
    "diff.context=3",
    "-c",
    "diff.interHunkContext=0",
    "diff",
    "HEAD",
    "--binary",
    "--full-index",
    "--no-ext-diff",
    "--no-textconv",
    "--no-color",
    "--no-renames",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

/// Collect git state for the checkout containing `dir`.
pub fn collect(dir: &Path) -> GitInfo {
    match git(dir, &["rev-parse", "--git-dir"]) {
        Ok(_) => {}
        Err(true) => return GitInfo::failed(GitStatus::GitUnavailable),
        Err(false) => return GitInfo::failed(GitStatus::NotARepo),
    }
    let sha = match git(dir, &["rev-parse", "HEAD"]) {
        Ok(out) => String::from_utf8_lossy(&out).trim().to_string(),
        Err(true) => return GitInfo::failed(GitStatus::GitUnavailable),
        Err(false) => return GitInfo::failed(GitStatus::NoCommits),
    };
    let porcelain = match git(dir, &["status", "--porcelain"]) {
        Ok(out) => out,
        Err(_) => return GitInfo::failed(GitStatus::GitUnavailable),
    };
    let mut dirty = false;
    let mut untracked_count = 0usize;
    for line in porcelain.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
        if line.starts_with(b"??") {
            untracked_count += 1;
        } else {
            dirty = true;
        }
    }
    let dirty_diff_sha256 = if dirty {
        match git(dir, DIFF_ARGS) {
            Ok(diff) => Some(sha256::hex_digest(&diff)),
            Err(_) => return GitInfo::failed(GitStatus::GitUnavailable),
        }
    } else {
        None
    };
    GitInfo {
        sha,
        dirty,
        dirty_diff_sha256,
        untracked_count,
        status: GitStatus::Ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn git_present() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!(
            "sundials-prov-{}-{}-{}",
            tag,
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn run(dir: &Path, args: &[&str]) {
        let st = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            st.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&st.stderr)
        );
    }

    fn committed_repo(tag: &str) -> PathBuf {
        let d = temp_dir(tag);
        run(&d, &["init", "-q"]);
        run(&d, &["config", "user.email", "t@example.invalid"]);
        run(&d, &["config", "user.name", "t"]);
        run(&d, &["config", "commit.gpgsign", "false"]);
        fs::write(d.join("f.txt"), "one\n").unwrap();
        run(&d, &["add", "f.txt"]);
        run(&d, &["commit", "-q", "-m", "init"]);
        d
    }

    macro_rules! need_git {
        () => {
            if !git_present() {
                eprintln!("SKIPPED: git binary not found");
                return;
            }
        };
    }

    #[test]
    fn clean_repo_has_sha_and_is_not_dirty() {
        need_git!();
        let d = committed_repo("clean");
        let g = collect(&d);
        assert_eq!(g.status, GitStatus::Ok);
        assert_eq!(g.sha.len(), 40);
        assert!(g.sha.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(!g.dirty);
        assert_eq!(g.dirty_diff_sha256, None);
        assert!(g.is_pinned());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn dirty_hash_changes_with_content_and_untracked_only_counted() {
        need_git!();
        let d = committed_repo("dirty");
        fs::write(d.join("f.txt"), "two\n").unwrap();
        let a = collect(&d);
        assert!(a.dirty);
        let ha = a.dirty_diff_sha256.clone().expect("hash when dirty");
        assert_eq!(ha.len(), 64);
        fs::write(d.join("f.txt"), "three\n").unwrap();
        let b = collect(&d);
        assert!(b.dirty);
        assert_ne!(Some(ha), b.dirty_diff_sha256);
        // Untracked files: counted, and do not change the dirty flag/hash.
        fs::write(d.join("new.txt"), "x").unwrap();
        let c = collect(&d);
        assert_eq!(c.untracked_count, 1);
        assert_eq!(c.dirty_diff_sha256, b.dirty_diff_sha256);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn dirty_hash_does_not_depend_on_diff_configuration() {
        need_git!();
        let d = committed_repo("cfg");
        fs::write(d.join("f.txt"), "one\ntwo\nthree\n").unwrap();
        let before = collect(&d);
        assert!(before.dirty, "setup: tree must be dirty");
        let h = before.dirty_diff_sha256.clone().expect("hash when dirty");
        // Every one of these settings changes what a plain `git diff` prints, or makes it fail outright
        // (diff.external pointing at a program that does not exist).
        for (k, v) in [
            ("diff.noprefix", "true"),
            ("diff.mnemonicPrefix", "true"),
            ("diff.algorithm", "histogram"),
            ("diff.context", "9"),
            ("diff.renames", "copies"),
            ("diff.external", "/nonexistent/diff-program"),
            ("color.ui", "always"),
            ("core.quotepath", "true"),
        ] {
            run(&d, &["config", k, v]);
        }
        let after = collect(&d);
        assert_eq!(
            after.status,
            GitStatus::Ok,
            "collect must not fail under odd diff settings"
        );
        assert_eq!(
            after.dirty_diff_sha256,
            Some(h),
            "dirty-diff hash must ignore diff settings"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn non_repo_reports_not_a_repo() {
        need_git!();
        let d = temp_dir("norepo");
        let g = collect(&d);
        // The temp dir could itself live inside a repo on exotic setups; skip then.
        if g.status == GitStatus::Ok {
            eprintln!("SKIPPED: temp dir is inside a git work tree");
            return;
        }
        assert_eq!(g.status, GitStatus::NotARepo);
        assert_eq!(g.sha, "unknown");
        assert!(!g.is_pinned());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn negative_control_detector_can_fail() {
        need_git!();
        let d = committed_repo("negctl");
        assert!(
            collect(&d).is_pinned(),
            "clean committed repo must be pinned"
        );
        fs::write(d.join("f.txt"), "tampered\n").unwrap();
        assert!(!collect(&d).is_pinned(), "dirty repo must NOT be pinned");
        run(&d, &["commit", "-q", "-a", "-m", "clean again"]);
        assert!(
            collect(&d).is_pinned(),
            "after clean commit must be pinned again"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn pinned_state_logic_rejects_bad_inputs() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert!(is_pinned_state(GitStatus::Ok, sha, false));
        assert!(!is_pinned_state(GitStatus::Ok, sha, true));
        assert!(!is_pinned_state(GitStatus::NotARepo, sha, false));
        assert!(!is_pinned_state(GitStatus::Ok, "unknown", false));
        assert!(!is_pinned_state(GitStatus::Ok, "ABCDEF", false));
    }
}
