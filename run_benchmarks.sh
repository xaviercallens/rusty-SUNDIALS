#!/bin/bash
# Rusty-SUNDIALS Scientific Benchmark Suite
# Runs all 10 use cases with wall-clock timing on Apple Silicon
set -euo pipefail

BIN=/Volumes/ramdisk/target/release/examples
TIMEOUT=30  # seconds per example

echo "================================================================"
echo "  Rusty-SUNDIALS Scientific Benchmark Suite"
echo "  Platform: $(sysctl -n machdep.cpu.brand_string), $(sysctl -n hw.ncpu) cores"
echo "  RAM: $(sysctl -n hw.memsize | awk '{printf "%.0f GB", $1/1073741824}')"
echo "  Rust: $(rustc --version)"
echo "  Build: release + target-cpu=native (NEON SIMD)"
echo "  Date: $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
echo "================================================================"
echo ""

RESULTS=()

run_bench() {
    local name="$1"
    local bin="$BIN/$2"
    local lines="${3:-10}"  # lines of output to show

    echo "──────────────────────────────────────────────────────────────"
    echo "▶ $name"
    echo "──────────────────────────────────────────────────────────────"

    local start_ns=$(python3 -c "import time; print(int(time.time()*1e9))")

    # Run with a timeout, capture output
    local output
    local exit_code=0
    output=$(perl -e 'alarm shift; exec @ARGV' "$TIMEOUT" "$bin" 2>&1) || exit_code=$?

    local end_ns=$(python3 -c "import time; print(int(time.time()*1e9))")
    local elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))

    if [ $exit_code -eq 142 ] || [ $exit_code -eq 137 ]; then
        echo "  ⏱ TIMEOUT after ${TIMEOUT}s"
        echo "$output" | head -5
        # Show last lines for partial results
        echo "  ..."
        echo "$output" | tail -3
        RESULTS+=("$name|TIMEOUT (>${TIMEOUT}s)|—")
    elif [ $exit_code -ne 0 ]; then
        echo "$output" | head -"$lines"
        RESULTS+=("$name|${elapsed_ms}ms|error:$exit_code")
    else
        echo "$output" | head -"$lines"
        # Extract steps if present
        local steps=$(echo "$output" | grep -i "steps:" | tail -1 || true)
        echo "  ..."
        echo "$output" | tail -3
        RESULTS+=("$name|${elapsed_ms}ms|$steps")
    fi
    echo "  ⏱ Wall time: ${elapsed_ms} ms"
    echo ""
}

# Run all 10 scientific benchmarks
run_bench "1. Lorenz Attractor (chaos, 3 ODEs)"            lorenz         8
run_bench "2. Hodgkin-Huxley Neuron (stiff, 4 ODEs)"       hodgkin_huxley 8
run_bench "3. SIR Epidemic (epidemiology, 3 ODEs)"         sir_epidemic   8
run_bench "4. Lotka-Volterra (ecology, 2 ODEs)"            lotka_volterra 8
run_bench "5. HIRES Photochemistry (stiff, 8 ODEs)"        hires          8
run_bench "6. Double Pendulum (chaotic, 4 ODEs)"           double_pendulum 8
run_bench "7. Rigid Body Euler (conservation, 3 ODEs)"     rigid_body     8
run_bench "8. Rössler Attractor (chaos, 3 ODEs)"           rossler        8
run_bench "9. FitzHugh-Nagumo (neuron, 2 ODEs)"            fitzhugh_nagumo 8
run_bench "10. Three-Body Problem (celestial, 12 ODEs)"    three_body     8

# Also run the N_Vector benchmark
echo "══════════════════════════════════════════════════════════════"
echo "▶ HARDWARE BENCHMARK: N_Vector at N=1,000,000"
echo "══════════════════════════════════════════════════════════════"
$BIN/bench_nvector 2>&1
echo ""

# Summary table
echo ""
echo "================================================================"
echo "  SUMMARY TABLE"
echo "================================================================"
printf "%-45s  %12s\n" "Benchmark" "Wall Time"
echo "----------------------------------------------------------------"
for r in "${RESULTS[@]}"; do
    IFS='|' read -r name time info <<< "$r"
    printf "%-45s  %12s\n" "$name" "$time"
done
echo "================================================================"
