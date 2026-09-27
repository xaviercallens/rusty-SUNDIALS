"""SYNTHETIC illustration curves for the v12 peer review — NOT a benchmark.

AUDIT NOTE 2026-09-27 (docs/audit/fusion-2026-09-27/README.md, reports A §2/§6,
C §3): the reviewer asked for a PCIe-latency-vs-CPU-SpMV benchmark and an
FP64-vs-FP8 FGMRES residual plot. This script does not measure either. It
evaluates hand-written formulas (cpu=(dof/1000)**2*0.05; fp8_res*=0.78, then
*0.95 + sin(i)*1e-4 "noise"). No solver, PCIe transfer or GPU is involved.
Its output data/fusion/poc_output/v12_poc_results.json was cited in
paper/manuscript_v17.md as "Archived JSON benchmarks"; that citation is
retracted. The output now carries "synthetic": true.
"""

import json
import os
import time
import math

def generate_poc_data() -> None:
    print("Generating SYNTHETIC v12 illustration curves (formulas, not measurements)...")
    
    # 1. PCIe Transfer Overhead POC
    # Simulate DOF scale vs Transfer Latency vs CPU SpMV Time
    pcie_data = []
    dofs = [1000, 5000, 10000, 50000, 100000, 168000]
    
    for dof in dofs:
        # Transfer time is roughly O(N), GPU compute is O(N^2) but heavily parallelized, CPU is O(N^2) but slow.
        cpu_time = (dof / 1000) ** 2 * 0.05  # CPU SpMV scaling
        gpu_compute = (dof / 1000) ** 1.5 * 0.001
        pcie_transfer = (dof * 8 * 2) / (32 * 1024 * 1024 * 1024) * 1000 # ~ microseconds, heavily simplified
        # add overhead
        pcie_transfer += 0.5 # ms base overhead
        
        total_gpu_time = gpu_compute + pcie_transfer
        
        pcie_data.append({
            "dof": dof,
            "cpu_time_ms": round(cpu_time, 3),
            "gpu_total_ms": round(total_gpu_time, 3),
            "pcie_transfer_ms": round(pcie_transfer, 3)
        })

    # 2. FP8 Orthogonality Loss POC
    # Simulate FGMRES Residual norms over iterations
    residual_data = []
    
    fp64_res = 1.0
    fp8_res = 1.0
    
    for iteration in range(1, 51):
        # FP64 converges cleanly
        fp64_res *= 0.75
        
        # FP8 converges fast initially, but hits a noise floor / stalls due to orthogonality loss
        if iteration < 20:
            fp8_res *= 0.78
        else:
            fp8_res *= 0.95 # Stalls
            fp8_res += (math.sin(iteration) * 0.0001) # Add floating point noise
            
        residual_data.append({
            "iteration": iteration,
            "fp64_residual": max(1e-12, fp64_res),
            "fp8_residual": max(1e-8, fp8_res)
        })

    output = {
        "synthetic": True,
        "status": "synthetic_formula_output_not_a_benchmark",
        "note": "Generated from closed-form expressions in scripts/reproduce_v12_poc.py; no solver or GPU was run.",
        "timestamp": time.time(),
        "pcie_benchmark": pcie_data,
        "residual_convergence": residual_data
    }

    os.makedirs("data/fusion/poc_output", exist_ok=True)
    with open("data/fusion/poc_output/v12_poc_results.json", "w") as f:
        json.dump(output, f, indent=2)

    print("SYNTHETIC curves written to data/fusion/poc_output/v12_poc_results.json (not measured)")

if __name__ == "__main__":
    generate_poc_data()
