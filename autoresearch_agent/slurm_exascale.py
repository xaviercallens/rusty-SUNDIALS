"""
SLURM Exascale Submitter (Mac M2 / Cray-1 Simulator Edition)
Dispatches the verified Rust integration codes.
"""
import random
import time

class SecurityError(Exception):
    pass

def submit_job(rust_binary: str, lean_certificate: str) -> bool:
    print("\n" + "="*80)
    print("      *** CRAY-1 OS / MAC M2 SILICON EXECUTOR SUB-SYSTEM ***")
    print("      *** YEAR 1976 - LOS ALAMOS NATIONAL LABORATORY ***")
    print("="*80)
    print(">>> INITIALIZING VECTOR REGISTERS ON APPLE M2 NEURAL ENGINE...")
    time.sleep(0.5)
    print(f">>> READING PUNCH CARDS FOR: {rust_binary[:20]}...")
    
    if not lean_certificate or not lean_certificate.startswith("CERT-LEAN4-"):
        raise SecurityError("Execution aborted! 'no_shortcut_to_deploy' policy violation. "
                            "Cannot dispatch unverified code to Exascale without Lean 4 proof certificate.")
    
    print(f">>> FORMAL PROOF CERTIFICATE {lean_certificate} ACCEPTED BY MAINFRAME.")
    time.sleep(0.5)
    print(">>> LOADING 1970s PHYSICS PROBLEM: PRINCETON LARGE TORUS (PLT) OHMIC HEATING...")
    time.sleep(0.5)
    print(">>> EXECUTING 160 MEGAFLOPS (Simulated via 3.49 GHz ARM M2)...")
    time.sleep(1.0)
    
    # Simulate execution on Exascale cluster
    speedup_achieved = random.random() > 0.1 
    if speedup_achieved:
         print(">>> JOB COMPLETE. CRITICAL PLASMA TEMPERATURE OF 60 MILLION DEGREES REACHED!")
    else:
         print(">>> JOB ABORTED. NUMERICAL INSTABILITY IN VECTOR PIPELINE. (Core Dumped)")
    
    print("="*80 + "\n")
    return speedup_achieved
