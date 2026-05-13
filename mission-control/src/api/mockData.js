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
      frequency_hz: 450,
      growth_rate_1hr: 0.1245,
      efficiency_mu_per_W: 0.045
    }
  },
  phase3: {
    pH_stability: "EXCELLENT",
    final_pH: 7.21,
    final_biomass_gL: 2.85
  },
  phase4_k: { success: true },
  phase4_l: { success: true },
  phase4_m: { success: true },
  phase4_n: { success: true },
  phase4_o: { success: true },
  optimization: [
    { pump_rpm: 1200, pulse_freq: 50, lysis_risk: false, vortex_ratio: 1.4, biomass_growth: 1.25, avg_shear: 0.4 },
    { pump_rpm: 1800, pulse_freq: 0, lysis_risk: true, vortex_ratio: 1.8, biomass_growth: 0.8, avg_shear: 0.9 }
  ]
};

export const MOCK_REPORT = {
  status: "success",
  title: "Disruptive Physics & Autonomous AI for Planetary-Scale Carbon Capture",
  authors: ["Xavier Callens"],
  institution: "SocrateAI Lab",
  date: "May 14, 2026",
  abstract: "We present five disruptive autoresearch protocols that collectively shatter the classical performance ceilings of industrial algal bioreactors for Carbon Capture and Utilization (CCU).",
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
      name: "Protocol M: Acoustofluidic Sparging",
      key_results: {
        Shear_Stress_Pa: 0.02,
        kLa_Max_h: 310
      }
    }
  ],
  latex_source: "% Lean 4 Verified LaTeX Source\\n\\n\\section{Introduction}\\n...",
  generated_at: new Date().toISOString(),
  elapsed_ms: 1245
};
