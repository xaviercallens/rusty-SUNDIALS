#!/bin/bash
# clippy as CI runs it (workspace, experimental features, -D warnings), 2 jobs
export CARGO_TARGET_DIR=/home/callensxavier_gmail_com/rusty-target-ar
M=/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-p2/Cargo.toml
taskset -c 6-7 cargo clippy -j 2 --offline --manifest-path "$M" --workspace --features experimental -- -D warnings 2>&1 | tail -60
