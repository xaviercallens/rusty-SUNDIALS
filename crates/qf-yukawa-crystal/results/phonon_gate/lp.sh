#!/bin/bash
# run the lattice_phonon release binary on cores 4-5 (2 threads max); args are passed through
exec taskset -c 4-5 /home/callensxavier_gmail_com/rusty-target-ar/release/lattice_phonon "$@"
