#!/bin/bash
# unit tests of the phonon module, pinned to cores 4-7, 2 jobs/threads
export CARGO_TARGET_DIR=/home/callensxavier_gmail_com/rusty-target-ar
M=/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-p2/Cargo.toml
taskset -c 4-7 cargo test --release -j 2 --offline --manifest-path "$M" -p qf-yukawa-crystal --lib "$@" -- --test-threads=2
