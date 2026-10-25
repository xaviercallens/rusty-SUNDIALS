#!/bin/bash
# Exact-potential re-relaxation of published seeds (erratum fix, 2026-10-25). Cores 4-7, 2 worker threads.
# Args of exact-relax: kappa nrun anneal(0/1) first.
set -e
# Private copy of the binary: the shared CARGO_TARGET_DIR is rebuilt by other lanes.
BIN=/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-ar-h2/target/h2bin/yukawa_crystal
CRATE=/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-ar-h2/crates/qf-yukawa-crystal
taskset -c 4-7 "$BIN" exact-relax "$CRATE" 2 4 0 1
taskset -c 4-7 "$BIN" exact-relax "$CRATE" 1 1 1 0
taskset -c 4-7 "$BIN" exact-saddle "$CRATE"
taskset -c 4-7 "$BIN" exact-relax "$CRATE" 1 2 0 1
