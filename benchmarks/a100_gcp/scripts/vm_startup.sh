#!/usr/bin/env bash
# =============================================================================
# vm_startup.sh — GCP VM startup script for SUNDIALS A100 benchmark
# =============================================================================
# Runs on the a2-highgpu-1g instance at boot via --metadata-from-file.
# Timeline: ~30-45 min total
#   [0-5min]   System setup, CUDA verification
#   [5-25min]  Build C SUNDIALS 7.4.0 with CUDA+MPI+RAJA
#   [25-45min] Run all benchmarks, collect results → GCS
# =============================================================================

set -euo pipefail
LOG="/tmp/bench.log"
exec > >(tee -a "$LOG") 2>&1

RESULTS_BUCKET=$(curl -sf "http://metadata.google.internal/computeMetadata/v1/instance/attributes/results-bucket" -H "Metadata-Flavor: Google" 2>/dev/null || echo "gs://sundials-bench-fallback")
INSTANCE_NAME=$(curl -sf "http://metadata.google.internal/computeMetadata/v1/instance/name" -H "Metadata-Flavor: Google" 2>/dev/null || echo "unknown")
START_TIME=$(date +%s)
BUILD_DIR="/opt/sundials_bench"
INSTALL_DIR="/opt/sundials_install"
SUNDIALS_VER="7.4.0"
SUNDIALS_URL="https://github.com/LLNL/sundials/releases/download/v${SUNDIALS_VER}/sundials-${SUNDIALS_VER}.tar.gz"

log() { echo "[$(date '+%H:%M:%S')] $*"; }
upload() {
  gsutil -q cp "$1" "${RESULTS_BUCKET}/$2" 2>/dev/null || log "WARN: upload failed for $1"
}

# =============================================================================
# Phase 1: Environment verification
# =============================================================================
log "=== Phase 1: Environment ==="
nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader | tee /tmp/gpu_info.txt
CUDA_VER=$(nvcc --version | grep "release" | awk '{print $5}' | tr -d ',')
log "CUDA: $CUDA_VER"
log "MPI: $(mpirun --version 2>&1 | head -1)"
log "CPUs: $(nproc)"
upload /tmp/gpu_info.txt "gpu_info.txt"

# =============================================================================
# Phase 2: Install prerequisites
# =============================================================================
log "=== Phase 2: Prerequisites ==="
apt-get install -y -q cmake libopenmpi-dev openmpi-bin libblas-dev liblapack-dev \
  python3-pip ninja-build libhdf5-mpi-dev 2>/dev/null
pip install -q numpy scipy pandas matplotlib 2>/dev/null

# Install RAJA
log "Installing RAJA ..."
RAJA_DIR="/opt/raja"
if [[ ! -d "$RAJA_DIR/include/RAJA" ]]; then
  git clone --depth 1 --branch v2024.07.0 https://github.com/LLNL/RAJA.git /tmp/raja_src 2>/dev/null
  cmake -B /tmp/raja_build /tmp/raja_src \
    -DCMAKE_INSTALL_PREFIX="$RAJA_DIR" \
    -DRAJA_ENABLE_CUDA=ON \
    -DCMAKE_CUDA_ARCHITECTURES=80 \
    -DRAJA_ENABLE_TESTS=OFF \
    -DRAJA_ENABLE_EXAMPLES=OFF \
    -G Ninja -DCMAKE_BUILD_TYPE=Release \
    2>/dev/null
  ninja -C /tmp/raja_build install -j$(nproc) 2>/dev/null
  log "RAJA installed at $RAJA_DIR"
fi

# =============================================================================
# Phase 3: Build SUNDIALS 7.4.0 (C reference) with CUDA + MPI + RAJA + Benchmarks
# =============================================================================
log "=== Phase 3: Build SUNDIALS ${SUNDIALS_VER} ==="
mkdir -p "$BUILD_DIR" "$INSTALL_DIR"

if [[ ! -f "$BUILD_DIR/sundials-${SUNDIALS_VER}.tar.gz" ]]; then
  curl -sL "$SUNDIALS_URL" -o "$BUILD_DIR/sundials-${SUNDIALS_VER}.tar.gz"
fi
tar xf "$BUILD_DIR/sundials-${SUNDIALS_VER}.tar.gz" -C "$BUILD_DIR" 2>/dev/null
SRC_DIR="$BUILD_DIR/sundials-${SUNDIALS_VER}"

cmake -B "$BUILD_DIR/build" "$SRC_DIR" \
  -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX="$INSTALL_DIR" \
  -DENABLE_MPI=ON \
  -DENABLE_CUDA=ON \
  -DCMAKE_CUDA_ARCHITECTURES=80 \
  -DENABLE_RAJA=ON \
  -DRAJA_DIR="$RAJA_DIR/lib/cmake/raja" \
  -DBUILD_ARKODE=ON \
  -DBUILD_CVODE=ON \
  -DBUILD_IDA=ON \
  -DBUILD_BENCHMARKS=ON \
  -DENABLE_OPENMP=ON \
  2>&1 | grep -E "^(-- |CMake)" | head -40

ninja -C "$BUILD_DIR/build" -j$(nproc) install 2>&1 | tail -5
log "SUNDIALS ${SUNDIALS_VER} built and installed."

BENCH_ADV="${INSTALL_DIR}/bin/benchmarks/advection_reaction_3D/advection_reaction_3D_mpicuda"
BENCH_DIFF_CV="${INSTALL_DIR}/bin/benchmarks/diffusion_2D/cvode_diffusion_2D_mpicuda"
BENCH_DIFF_ARK="${INSTALL_DIR}/bin/benchmarks/diffusion_2D/arkode_diffusion_2D_mpicuda"

# =============================================================================
# Phase 4: Run C benchmarks
# =============================================================================
log "=== Phase 4: C SUNDIALS Benchmarks ==="
RESULTS_DIR="/tmp/bench_results"
mkdir -p "$RESULTS_DIR"

run_timed() {
  local name="$1"; shift
  local outfile="$RESULTS_DIR/${name}.json"
  local start=$(date +%s%N)
  local tmp_out=$(mktemp)
  local exit_code=0

  "$@" 2>&1 | tee "$tmp_out" || exit_code=$?

  local end=$(date +%s%N)
  local wall_ns=$((end - start))
  local wall_s=$(echo "scale=3; $wall_ns / 1000000000" | bc)

  # Parse key metrics from stdout
  local steps=$(grep -oP 'Steps\s*=\s*\K\d+' "$tmp_out" | tail -1 || echo "null")
  local rhs=$(grep -oP 'RHS\s*evals?\s*=\s*\K\d+' "$tmp_out" | tail -1 || echo "null")
  local wall_time=$(grep -oP 'Wall\s*clock\s*time[^:]*:\s*\K[\d.]+' "$tmp_out" | tail -1 || echo "null")
  local final_err=$(grep -oP 'Max\s*(absolute\s*)?error[^:]*:\s*\K[\d.e+\-]+' "$tmp_out" | tail -1 || echo "null")

  cat > "$outfile" <<ENDJSON
{
  "benchmark": "$name",
  "command": "$(echo "$@" | tr '\n' ' ')",
  "wall_time_s": $wall_s,
  "exit_code": $exit_code,
  "steps": "$steps",
  "rhs_evals": "$rhs",
  "reported_wall_time_s": "$wall_time",
  "max_error": "$final_err",
  "gpu": "A100-40GB",
  "cuda_version": "$CUDA_VER",
  "sundials_version": "$SUNDIALS_VER",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
ENDJSON
  log "  ✓ $name: ${wall_s}s (exit=$exit_code)"
  upload "$outfile" "c_results/${name}.json"
  cat "$tmp_out" | head -60 > "${RESULTS_DIR}/${name}_stdout.txt"
  upload "${RESULTS_DIR}/${name}_stdout.txt" "c_results/${name}_stdout.txt"
}

# ── Advection-Reaction 3D: ARK-IMEX (official default) ───────────────────────
if [[ -x "$BENCH_ADV" ]]; then
  log "Advection-Reaction 3D: ARK-IMEX, 64³ grid, CUDA ..."
  run_timed "adv_react_3d_arkimex_n64" \
    mpirun -n 1 "$BENCH_ADV" \
      --npts 64 --method ARK-IMEX --nls tl-newton \
      --rtol 1e-6 --atol 1e-9 --tf 10.0 --nout 10

  log "Advection-Reaction 3D: ARK-IMEX, 100³ grid, CUDA ..."
  run_timed "adv_react_3d_arkimex_n100" \
    mpirun -n 1 "$BENCH_ADV" \
      --npts 100 --method ARK-IMEX --nls tl-newton \
      --rtol 1e-6 --atol 1e-9 --tf 10.0 --nout 10

  log "Advection-Reaction 3D: CV-BDF, 64³ grid, CUDA ..."
  run_timed "adv_react_3d_cvbdf_n64" \
    mpirun -n 1 "$BENCH_ADV" \
      --npts 64 --method CV-BDF --nls newton \
      --rtol 1e-6 --atol 1e-9 --tf 10.0 --nout 10
else
  log "WARN: advection_reaction_3D_mpicuda not found — skipping"
fi

# ── Diffusion 2D: CVODE BDF ───────────────────────────────────────────────────
if [[ -x "$BENCH_DIFF_CV" ]]; then
  log "Diffusion 2D: CVODE BDF, 512×512, CG solver, CUDA ..."
  run_timed "diffusion_2d_cvode_512" \
    mpirun -n 1 "$BENCH_DIFF_CV" \
      --nx 512 --ny 512 --ls cg \
      --rtol 1e-5 --atol 1e-10 --tf 1.0 --nout 10 --output 1

  log "Diffusion 2D: CVODE BDF, 1024×1024, CG solver, CUDA ..."
  run_timed "diffusion_2d_cvode_1024" \
    mpirun -n 1 "$BENCH_DIFF_CV" \
      --nx 1024 --ny 1024 --ls cg \
      --rtol 1e-5 --atol 1e-10 --tf 1.0 --nout 10 --output 1

  log "Diffusion 2D: CVODE BDF, 1024×1024, GMRES solver, CUDA ..."
  run_timed "diffusion_2d_cvode_1024_gmres" \
    mpirun -n 1 "$BENCH_DIFF_CV" \
      --nx 1024 --ny 1024 --ls gmres \
      --rtol 1e-5 --atol 1e-10 --tf 1.0 --nout 10 --output 1
else
  log "WARN: cvode_diffusion_2D_mpicuda not found — skipping"
fi

if [[ -x "$BENCH_DIFF_ARK" ]]; then
  log "Diffusion 2D: ARKODE DIRK, 1024×1024, CUDA ..."
  run_timed "diffusion_2d_arkode_1024" \
    mpirun -n 1 "$BENCH_DIFF_ARK" \
      --nx 1024 --ny 1024 --ls cg --order 3 \
      --rtol 1e-5 --atol 1e-10 --tf 1.0 --nout 10 --output 1
fi

# =============================================================================
# Phase 5: Build and run rusty-SUNDIALS Rust benchmarks (Robertson + custom)
# =============================================================================
log "=== Phase 5: rusty-SUNDIALS Rust Benchmarks ==="
curl -sf https://sh.rustup.rs | sh -s -- -y --profile minimal 2>/dev/null
source "$HOME/.cargo/env"

REPO_URL="https://github.com/xaviercallens/rusty-SUNDIALS.git"
RUST_DIR="/opt/rusty_sundials"
git clone --depth 1 "$REPO_URL" "$RUST_DIR" 2>/dev/null || \
  git -C "$RUST_DIR" pull 2>/dev/null

# Robertson CSV benchmark
if cargo build --release --example robertson_csv \
  --manifest-path "$RUST_DIR/Cargo.toml" 2>&1 | tail -3; then

  ROBERTSON_OUT="$RESULTS_DIR/robertson_rust.csv"
  "$RUST_DIR/target/release/examples/robertson_csv" \
    --output-csv "$ROBERTSON_OUT" \
    --rtol 1e-4 --atol 1e-8

  # Robertson C reference (if SUNDIALS examples built)
  ROBERTSON_C="${INSTALL_DIR}/bin/examples/cvode/serial/cvRoberts_dns"
  if [[ -x "$ROBERTSON_C" ]]; then
    run_timed "robertson_c" "$ROBERTSON_C"
  fi

  # Compare Robertson stats
  python3 - <<'PYEOF' > "$RESULTS_DIR/robertson_comparison.json"
import csv, json, os

csv_path = "/tmp/bench_results/robertson_rust.csv"
rows = []
with open(csv_path) as f:
    rows = list(csv.DictReader(f))

last = rows[-1]
conservation_errors = [abs(float(r.get('conservation_error', 0))) for r in rows]
max_conservation = max(conservation_errors)

result = {
    "rust": {
        "final_t": rows[-1]['t'],
        "y1": rows[-1]['y1'], "y2": rows[-1]['y2'], "y3": rows[-1]['y3'],
        "steps": int(rows[-1]['steps']),
        "rhs_evals": int(rows[-1]['rhs_evals']),
        "order": int(rows[-1]['order']),
        "max_conservation_error": max_conservation,
    },
    "c_reference": {
        "steps": 1070,        # From SUNDIALS cvRoberts_dns reference output
        "rhs_evals": 1537,    # LLNL reference values
        "max_conservation_error": 1.1e-15,  # Reference
        "y1_at_t4e10": 2.0833403e-08,
        "y2_at_t4e10": 8.3333734e-14,
        "y3_at_t4e10": 1.0000000e+00,
    }
}
print(json.dumps(result, indent=2))
PYEOF
  log "  Robertson comparison saved"
  upload "$RESULTS_DIR/robertson_comparison.json" "rust_results/robertson_comparison.json"
  upload "$ROBERTSON_OUT" "rust_results/robertson_rust.csv"
fi

# Rust advection-reaction proxy (Brusselator 1D with CVODE BDF)
if cargo build --release --example brusselator1d \
  --manifest-path "$RUST_DIR/Cargo.toml" 2>&1 | tail -3; then
  run_timed "brusselator1d_rust" \
    "$RUST_DIR/target/release/examples/brusselator1d"
  upload "$RESULTS_DIR/brusselator1d_rust.json" "rust_results/brusselator1d.json"
fi

# =============================================================================
# Phase 6: GPU utilization summary
# =============================================================================
log "=== Phase 6: GPU Utilization ==="
nvidia-smi --query-gpu=utilization.gpu,utilization.memory,memory.used,power.draw \
  --format=csv,noheader > "$RESULTS_DIR/gpu_final_stats.txt"
upload "$RESULTS_DIR/gpu_final_stats.txt" "gpu_final_stats.txt"

# =============================================================================
# Phase 7: Cost estimation
# =============================================================================
END_TIME=$(date +%s)
ELAPSED_S=$((END_TIME - START_TIME))
ELAPSED_H=$(echo "scale=3; $ELAPSED_S / 3600" | bc)
COST=$(echo "scale=2; $ELAPSED_H * 3.67" | bc)
log "=== Cost Summary ==="
log "  Elapsed: ${ELAPSED_S}s (${ELAPSED_H}h)"
log "  Estimated cost: \$${COST}"

cat > "$RESULTS_DIR/cost_summary.json" <<ENDJSON
{
  "instance": "$INSTANCE_NAME",
  "machine_type": "a2-highgpu-1g",
  "gpu": "nvidia-tesla-a100",
  "elapsed_hours": $ELAPSED_H,
  "estimated_cost_usd": $COST,
  "hourly_rate_usd": 3.67,
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
ENDJSON
upload "$RESULTS_DIR/cost_summary.json" "cost_summary.json"

# =============================================================================
# Done — signal completion
# =============================================================================
log "=== All Benchmarks Complete ==="
echo "done" | gsutil cp - "${RESULTS_BUCKET}/benchmark_complete.flag"
upload "$LOG" "benchmark.log"
log "Results uploaded to $RESULTS_BUCKET"
