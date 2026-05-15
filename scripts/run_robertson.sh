#!/usr/bin/env bash
# =============================================================================
# run_robertson.sh — rusty-SUNDIALS v11 Robertson benchmark runner
# =============================================================================
# Builds and runs the Robertson chemical kinetics stiff ODE solver.
# Optionally writes machine-readable CSV output for CI validation and paper figures.
#
# Usage:
#   ./scripts/run_robertson.sh                       # table output to stdout
#   ./scripts/run_robertson.sh --csv results.csv     # CSV file output
#   ./scripts/run_robertson.sh --csv -               # CSV to stdout
#   ./scripts/run_robertson.sh --rtol 1e-6 --atol 1e-10 --csv bench.csv
#   ./scripts/run_robertson.sh --release              # optimized (default: debug)
#   ./scripts/run_robertson.sh --help
#
# Environment variables:
#   RUSTFLAGS        — passed through to cargo (e.g. "-C target-cpu=native")
#   CARGO_FLAGS      — extra flags for cargo build
#   SKIP_BUILD=1     — skip cargo build (use existing binary)
#
# Exit codes:
#   0 — success
#   1 — build failure
#   2 — solver runtime error
#   3 — CSV validation failure (conservation error > threshold)
# =============================================================================

set -euo pipefail

# ── Defaults ──────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CSV_PATH=""
RTOL="1e-4"
ATOL="1e-8"
PROFILE="debug"
VALIDATE=1
CONSERVATION_TOL="1e-12"

# ── Help ──────────────────────────────────────────────────────────────────────
usage() {
  grep '^#' "$0" | grep -v '^#!/' | sed 's/^# \?//'
  exit 0
}

# ── Parse arguments ───────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --csv)       CSV_PATH="$2"; shift 2 ;;
    --rtol)      RTOL="$2";     shift 2 ;;
    --atol)      ATOL="$2";     shift 2 ;;
    --release)   PROFILE="release"; shift ;;
    --no-validate) VALIDATE=0;  shift ;;
    --help|-h)   usage ;;
    *) echo "Unknown option: $1" >&2; usage ;;
  esac
done

# ── Colors ────────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info()  { echo -e "${GREEN}[robertson]${NC} $*"; }
warn()  { echo -e "${YELLOW}[robertson]${NC} $*"; }
error() { echo -e "${RED}[robertson]${NC} $*" >&2; }

cd "$REPO_ROOT"

# ── Build ─────────────────────────────────────────────────────────────────────
if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  info "Building robertson_csv example (profile=$PROFILE)..."
  CARGO_OPTS=""
  [[ "$PROFILE" == "release" ]] && CARGO_OPTS="--release"
  if ! cargo build --example robertson_csv $CARGO_OPTS ${CARGO_FLAGS:-} 2>&1; then
    error "cargo build failed"
    exit 1
  fi
  info "Build complete."
else
  warn "SKIP_BUILD=1 — using existing binary."
fi

# ── Binary path ───────────────────────────────────────────────────────────────
if [[ "$PROFILE" == "release" ]]; then
  BINARY="$REPO_ROOT/target/release/examples/robertson_csv"
else
  BINARY="$REPO_ROOT/target/debug/examples/robertson_csv"
fi

[[ -x "$BINARY" ]] || { error "Binary not found: $BINARY"; exit 1; }

# ── Build solver args ─────────────────────────────────────────────────────────
SOLVER_ARGS=("--rtol" "$RTOL" "--atol" "$ATOL")
[[ -n "$CSV_PATH" ]] && SOLVER_ARGS+=("--output-csv" "$CSV_PATH")

# ── Run ───────────────────────────────────────────────────────────────────────
info "Running Robertson benchmark (rtol=$RTOL, atol=$ATOL)..."
if ! "$BINARY" "${SOLVER_ARGS[@]}"; then
  error "Solver exited with non-zero status"
  exit 2
fi

# ── Validate CSV conservation error ──────────────────────────────────────────
if [[ "$VALIDATE" == "1" && -n "$CSV_PATH" && "$CSV_PATH" != "-" && -f "$CSV_PATH" ]]; then
  info "Validating conservation error (threshold: $CONSERVATION_TOL)..."
  FAILED=$(awk -F',' -v tol="$CONSERVATION_TOL" '
    NR==1 { next }  # skip header
    {
      # column 8 = conservation_error
      err = $8 + 0
      if (err < 0) err = -err
      if (err > tol) { print "FAIL t=" $1 " err=" err; exit 1 }
    }
  ' "$CSV_PATH") || {
    error "Conservation check FAILED: $FAILED"
    exit 3
  }
  ROWS=$(tail -n +2 "$CSV_PATH" | wc -l | tr -d ' ')
  info "Conservation check PASSED on $ROWS time points."
fi

# ── Summary ───────────────────────────────────────────────────────────────────
if [[ -n "$CSV_PATH" && "$CSV_PATH" != "-" ]]; then
  info "Results written to: $CSV_PATH"
fi
info "Robertson benchmark complete. ✓"
