#!/bin/bash
# Build (release, -j 2, shared target dir) and run the CVODE cross-check; report to results/.
cd /home/callensxavier_gmail_com/rusty-SUNDIALS-wt-ss-table || exit 1
export CARGO_TARGET_DIR=/home/callensxavier_gmail_com/rusty-target-ar
cargo build --release -j 2 -p qf-sandier-serfaty-table 2>&1 | tail -20
mkdir -p crates/qf-sandier-serfaty-table/results
"$CARGO_TARGET_DIR/release/ss_table_cvode" "$@" > crates/qf-sandier-serfaty-table/results/cross_check.json 2> crates/qf-sandier-serfaty-table/results/cross_check.log
echo "exit $?"
