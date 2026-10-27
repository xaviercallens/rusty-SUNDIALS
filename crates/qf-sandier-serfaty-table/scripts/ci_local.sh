#!/bin/bash
# Local run of the CI gates relevant to this crate: rustfmt check, clippy as CI runs it, and the crate's tests.
cd /home/callensxavier_gmail_com/rusty-SUNDIALS-wt-ss-table || exit 1
export CARGO_TARGET_DIR=/home/callensxavier_gmail_com/rusty-target-ar
echo "== fmt"; cargo fmt -p qf-sandier-serfaty-table -- --check; echo "fmt exit $?"
echo "== clippy"; cargo clippy -j 2 --workspace --features experimental -- -D warnings 2>&1 | tail -30; echo "clippy exit ${PIPESTATUS[0]}"
echo "== test"; cargo test -j 2 -p qf-sandier-serfaty-table 2>&1 | tail -30; echo "test exit ${PIPESTATUS[0]}"
