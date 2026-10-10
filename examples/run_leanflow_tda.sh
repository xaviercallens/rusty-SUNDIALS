#!/bin/bash
set -e

echo "=========================================================="
echo " LeanFlow TDA Orchestration Pipeline"
echo "=========================================================="

cd /home/xavkal/xdev/rusty-SUNDIALS

echo "[1/3] Generating Cosmological Trajectory (rusty-SUNDIALS)..."
cargo run --example cosmology_quintessence || true
echo "Telemetry exported to cosmology_quintessence.csv"

echo "[2/3] Activating Python TDA Environment..."
cd examples
source tda_env/bin/activate

echo "[3/3] Running GUDHI TDA Analysis..."
# We analyze the phase space shape using x, y, rho, p
python tda_analyzer.py --input ../cosmology_quintessence.csv --columns "x,y,rho,p" --max-edge 1.5 --max-dim 2 --output-dir tda_output --prefix cosmology_fricke

echo "=========================================================="
echo " Pipeline Complete. Topological signatures saved in:"
echo " /home/xavkal/xdev/rusty-SUNDIALS/examples/tda_output/"
echo "=========================================================="
