# sundials-provenance

Build-time provenance for rusty-SUNDIALS binaries and JSON outputs: call
`sundials_provenance::build_info()` and embed `to_json()` in your output.

## Recorded
- Git commit SHA (40 hex) of the checkout, plus a status: `ok`, `git_unavailable`, `not_a_repo`, `no_commits`.
- `dirty` flag, and SHA-256 of `git diff HEAD --binary` when tracked files differ from HEAD.
- Number of untracked files.
- `rustc` release and commit hash, target, profile, opt-level, crate version.
- Optional preregistration file: set `RUSTY_PREREG_FILE=/path/PREREG_ITEM.md` at build time to record its path and SHA-256 (the build fails if the file is unreadable).

## Guarantees
- Deterministic: no timestamps; same checkout and toolchain give the same strings.
- `is_pinned()` is true only if git worked, the SHA is 40 hex and tracked files are clean.
- The build re-runs when HEAD, the branch ref, the index or any tracked file changes.

## Not guaranteed
- Untracked files are counted, not hashed.
- Third-party dependency versions are not recorded (see `Cargo.lock`).
- The build host is not recorded.
- Downstream path dependencies: the build script runs in this crate's directory, so the values describe the rusty-SUNDIALS checkout, not the downstream crate.

Try: `cargo run -p sundials-provenance --example print_provenance -- --require-pinned`
