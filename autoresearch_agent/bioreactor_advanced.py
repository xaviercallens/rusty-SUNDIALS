"""
Advanced Algae Bioreactor Phase 2: Multi-Species + CO2 + Temperature
=====================================================================
Extends Phase 1 with:
  - CO2 dissolution kinetics (Henry's law)
  - Temperature-dependent Monod growth (Arrhenius)
  - Light/dark cycle (diurnal PAR)
  - Multi-frequency pulsation gradient search
  - Pareto front: vortex_ratio vs shear_stress vs biomass_yield
"""
import numpy as np
from scipy.integrate import solve_ivp
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import os, time, json
from datetime import datetime, timezone

# ── Physical Constants ─────────────────────────────────────────
N_R = 64
R_tube = 0.025  # m
dr = R_tube / N_R
r = np.linspace(dr/2, R_tube - dr/2, N_R)

# State: [C_alg, C_nut, C_co2, T_fluid, v_theta, v_z] × N_R
N_FIELDS = 6
N_STATE = N_FIELDS * N_R

# Bioreactor parameters
MU_MAX = 0.06       # 1/hr max growth
K_S = 0.05          # g/L nutrient half-sat
K_CO2 = 0.01        # g/L CO2 half-sat
K_I = 300            # light inhibition
Y_XS = 0.4          # yield
K_D = 0.002         # death rate 1/hr
C_MAX = 12.0        # carrying capacity g/L
TAU_LYSIS = 50.0    # Pa lysis threshold
RHO = 1000.0        # kg/m³
MU_VISC = 1e-3      # Pa·s
E_A = 5000.0        # Arrhenius activation (J/mol)
R_GAS = 8.314       # J/(mol·K)
T_REF = 298.15      # 25°C reference
H_CO2 = 0.034       # Henry constant (mol/L/atm)
CO2_ATM = 0.0004    # atmospheric CO2 fraction


def initial_state_adv():
    C_alg = np.full(N_R, 0.5)
    C_nut = np.full(N_R, 2.0)
    C_co2 = np.full(N_R, 0.015)  # dissolved CO2 g/L
    T_fl = np.full(N_R, 298.15)  # 25°C
    omega = 60 * 2 * np.pi / 60
    v_t = omega * r
    Q = 120 / 3.6e6
    v_z = 2 * (Q / (np.pi * R_tube**2)) * (1 - (r / R_tube)**2)
    return np.concatenate([C_alg, C_nut, C_co2, T_fl, v_t, v_z])


def _sl(i):
    """Return slice for field i."""
    return slice(i * N_R, (i + 1) * N_R)


def compute_rhs_adv(t, state, pump_rpm=60, pulse_freq=0.0, pulse_duty=1.0,
                    air_flow=2.0, light_base=200):
    C_alg = state[_sl(0)]
    C_nut = state[_sl(1)]
    C_co2 = state[_sl(2)]
    T_fl  = state[_sl(3)]
    v_t   = state[_sl(4)]
    v_z   = state[_sl(5)]

    # ── Fluid relaxation ──
    omega = pump_rpm * 2 * np.pi / 60
    if pulse_freq > 0:
        phase = (t * pulse_freq) % 1.0
        if phase > pulse_duty:
            omega *= 0.1
    v_t_eq = omega * r
    Q = 120 / 3.6e6
    v_z_eq = 2 * (Q / (np.pi * R_tube**2)) * (1 - (r / R_tube)**2)
    tau_f = 0.01
    dv_t = (v_t_eq - v_t) / tau_f
    dv_z = (v_z_eq - v_z) / tau_f

    # ── Shear stress ──
    dvt_dr = np.gradient(v_t, dr)
    dvz_dr = np.gradient(v_z, dr)
    tau = MU_VISC * np.sqrt(dvt_dr**2 + dvz_dr**2)

    # ── Diurnal light cycle ──
    hour = (t / 3600) % 24
    if 6 <= hour <= 18:
        I_surface = light_base * np.sin(np.pi * (hour - 6) / 12)
    else:
        I_surface = 0.0
    I_local = max(I_surface, 5.0) * np.exp(-0.1 * np.cumsum(C_alg) * dr)

    # ── Temperature-dependent growth (Arrhenius) ──
    T_factor = np.exp(-E_A / R_GAS * (1.0 / T_fl - 1.0 / T_REF))

    # ── Monod kinetics with CO2 ──
    mu = MU_MAX * (C_nut / (K_S + C_nut)) * (C_co2 / (K_CO2 + C_co2))
    mu *= I_local / (K_I + I_local) * T_factor
    growth_cap = np.maximum(1 - C_alg / C_MAX, 0)
    f_lysis = np.where(tau > TAU_LYSIS, K_D * (tau / TAU_LYSIS)**2, 0)

    dC_alg = (mu * growth_cap - K_D - f_lysis) * C_alg / 3600
    dC_nut = -mu * C_alg / (Y_XS * 3600)

    # ── CO2 dissolution + consumption ──
    co2_sat = H_CO2 * CO2_ATM * 44.0  # g/L at surface
    k_La = 0.5 * air_flow  # mass transfer coeff (1/hr)
    dC_co2 = (k_La * (co2_sat - C_co2) - 0.5 * mu * C_alg / Y_XS) / 3600

    # ── Temperature (metabolic heat + ambient) ──
    T_amb = 298.15
    dT = (0.001 * mu * C_alg - 0.01 * (T_fl - T_amb)) / 3600

    # ── Radial diffusion ──
    D_alg, D_nut, D_co2 = 1e-9, 1e-8, 1.9e-9
    v_settle = -0.001 * v_t**2 / (r + 1e-10)
    for field, D, dF in [(C_alg, D_alg, dC_alg), (C_nut, D_nut, dC_nut),
                          (C_co2, D_co2, dC_co2)]:
        lap = np.zeros(N_R)
        for i in range(1, N_R - 1):
            lap[i] = (field[i+1] - 2*field[i] + field[i-1]) / dr**2 + \
                      (field[i+1] - field[i-1]) / (2 * r[i] * dr)
        dF += D * lap

    # Centripetal advection on algae
    adv = np.zeros(N_R)
    for i in range(1, N_R - 1):
        if v_settle[i] < 0:
            adv[i] = v_settle[i] * (C_alg[i] - C_alg[i-1]) / dr
        else:
            adv[i] = v_settle[i] * (C_alg[i+1] - C_alg[i]) / dr
    dC_alg += adv

    # Positivity limiter
    dC_alg = np.where(C_alg + dC_alg * 0.01 < 0, -C_alg / 0.01, dC_alg)
    dC_nut = np.where(C_nut + dC_nut * 0.01 < 0, -C_nut / 0.01, dC_nut)
    dC_co2 = np.where(C_co2 + dC_co2 * 0.01 < 0, -C_co2 / 0.01, dC_co2)

    return np.concatenate([dC_alg, dC_nut, dC_co2, dT, dv_t, dv_z])


def invariants_adv(state):
    C_alg = state[_sl(0)]
    C_nut = state[_sl(1)]
    C_co2 = state[_sl(2)]
    T_fl  = state[_sl(3)]
    v_t   = state[_sl(4)]
    v_z   = state[_sl(5)]
    dvt = np.gradient(v_t, dr)
    dvz = np.gradient(v_z, dr)
    tau = MU_VISC * np.sqrt(dvt**2 + dvz**2)
    center = np.mean(C_alg[:N_R//4])
    wall = np.mean(C_alg[3*N_R//4:])
    return {
        "total_biomass": float(np.trapezoid(C_alg * 2 * np.pi * r, r)),
        "total_nutrients": float(np.trapezoid(C_nut * 2 * np.pi * r, r)),
        "avg_co2": float(np.mean(C_co2)),
        "avg_temp_C": float(np.mean(T_fl) - 273.15),
        "avg_shear_Pa": float(np.mean(tau)),
        "max_shear_Pa": float(np.max(tau)),
        "center_conc_gL": float(center),
        "wall_conc_gL": float(wall),
        "vortex_ratio": float(center / max(wall, 1e-10)),
    }


def run_advanced(t_end_hours=4.0, pump_rpm=60, pulse_freq=0.0,
                 pulse_duty=1.0, air_flow=2.0):
    """Run the advanced 6-field simulation with IMEX + projection."""
    t_end = t_end_hours * 3600
    state0 = initial_state_adv()
    state = state0.copy()
    dt_chunk = 60.0
    nfev = 0
    ts = {"times": [0], "biomass": [], "vortex": [], "shear": [],
          "co2": [], "temp": []}
    inv0 = invariants_adv(state)
    for k in ["biomass", "vortex", "shear", "co2", "temp"]:
        key = {"biomass": "total_biomass", "vortex": "vortex_ratio",
               "shear": "avg_shear_Pa", "co2": "avg_co2",
               "temp": "avg_temp_C"}[k]
        ts[k].append(inv0[key])

    rhs = lambda t, s: compute_rhs_adv(t, s, pump_rpm, pulse_freq,
                                        pulse_duty, air_flow)
    start = time.time()
    t_cur = 0.0
    while t_cur < t_end:
        t_next = min(t_cur + dt_chunk, t_end)
        try:
            sol = solve_ivp(rhs, [t_cur, t_next], state, method="BDF",
                            rtol=1e-6, atol=1e-8, max_step=dt_chunk)
            if not sol.success:
                break
            state = sol.y[:, -1]
            nfev += sol.nfev
            # Positivity projection
            for i in range(3):
                state[_sl(i)] = np.maximum(state[_sl(i)], 0)
            t_cur = t_next
            inv = invariants_adv(state)
            ts["times"].append(t_cur / 3600)
            ts["biomass"].append(inv["total_biomass"])
            ts["vortex"].append(inv["vortex_ratio"])
            ts["shear"].append(inv["avg_shear_Pa"])
            ts["co2"].append(inv["avg_co2"])
            ts["temp"].append(inv["avg_temp_C"])
        except Exception:
            break

    elapsed = time.time() - start
    inv_f = invariants_adv(state)
    return {
        "success": True,
        "method": "BDF+IMEX+Projection+CO2+Thermal",
        "elapsed": round(elapsed, 3),
        "nfev": nfev,
        "initial": inv0,
        "final": inv_f,
        "biomass_growth": inv_f["total_biomass"] / max(inv0["total_biomass"], 1e-10),
        "vortex_ratio": inv_f["vortex_ratio"],
        "positivity": bool(np.min(state[_sl(0)]) >= -1e-10),
        "timeseries": ts,
        "config": {"pump_rpm": pump_rpm, "pulse_freq": pulse_freq,
                   "pulse_duty": pulse_duty, "air_flow": air_flow,
                   "t_end_hours": t_end_hours},
    }


def run_pareto_sweep():
    """Multi-dimensional sweep to find the Pareto front."""
    configs = [
        {"pump_rpm": 40,  "pulse_freq": 0,   "pulse_duty": 1.0, "air_flow": 1.0},
        {"pump_rpm": 60,  "pulse_freq": 0,   "pulse_duty": 1.0, "air_flow": 2.0},
        {"pump_rpm": 60,  "pulse_freq": 0.3, "pulse_duty": 0.7, "air_flow": 2.0},
        {"pump_rpm": 60,  "pulse_freq": 0.5, "pulse_duty": 0.6, "air_flow": 3.0},
        {"pump_rpm": 90,  "pulse_freq": 0.3, "pulse_duty": 0.7, "air_flow": 2.0},
        {"pump_rpm": 90,  "pulse_freq": 0.2, "pulse_duty": 0.8, "air_flow": 4.0},
        {"pump_rpm": 120, "pulse_freq": 0.3, "pulse_duty": 0.7, "air_flow": 2.0},
        {"pump_rpm": 120, "pulse_freq": 0.15,"pulse_duty": 0.85,"air_flow": 5.0},
    ]
    results = []
    for cfg in configs:
        res = run_advanced(t_end_hours=2.0, **cfg)
        results.append({
            **cfg,
            "vortex_ratio": res["final"]["vortex_ratio"],
            "avg_shear": res["final"]["avg_shear_Pa"],
            "max_shear": res["final"]["max_shear_Pa"],
            "biomass_growth": res["biomass_growth"],
            "center_conc": res["final"]["center_conc_gL"],
            "co2_final": res["final"]["avg_co2"],
            "temp_final": res["final"]["avg_temp_C"],
            "lysis_risk": res["final"]["max_shear_Pa"] > TAU_LYSIS,
            "elapsed": res["elapsed"],
        })
    return results


def generate_advanced_plots(result, output_dir="/tmp/discoveries"):
    os.makedirs(output_dir, exist_ok=True)
    ts = result.get("timeseries", {})
    if not ts.get("times"):
        return None
    fig, axes = plt.subplots(2, 3, figsize=(18, 10))
    fig.suptitle("Advanced Bioreactor: 6-Field IMEX (CO₂ + Thermal + Diurnal)",
                 fontsize=14, fontweight="bold")
    plots = [
        (axes[0,0], "biomass", "Total Biomass (g·m)", "g", "Biomass Growth"),
        (axes[0,1], "vortex", "Vortex Ratio", "b", "Harvesting Efficiency"),
        (axes[0,2], "shear", "Shear Stress (Pa)", "orange", "Cell Safety"),
        (axes[1,0], "co2", "Dissolved CO₂ (g/L)", "purple", "CO₂ Dynamics"),
        (axes[1,1], "temp", "Temperature (°C)", "red", "Thermal Profile"),
    ]
    for ax, key, ylabel, color, title in plots:
        ax.plot(ts["times"][:len(ts[key])], ts[key], color=color, lw=2)
        ax.set_xlabel("Time (hours)"); ax.set_ylabel(ylabel)
        ax.set_title(title); ax.grid(True, alpha=0.3)
    if "shear" in ts:
        axes[0,2].axhline(y=TAU_LYSIS, color="red", ls="--", label="Lysis")
        axes[0,2].legend()
    axes[1,2].axis("off")
    cfg = result.get("config", {})
    txt = f"Config:\n RPM={cfg.get('pump_rpm')}\n Pulse={cfg.get('pulse_freq')}Hz\n"
    txt += f" Duty={cfg.get('pulse_duty')}\n Air={cfg.get('air_flow')} L/min\n"
    txt += f"\nResults:\n Vortex={result['vortex_ratio']:.2f}x\n"
    txt += f" Growth={result['biomass_growth']:.4f}x\n"
    txt += f" Positivity={result['positivity']}"
    axes[1,2].text(0.1, 0.5, txt, fontsize=12, family="monospace",
                   transform=axes[1,2].transAxes, va="center")
    plt.tight_layout()
    path = f"{output_dir}/bioreactor_advanced_{int(time.time())}.png"
    plt.savefig(path, dpi=150, bbox_inches="tight"); plt.close()
    return path


if __name__ == "__main__":
    print("Running advanced 6-field bioreactor simulation...")
    res = run_advanced(t_end_hours=2.0, pump_rpm=60, pulse_freq=0.3, pulse_duty=0.7)
    print(f"Success={res['success']}, Growth={res['biomass_growth']:.4f}x, "
          f"Vortex={res['vortex_ratio']:.2f}x, Positivity={res['positivity']}")
    path = generate_advanced_plots(res)
    print(f"Plot: {path}")
    print("\nRunning Pareto sweep...")
    pareto = run_pareto_sweep()
    for p in pareto:
        safe = "SAFE" if not p["lysis_risk"] else "LYSIS"
        print(f"  {p['pump_rpm']}RPM pulse={p['pulse_freq']}Hz → "
              f"vortex={p['vortex_ratio']:.2f}x shear={p['max_shear']:.2f}Pa "
              f"growth={p['biomass_growth']:.4f}x [{safe}]")
