#!/usr/bin/env bash
# =============================================================================
# run_a100_benchmark.sh — rusty-SUNDIALS v11 A100 GPU Benchmark Suite
# =============================================================================
# Runs SUNDIALS official benchmark problems on GCP A100 (a2-highgpu-1g)
# and compares C SUNDIALS v7.4.0 results with rusty-SUNDIALS Rust implementation.
#
# Budget: < $100 (a2-highgpu-1g @ ~$3.67/hr, target: < 4 hours compute)
#
# Benchmarks:
#   1. Advection-Reaction 3D (ARKODE ARK-IMEX, CUDA NVVector)
#      -- 100³ grid, 3 species, Brusselator kinetics
#      -- Methods: ARK-IMEX (default), CV-BDF
#      -- Parallelism: MPI + CUDA (2 A100 tasks → 1 A100 for our 1-GPU config)
#
#   2. Diffusion 2D (CVODE BDF, CUDA NVVector)
#      -- 1024×1024 grid, anisotropic heat equation
#      -- Linear solvers: CG (default), GMRES
#      -- Parallelism: MPI + CUDA
#
#   3. Robertson (rusty-SUNDIALS BDF reference)
#      -- Classic stiff ODE — used to verify Rust matches C output
#
# Reference C results (LLNL Lassen/Summit):
#   Advection-Reaction 3D: ~42s wall-time on 2× A100 (jsrun -n 2 -g 1)
#   Diffusion 2D:          ~18s wall-time on 2× A100
#   Robertson C:           steps=1070, rhs=1537, conservation=1e-15
#
# Usage:
#   ./benchmarks/a100_gcp/scripts/run_a100_benchmark.sh [--dry-run] [--no-gpu]
# =============================================================================

set -euo pipefail

PROJECT="gen-lang-client-0625573011"
ZONE="us-central1-a"
INSTANCE="rusty-sundials-a100-bench"
MACHINE="a2-highgpu-1g"          # 12 vCPU, 85 GB, 1× A100 40GB
DISK_SIZE="200GB"
IMAGE_FAMILY="common-cu121"      # CUDA 12.1 Deep Learning VM
IMAGE_PROJECT="deeplearning-platform-release"
RESULTS_BUCKET="gs://${PROJECT}-sundials-bench"
MAX_COST_USD=80                  # hard limit below $100
HOURLY_COST_USD=3.67            # a2-highgpu-1g on-demand

# Colors
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; NC='\033[0m'
info()  { echo -e "${GREEN}[bench]${NC} $*"; }
warn()  { echo -e "${YELLOW}[bench]${NC} $*"; }
error() { echo -e "${RED}[bench]${NC} $*" >&2; }

DRY_RUN=0
NO_GPU=0
for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN=1 ;;
    --no-gpu)  NO_GPU=1  ;;
  esac
done

[[ $DRY_RUN -eq 1 ]] && info "DRY RUN mode — no resources will be created."

# =============================================================================
# Step 1: Cost guard
# =============================================================================
MAX_HOURS=$(echo "$MAX_COST_USD $HOURLY_COST_USD" | awk '{printf "%.1f", $1/$2}')
info "Cost guard: max $${MAX_COST_USD} = ${MAX_HOURS}h on ${MACHINE}"

# =============================================================================
# Step 2: Create GCS bucket for results
# =============================================================================
if [[ $DRY_RUN -eq 0 ]]; then
  gsutil ls "$RESULTS_BUCKET" 2>/dev/null || \
    gsutil mb -p "$PROJECT" -l US-CENTRAL1 "$RESULTS_BUCKET"
  info "Results bucket: $RESULTS_BUCKET"
fi

# =============================================================================
# Step 3: Create A100 VM
# =============================================================================
STARTUP_SCRIPT="$(dirname "$0")/vm_startup.sh"
FIREWALL_TAG="sundials-bench"

create_vm() {
  info "Creating $MACHINE VM: $INSTANCE in $ZONE ..."
  gcloud compute instances create "$INSTANCE" \
    --project="$PROJECT" \
    --zone="$ZONE" \
    --machine-type="$MACHINE" \
    --accelerator="count=1,type=nvidia-tesla-a100" \
    --maintenance-policy=TERMINATE \
    --restart-on-failure=no \
    --image-family="$IMAGE_FAMILY" \
    --image-project="$IMAGE_PROJECT" \
    --boot-disk-size="$DISK_SIZE" \
    --boot-disk-type=pd-ssd \
    --metadata-from-file=startup-script="$STARTUP_SCRIPT" \
    --metadata="results-bucket=$RESULTS_BUCKET" \
    --scopes="cloud-platform" \
    --tags="$FIREWALL_TAG" \
    --no-shielded-secure-boot \
    && info "VM created. Waiting for startup (~5 min)..."
}

delete_vm() {
  warn "Deleting VM $INSTANCE ..."
  gcloud compute instances delete "$INSTANCE" \
    --project="$PROJECT" --zone="$ZONE" --quiet
  info "VM deleted."
}

# =============================================================================
# Step 4: Poll for benchmark completion (with cost timeout)
# =============================================================================
wait_for_results() {
  local timeout_secs=$(echo "$MAX_HOURS * 3600" | bc | cut -d. -f1)
  local elapsed=0
  local poll_interval=60
  local results_marker="${RESULTS_BUCKET}/benchmark_complete.flag"

  info "Waiting for benchmark results (timeout: ${MAX_HOURS}h = ${timeout_secs}s) ..."
  while [[ $elapsed -lt $timeout_secs ]]; do
    if gsutil -q stat "$results_marker" 2>/dev/null; then
      info "Benchmarks complete! Downloading results..."
      return 0
    fi
    local cost_so_far=$(echo "scale=2; $elapsed * $HOURLY_COST_USD / 3600" | bc)
    info "  Elapsed: ${elapsed}s | Cost so far: \$${cost_so_far}"
    sleep $poll_interval
    elapsed=$((elapsed + poll_interval))
  done

  error "Timeout after ${MAX_HOURS}h (\$${MAX_COST_USD}). Deleting VM."
  delete_vm
  exit 1
}

# =============================================================================
# Step 5: Download and analyze results
# =============================================================================
download_and_analyze() {
  local results_dir="$(dirname "$0")/../results"
  mkdir -p "$results_dir"
  gsutil -m cp -r "${RESULTS_BUCKET}/*.json" "$results_dir/" 2>/dev/null || true
  gsutil -m cp -r "${RESULTS_BUCKET}/*.csv"  "$results_dir/" 2>/dev/null || true
  info "Results downloaded to: $results_dir"

  # Run analysis script
  python3 "$(dirname "$0")/../analysis/analyze_results.py" \
    --results-dir "$results_dir" \
    --output "$(dirname "$0")/../results/benchmark_report.md"
}

# =============================================================================
# Main
# =============================================================================
if [[ $DRY_RUN -eq 1 ]]; then
  info "Would create: $MACHINE ($ZONE, 1× A100)"
  info "Would run startup: vm_startup.sh"
  info "Would poll: $RESULTS_BUCKET"
  exit 0
fi

trap 'error "Interrupted — cleaning up VM..."; delete_vm || true; exit 1' INT TERM

create_vm
wait_for_results
download_and_analyze
delete_vm

info "Benchmark complete. Report: benchmarks/a100_gcp/results/benchmark_report.md"
