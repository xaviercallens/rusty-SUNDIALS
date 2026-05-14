export const MOCK_RESULTS = {
  timestamp: new Date().toISOString(),
  phase1: {
    success: true,
    avg_kla_final: 138.4,
    biomass_final_gL: 2.12,
    co2_utilization_pct: 85.0
  },
  phase2: {
    optimal: {
      frequency_hz: 50,
      growth_rate_1hr: 0.1245,
      efficiency_mu_per_W: 0.014608,
      duty_cycle: 0.20,
      light_intensity: 1000,
      red_blue_ratio: 3.0
    }
  },
  phase3: {
    pH_stability: "EXCELLENT",
    final_pH: 7.21,
    final_biomass_gL: 2.85
  },
  phase3_f: { success: true },
  phase3_g: { success: true },
  phase3_h: { success: true },
  phase3_i: { success: true },
  phase4_k: { success: true },
  phase4_l: { success: true },
  phase4_m: { success: true },
  phase4_n: { success: true },
  phase4_o: { success: true },
  hpc_exascale: {
    a100_speedup: 441.8,
    precision_error: 9.54e-07
  },
  kalundborg: {
    global_optima: "Jubail Industrial City, Saudi Arabia",
    co2_reduction: 32.4,
    agri_boost: 240
  },
  planetary: {
    optimal_node: "Namib Coastal Edge",
    neutrality_years: 0.6,
    drawdown_megatons: 15492
  },
  optimization: [
    { pump_rpm: 1200, pulse_freq: 50, lysis_risk: false, vortex_ratio: 1.4, biomass_growth: 1.25, avg_shear: 0.4 },
    { pump_rpm: 1800, pulse_freq: 0, lysis_risk: true, vortex_ratio: 1.8, biomass_growth: 0.8, avg_shear: 0.9 }
  ]
};

export const MOCK_REPORT = {
  status: "success",
  title: "Earth Digital Twin: Planetary Geo-Optimization for Net-Negative Carbon Drawdown",
  authors: ["Xavier Callens"],
  institution: "SocrateAI Lab",
  date: "May 14, 2026",
  abstract: "Using the rusty-SUNDIALS Parareal PinT engine and NASA POWER CERES/MERRA-2 datasets, the AI auto-research agent simulated the deployment of SymbioticFactory nodes across Earth's coastal deserts. The Namib Coastal Edge emerged as the global optima, yielding a 15,492 Megaton drawdown over 25 years while maintaining an Ecological Disruption Score of zero.",
  equations: [
    { name: "Radiative Transfer Equation", latex: "\\frac{dI_\\lambda}{ds} = -(\\kappa_\\lambda + \\sigma_\\lambda) I_\\lambda + \\dots" },
    { name: "Cahn-Hilliard Navier-Stokes", latex: "\\frac{\\partial \\phi}{\\partial t} + \\mathbf{u} \\cdot \\nabla \\phi = M \\nabla^2 \\mu" }
  ],
  sections: [
    {
      name: "Protocol K: Quantum Dot Upconversion",
      key_results: {
        Equivalent_Efficiency: "18.2%",
        Yield_Limit: "14,120 t/km2"
      }
    },
    {
      name: "Protocol F: Tensor-Train Gyrokinetic Integration",
      key_results: {
        Memory_Footprint: "46.2 MB",
        Run_Time: "14.2s",
        Compression_Ratio: "320,000x"
      }
    },
    {
      name: "Protocol I: HDC Boolean Control",
      key_results: {
        Execution_Latency: "40 ns",
        Speedup_Factor: "1,375x"
      }
    },
    {
      name: "Protocol M: Acoustofluidic Sparging",
      key_results: {
        Shear_Stress_Pa: 0.02,
        kLa_Max_h: 310
      }
    },
    {
      name: "PSC Module SUN: Plasmonic Desalination",
      key_results: {
        Optimal_Enthalpy: "1320.92 kJ/kg",
        ZLD_Status: "Stable"
      }
    },
    {
      name: "PSC Module TERRE: Anaerobic Pyrolysis",
      key_results: {
        Optimal_Syngas: "110.50",
        OC_Ratio: "0.050"
      }
    },
    {
      name: "PSC Module FIRE: HTL & Fermentation",
      key_results: {
        Energy_Density: "37.15 MJ/kg",
        EROI: "6.13"
      }
    },
    {
      name: "Planetary Geo-Optimization (NASA POWER)",
      key_results: {
        Optimal_Node: "Namib Coastal Edge",
        Net_Neutrality: "0.6 Years"
      }
    },
    {
      name: "HPC Exascale Validation (A100 Tensor)",
      key_results: {
        Speedup: "441.8x",
        Precision: "9.54e-07"
      }
    },
    {
      name: "Kalundborg 2.0 Global Anchor Search",
      key_results: {
        Optima: "Jubail Industrial City",
        CO2_Reduction: "32.4 Mt/yr"
      }
    }
  ],
  latex_source: "% Lean 4 Verified LaTeX Source\\n\\n\\section{Introduction}\\n...",
  generated_at: new Date().toISOString(),
  elapsed_ms: 1245
};

export const MOCK_VERIFICATION = {
  total_proofs: 4,
  proved: 4,
  failed: 0,
  pending: 0,
  pass_rate: 100,
  timestamp: new Date().toISOString(),
  elapsed_ms: 7340,
  proofs: [
    {
      theorem: "theorem hpc_tensor_core_precision (a100 : GPU)",
      module: "hpc_exascale.lean",
      status: "proved",
      evidence: "Verified precision bound ≤ 10⁻⁶",
      certificate: "CERT-A100-8891",
      time_ms: 1250,
      lean4: "theorem hpc_tensor_core_precision (a100 : GPU) : \\n  ∀ t, error (tensor_gmres a100 t) ≤ 1e-6 := by\\n  apply tensor_error_bound\\n  exact hpc_precision_axioms"
    },
    {
      theorem: "theorem kalundborg_topology_valid (eip : SymbioticFactory)",
      module: "global_eip_safety.lean",
      status: "proved",
      evidence: "Verified Zero-Waste cascade topology",
      certificate: "CERT-EIP-414",
      time_ms: 840,
      lean4: "theorem kalundborg_topology_valid (eip : SymbioticFactory) :\\n  is_zero_waste (eip) ∧ valid_cascade (eip) := by\\n  apply thermodynamic_cascade\\n  exact eip_graph_theory"
    },
    {
      theorem: "theorem earth_twin_drawdown_bounds (planet : Earth)",
      module: "planetary_geo.lean",
      status: "proved",
      evidence: "Verified Net-Negative Carbon Flow",
      certificate: "CERT-GEO-992",
      time_ms: 2150,
      lean4: "theorem earth_twin_drawdown_bounds (planet : Earth) :\\n  carbon_flow (planet.atmosphere) < 0 := by\\n  apply navier_stokes_drawdown\\n  exact coastal_desert_optima"
    },
    {
      theorem: "theorem iter_gyrokinetic_stability (plasma : Fusion)",
      module: "iter_phase3.lean",
      status: "proved",
      evidence: "Verified Tensor-Train Phase-Space bounds",
      certificate: "CERT-FUS-110",
      time_ms: 3100,
      lean4: "theorem iter_gyrokinetic_stability (plasma : Fusion) :\\n  is_stable (tensor_train plasma) := by\\n  apply lyapunov_stability\\n  exact mhd_bounds"
    }
  ]
};
