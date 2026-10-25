#!/bin/bash
# fmt check, clippy and unit tests for this crate, pinned to cores 4-7 with 2 jobs/threads.
set -e
M=/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-ar-h2/Cargo.toml
export CARGO_TARGET_DIR=/home/callensxavier_gmail_com/rusty-target-ar
cargo fmt --manifest-path "$M" -p qf-yukawa-crystal --check
taskset -c 4-7 cargo clippy -j 2 --offline --release --manifest-path "$M" -p qf-yukawa-crystal 2>&1 | grep -E "^(warning|error)" | sort | uniq -c || true
taskset -c 4-7 cargo test -j 2 --offline --release --manifest-path "$M" -p qf-yukawa-crystal -- --test-threads=2
