#!/usr/bin/env python3
"""
ITER Fusion Data Integration for rusty-SUNDIALS v9/v10

This script:
1. Explores the IMAS Data Dictionary to identify relevant plasma data structures
2. Generates an ITER-like tokamak equilibrium via FreeGS (Grad-Shafranov solver)
3. Computes relevant plasma parameters using PlasmaPy
4. Exports initial conditions as CSV for CVODE integration

Output: data/fusion/iter_equilibrium.csv — psi(r), profiles for MHD simulations
"""

import os
import sys
import csv
import numpy as np

# ═══════════════════════════════════════════════════════════════════
# Part 1: IMAS Data Dictionary Exploration
# ═══════════════════════════════════════════════════════════════════

def explore_imas_data_dictionary():
    """Explore the IMAS data model to identify relevant Interface Data Structures."""
    try:
        import imas

        print("=" * 70)
        print(" IMAS Data Dictionary Exploration (imas-python v{})".format(imas.__version__))
        print("=" * 70)

        # List available IDS types relevant to MHD
        mhd_relevant_ids = [
            "equilibrium",
            "core_profiles",
            "mhd_linear",
            "magnetics",
            "summary",
        ]

        print("\n📋 MHD-relevant Interface Data Structures (IDS):")
        for ids_name in mhd_relevant_ids:
            try:
                # Create an empty IDS to explore its structure
                ids_obj = imas.IDSFactory.new(ids_name)
                print(f"  ✅ {ids_name:<20s} — accessible")
            except TypeError:
                # Try alternative API
                try:
                    ids_obj = getattr(imas, ids_name, None)
                    if ids_obj is not None:
                        print(f"  ✅ {ids_name:<20s} — found in imas module")
                    else:
                        print(f"  ℹ️  {ids_name:<20s} — defined in data dictionary")
                except Exception:
                    print(f"  ℹ️  {ids_name:<20s} — defined in data dictionary")

        return True
    except ImportError:
        print("⚠️  imas-python not available, skipping IMAS exploration")
        return False
    except Exception as e:
        print(f"ℹ️  IMAS exploration limited: {e}")
        return False


# ═══════════════════════════════════════════════════════════════════
# Part 2: FreeGS Equilibrium Generation (ITER-like)
# ═══════════════════════════════════════════════════════════════════

def generate_iter_equilibrium(output_dir: str):
    """
    Generate an ITER-like tokamak equilibrium using FreeGS.
    
    Returns ψ(R,Z) and 1D profiles that can be used as initial conditions
    for rusty-SUNDIALS MHD simulations.
    """
    try:
        import freegs
        import freegs.geqdsk
    except ImportError:
        print("⚠️  freegs not available, skipping equilibrium generation")
        return None

    print("\n" + "=" * 70)
    print(" FreeGS ITER-like Equilibrium Generation")
    print("=" * 70)

    # ITER-like geometry parameters
    R0 = 6.2    # Major radius [m]
    a = 2.0     # Minor radius [m]
    B0 = 5.3    # Toroidal field at R0 [T]
    Ip = 15e6   # Plasma current [A] (15 MA)
    kappa = 1.7 # Elongation
    delta = 0.33 # Triangularity

    print(f"\n  Major radius R0 = {R0} m")
    print(f"  Minor radius a  = {a} m")
    print(f"  Toroidal field   = {B0} T")
    print(f"  Plasma current   = {Ip/1e6:.0f} MA")
    print(f"  Elongation κ     = {kappa}")
    print(f"  Triangularity δ  = {delta}")

    # Create a simple tokamak machine
    # Use built-in coil sets for a generic tokamak
    coils = [
        ("P1", freegs.machine.Coil(R0 + a + 0.5, 0.0)),
        ("P2", freegs.machine.Coil(R0 + a + 0.5, a * kappa)),
        ("P3", freegs.machine.Coil(R0 - a - 0.5, 0.0)),
        ("P4", freegs.machine.Coil(R0 - a - 0.5, a * kappa)),
        ("P5", freegs.machine.Coil(R0, a * kappa + 1.0)),
        ("P6", freegs.machine.Coil(R0, -(a * kappa + 1.0))),
    ]

    tokamak = freegs.machine.Machine(coils)

    # Create equilibrium on a grid
    eq = freegs.Equilibrium(
        tokamak=tokamak,
        Rmin=R0 - a - 1.5,  # Grid extends beyond plasma
        Rmax=R0 + a + 1.5,
        Zmin=-(a * kappa + 1.5),
        Zmax=a * kappa + 1.5,
        nx=65,  # Grid resolution
        ny=65,
        boundary=freegs.boundary.freeBoundaryHagenow,
    )

    # Define plasma profiles (pressure and current)
    fvac = R0 * B0  # Vacuum f = R*Bt
    profiles = freegs.jtor.ConstrainPaxisIp(
        eq,
        paxis=1e5,   # Peak pressure [Pa]
        Ip=Ip,       # Plasma current
        fvac=fvac,
    )

    # Solve the Grad-Shafranov equation
    print("\n  Solving Grad-Shafranov equation...")
    try:
        freegs.solve(
            eq,
            profiles,
            show=False,
        )
        print("  ✅ Equilibrium converged!")
    except Exception as e:
        print(f"  ⚠️  Solver warning: {e}")
        print("  Continuing with partial solution...")

    # Extract 1D profiles along midplane (Z=0)
    R_1d = np.linspace(R0 - a, R0 + a, 100)
    Z_mid = 0.0

    # Get ψ along midplane using interpolation
    psi_1d = np.array([eq.psiRZ(R, Z_mid) for R in R_1d])

    # Normalize to ψ_n ∈ [0, 1]
    psi_axis = getattr(eq, 'psi_axis', None)
    psi_bndry = getattr(eq, 'psi_bndry', None)
    psi_min = psi_axis if psi_axis is not None else float(np.min(psi_1d))
    psi_max = psi_bndry if psi_bndry is not None else float(np.max(psi_1d))
    if abs(psi_max - psi_min) > 1e-10:
        psi_norm = (psi_1d - psi_min) / (psi_max - psi_min)
    else:
        psi_norm = np.zeros_like(psi_1d)

    # Compute ρ (normalized minor radius)
    rho = (R_1d - R0) / a

    # Simple model profiles (parabolic)
    # These approximate ITER baseline scenario profiles
    T_e = 25.0 * (1.0 - rho**2)**2  # Electron temperature [keV]
    n_e = 1e20 * (1.0 - 0.8 * rho**2)  # Electron density [m^-3]
    j_phi = (Ip / (np.pi * a**2)) * (1.0 - rho**2)**1.5  # Current density [A/m^2]

    # Export to CSV
    csv_path = os.path.join(output_dir, "iter_equilibrium.csv")
    with open(csv_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow([
            "rho", "R_m", "psi_Wb", "psi_norm",
            "Te_keV", "ne_m3", "j_phi_Am2"
        ])
        for i in range(len(R_1d)):
            writer.writerow([
                f"{rho[i]:.6f}",
                f"{R_1d[i]:.6f}",
                f"{psi_1d[i]:.10e}",
                f"{psi_norm[i]:.10e}",
                f"{T_e[i]:.6f}",
                f"{n_e[i]:.4e}",
                f"{j_phi[i]:.4e}",
            ])

    print(f"  📊 Exported: {csv_path} ({len(R_1d)} radial points)")
    print(f"     ψ range: [{psi_min:.4e}, {psi_max:.4e}] Wb")
    print(f"     T_e(0) = {T_e[50]:.1f} keV, n_e(0) = {n_e[50]:.2e} m⁻³")

    return {
        "rho": rho,
        "R": R_1d,
        "psi": psi_1d,
        "psi_norm": psi_norm,
        "Te": T_e,
        "ne": n_e,
        "j_phi": j_phi,
        "csv_path": csv_path,
    }


# ═══════════════════════════════════════════════════════════════════
# Part 3: PlasmaPy Parameter Computation
# ═══════════════════════════════════════════════════════════════════

def compute_plasma_parameters():
    """
    Compute key plasma parameters for MHD simulations using PlasmaPy.
    
    These values are used as coefficients in the RHS functions of
    tearing_mode_hero_test.rs and fusion_mhd_benchmark.rs.
    """
    try:
        import astropy.units as u
        from plasmapy.formulary import (
            Alfven_speed,
            plasma_frequency,
            gyrofrequency,
            thermal_speed,
        )
        from plasmapy.formulary.dimensionless import lundquist_number
    except ImportError:
        print("⚠️  plasmapy not available, using hardcoded ITER parameters")
        return compute_iter_parameters_manual()

    print("\n" + "=" * 70)
    print(" PlasmaPy: ITER Plasma Parameter Computation")
    print("=" * 70)

    # ITER baseline scenario parameters
    B0 = 5.3 * u.T           # Toroidal magnetic field
    n_e = 1e20 * u.m**(-3)   # Electron density
    T_e = 25.0 * u.keV        # Electron temperature (peak)
    T_i = 20.0 * u.keV        # Ion temperature (peak)
    L = 2.0 * u.m             # Characteristic length (minor radius)

    print(f"\n  B₀ = {B0}")
    print(f"  nₑ = {n_e}")
    print(f"  Tₑ = {T_e}")
    print(f"  Tᵢ = {T_i}")
    print(f"  L  = {L}")

    # Alfvén speed
    v_A = Alfven_speed(B=B0, density=n_e, ion="D+")
    print(f"\n  Alfvén speed          vₐ = {v_A.to(u.m/u.s):.4e}")

    # Thermal speed
    v_th_e = thermal_speed(T=T_e, particle="e-")
    v_th_i = thermal_speed(T=T_i, particle="D+")
    print(f"  Electron thermal speed    = {v_th_e.to(u.m/u.s):.4e}")
    print(f"  Ion thermal speed         = {v_th_i.to(u.m/u.s):.4e}")

    # Gyrofrequencies
    omega_ci = gyrofrequency(B=B0, particle="D+")
    omega_ce = gyrofrequency(B=B0, particle="e-")
    print(f"  Ion gyrofrequency    ωci = {omega_ci.to(u.rad/u.s):.4e}")
    print(f"  Electron gyrofreq   ωce = {omega_ce.to(u.rad/u.s):.4e}")

    # Plasma frequency
    omega_pe = plasma_frequency(n=n_e, particle="e-")
    print(f"  Plasma frequency    ωpe = {omega_pe.to(u.rad/u.s):.4e}")

    # Resistivity (Spitzer)
    eta = 1e-8  # Ω·m (approximate for 25 keV ITER plasma)
    mu0 = 4 * np.pi * 1e-7
    tau_R = mu0 * (L.value**2) / eta  # Resistive diffusion time
    tau_A = L.value / v_A.to(u.m / u.s).value  # Alfvén transit time

    # Lundquist number S = τ_R / τ_A
    S = tau_R / tau_A

    print(f"\n  Resistivity η        = {eta:.2e} Ω·m")
    print(f"  Resistive time τ_R   = {tau_R:.4e} s")
    print(f"  Alfvén time τ_A      = {tau_A:.4e} s")
    print(f"  Lundquist number S   = {S:.4e}")
    print(f"  → Tearing growth rate γ ~ S^(-3/5) τ_A^(-1)")

    # Tearing mode growth rate estimate (classical)
    gamma_tearing = S**(-3.0/5.0) / tau_A
    print(f"  → γ_tearing ≈ {gamma_tearing:.4e} s⁻¹")
    print(f"  → τ_tearing ≈ {1.0/gamma_tearing:.4e} s")

    results = {
        "B0_T": B0.value,
        "ne_m3": n_e.value,
        "Te_keV": T_e.value,
        "Ti_keV": T_i.value,
        "v_A_ms": v_A.to(u.m/u.s).value,
        "eta_Ohm_m": eta,
        "tau_R_s": tau_R,
        "tau_A_s": tau_A,
        "S_lundquist": S,
        "gamma_tearing_s1": gamma_tearing,
    }

    return results


def compute_iter_parameters_manual():
    """Fallback: compute ITER parameters without PlasmaPy."""
    B0 = 5.3       # T
    n_e = 1e20      # m^-3
    T_e = 25.0      # keV
    L = 2.0         # m
    m_D = 3.34e-27  # kg (deuteron mass)
    mu0 = 4 * np.pi * 1e-7
    eta = 1e-8      # Ω·m

    v_A = B0 / np.sqrt(mu0 * n_e * m_D)
    tau_R = mu0 * L**2 / eta
    tau_A = L / v_A
    S = tau_R / tau_A
    gamma_tearing = S**(-3.0/5.0) / tau_A

    return {
        "B0_T": B0,
        "ne_m3": n_e,
        "Te_keV": T_e,
        "v_A_ms": v_A,
        "eta_Ohm_m": eta,
        "tau_R_s": tau_R,
        "tau_A_s": tau_A,
        "S_lundquist": S,
        "gamma_tearing_s1": gamma_tearing,
    }


# ═══════════════════════════════════════════════════════════════════
# Part 4: Export parameters as Rust constants
# ═══════════════════════════════════════════════════════════════════

def export_rust_constants(params: dict, output_dir: str):
    """Generate a Rust module with ITER plasma constants for MHD simulations."""
    rust_path = os.path.join(output_dir, "iter_plasma_constants.rs")
    with open(rust_path, "w") as f:
        f.write("//! ITER Baseline Scenario Plasma Constants\n")
        f.write("//!\n")
        f.write("//! Auto-generated by scripts/fusion_data_integration.py\n")
        f.write("//! Source: PlasmaPy + ITER design parameters\n")
        f.write("//!\n")
        f.write("//! These constants can be used as coefficients in the RHS\n")
        f.write("//! functions of MHD benchmark examples.\n\n")
        f.write("#![allow(dead_code)]\n\n")

        for key, value in params.items():
            const_name = key.upper()
            f.write(f"/// {key}\n")
            if isinstance(value, float):
                f.write(f"pub const {const_name}: f64 = {value:.10e};\n\n")
            else:
                f.write(f"pub const {const_name}: f64 = {float(value):.10e};\n\n")

    print(f"\n  📝 Exported Rust constants: {rust_path}")

    # Also export as CSV for the paper
    csv_path = os.path.join(output_dir, "iter_plasma_parameters.csv")
    with open(csv_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["parameter", "value", "unit"])
        units = {
            "B0_T": "T", "ne_m3": "m^-3", "Te_keV": "keV", "Ti_keV": "keV",
            "v_A_ms": "m/s", "eta_Ohm_m": "Ω·m", "tau_R_s": "s", "tau_A_s": "s",
            "S_lundquist": "—", "gamma_tearing_s1": "s^-1",
        }
        for key, value in params.items():
            writer.writerow([key, f"{float(value):.6e}", units.get(key, "")])

    print(f"  📊 Exported CSV: {csv_path}")


# ═══════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════

def main():
    output_dir = os.path.join(
        os.path.dirname(os.path.abspath(__file__)),
        "..", "data", "fusion"
    )
    os.makedirs(output_dir, exist_ok=True)

    print("╔══════════════════════════════════════════════════════════════════╗")
    print("║   rusty-SUNDIALS Fusion Data Integration Pipeline              ║")
    print("║   ITER / MAST / TokaMark → CVODE Initial Conditions           ║")
    print("╚══════════════════════════════════════════════════════════════════╝")

    # Step 1: IMAS exploration
    explore_imas_data_dictionary()

    # Step 2: Generate ITER-like equilibrium
    eq_data = generate_iter_equilibrium(output_dir)

    # Step 3: Compute plasma parameters
    params = compute_plasma_parameters()

    # Step 4: Export Rust constants
    if params:
        export_rust_constants(params, output_dir)

    print("\n" + "=" * 70)
    print(" ✅ Integration pipeline complete!")
    print("=" * 70)
    print(f"\n  Output directory: {output_dir}")
    print("  Files generated:")
    for f in sorted(os.listdir(output_dir)):
        path = os.path.join(output_dir, f)
        size = os.path.getsize(path)
        print(f"    {f} ({size:,} bytes)")

    print("\n  Next: Use these in rusty-SUNDIALS examples:")
    print("    cargo run --example tearing_mode_hero_test")
    print("    cargo run --example fusion_mhd_benchmark")


if __name__ == "__main__":
    main()
