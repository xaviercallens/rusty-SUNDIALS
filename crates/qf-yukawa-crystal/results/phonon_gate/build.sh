#!/bin/bash
# release build of the lattice_phonon binary, cores 4-7, 2 jobs
export CARGO_TARGET_DIR=/home/callensxavier_gmail_com/rusty-target-ar
M=/home/callensxavier_gmail_com/rusty-SUNDIALS-wt-p2/Cargo.toml
taskset -c 4-7 cargo build --release -j 2 --offline --manifest-path "$M" -p qf-yukawa-crystal --bin lattice_phonon
