# Fusion & ITER Open-Source Scientific Datasets for rusty-SUNDIALS

**Objective:** Identify datasets, tools, and visualization infrastructure from the ITER/fusion ecosystem that can provide real-world validation data and 3D scientific visualization for the MHD, tearing mode, and plasma dynamics experiments implemented in rusty-SUNDIALS v9/v10.

> **Context (May 2026):** The ITER Organization has officially released the Integrated Modeling and Analysis Suite (IMAS), including physics modelling codes for tokamak plasma scenarios, under open-source licenses on GitHub. This historic decision by the ITER Director-General enables the global fusion community — including privately-funded initiatives — to access, validate, and contribute to the same infrastructure used for ITER plasma operations.
>
> — Sources: [ITER Organization GitHub](https://github.com/iterorganization), [EUROfusion: 3D Visualization Brings Nuclear Fusion to Life](https://euro-fusion.org/member-news/3d-visualization-brings-nuclear-fusion-to-life/), [Springer: Visualization of Fusion Plasmas (2026)](https://link.springer.com/article/10.1007/s10894-026-00549-z)

---

## 🏆 Tier 1: Immediately Actionable (can integrate this week)

### 1. TokaMark — MAST Tokamak AI Benchmark
> [!IMPORTANT]
> **The most immediately useful dataset.** Real experimental data from the Mega Ampere Spherical Tokamak (MAST) at UKAEA, covering MHD activity, tearing modes, and disruption prediction.

| Field | Detail |
|-------|--------|
| **Source** | UKAEA + IBM + STFC |
| **Access** | `huggingface-cli download UKAEA-IBM-STFC/tokamark-dataset` |
| **License** | Open (Hugging Face) |
| **Format** | HDF5/CSV with metadata |
| **Size** | ~50 GB (full), task subsets available |
| **14 Tasks** | MHD mode detection, disruption prediction, profile reconstruction, equilibrium inference |

**Integration with rusty-SUNDIALS:**
- Use TokaMark's **MHD mode activity** data as ground-truth for `tearing_mode_hero_test.rs`
- Validate `fusion_mhd_benchmark.rs` tearing mode growth rates against MAST shots
- Feed time-series profiles into CVODE as initial conditions for reduced MHD evolution
- Compare Neural ODE predictions (v10 roadmap) against experimental trajectories

```bash
# Download the dataset
pip install huggingface_hub
huggingface-cli download UKAEA-IBM-STFC/tokamark-dataset \
  --repo-type dataset --local-dir ./data/tokamark
```

---

### 2. ITER IMAS Data Dictionary + imas-python
> [!NOTE]
> The **official ITER data model** — now open-source. Provides standardized plasma scenario data structures used across all ITER-member institutions.

| Field | Detail |
|-------|--------|
| **Source** | ITER Organization (official) |
| **GitHub** | [iterorganization/IMAS-Python](https://github.com/iterorganization/IMAS-Python) |
| **License** | LGPL-3.0 |
| **Install** | `pip install imas-python` |
| **Data Format** | HDF5/netCDF via Interface Data Structures (IDS) |

**Key IDSs for rusty-SUNDIALS:**

| IDS Name | Content | Use Case |
|----------|---------|----------|
| `equilibrium` | Grad-Shafranov solution, ψ profiles | Initial conditions for MHD |
| `core_profiles` | Temperature, density, current profiles | Boundary conditions |
| `mhd_linear` | Linear MHD stability results | Validation targets |
| `disruption` | Disruption event data | Tearing mode onset prediction |
| `magnetics` | Magnetic diagnostics (Mirnov coils) | Compare with simulation B-field |

```python
import imas

# Load an ITER scenario from HDF5
with imas.DBEntry("imas:hdf5?path=./iter_scenario", "r") as db:
    eq = db.get("equilibrium")
    profiles = db.get("core_profiles")
    
    # Extract ψ(r) for initial condition
    psi = eq.time_slice[0].profiles_1d.psi
    rho = eq.time_slice[0].profiles_1d.rho_tor_norm
```

---

### 3. FAIR-MAST — Raw MAST Diagnostics
| Field | Detail |
|-------|--------|
| **Source** | UKAEA (Culham Centre for Fusion Energy) |
| **Access** | Public API: `https://fair-mast.fusion.ukaea.uk/` |
| **Content** | 30+ years of MAST experimental data |
| **Format** | Searchable metadata + raw signals |

**Use case:** Ground-truth validation for reduced MHD simulations. Access specific discharge shots with known tearing mode activity.

---

### 4. IMAS-ParaView — 3D Visualization of ITER Disruption Simulations
> [!TIP]
> **The official ITER visualization toolkit** for 3D post-processing of JOREK disruption simulations. Visualizes induced currents in vacuum vessel structures and plasma electron temperature evolution during ITER disruptions.

| Field | Detail |
|-------|--------|
| **Source** | ITER Organization |
| **GitHub** | [iterorganization/IMAS-ParaView](https://github.com/iterorganization/IMAS-ParaView) |
| **License** | LGPL-3.0 |
| **Version** | v2.3.0 (March 2026) |
| **Language** | Python 100% |
| **Docs** | [imas-paraview.readthedocs.io](https://imas-paraview.readthedocs.io/en/latest/) |
| **Gallery** | [Example images & animations](https://imas-paraview.readthedocs.io/en/latest/gallery.html) |

**Capabilities:**
- Converts GGD (Generalized Grid Description) structures to VTK formats
- ParaView plugins for IDS data visualization (both GGD and non-GGD)
- Time-dependent 3D animation of disruption evolution
- Induced current visualization in vacuum vessel during thermal quench
- Electron temperature field rendering with JOREK simulation data
- VR-ready output for CAVE-type and HMD visualization systems

**Integration with rusty-SUNDIALS:**
- Export CVODE MHD simulation results to IMAS IDS format → visualize in ParaView
- Compare rusty-SUNDIALS tearing mode evolution with JOREK reference animations
- Generate publication-quality 3D figures for the TOMS manuscript
- Create time-lapse videos of B-field and T_e evolution for conference presentations

**Visualization Pipeline:**
```
rusty-SUNDIALS (CVODE)  →  CSV/HDF5  →  imas-python (IDS)  →  IMAS-ParaView  →  3D VTK/ParaView
         ↓                                     ↓                       ↓
   MHD state vector          IMAS Data Dictionary           Time-dependent 3D animation
   (ψ, B, T_e, j)           (equilibrium IDS)              + induced current maps
```

```python
# Example: Export rusty-SUNDIALS results for IMAS-ParaView visualization
import imas
from imas_paraview import ggd_to_vtk

# Load simulation results into IMAS IDS
with imas.DBEntry("imas:hdf5?path=./rusty_sundials_output", "w") as db:
    eq = imas.IDSFactory.new("equilibrium")
    eq.time_slice.resize(1)
    eq.time_slice[0].profiles_1d.psi = psi_from_cvode
    eq.time_slice[0].profiles_1d.rho_tor_norm = rho_grid
    db.put(eq)

# Convert to VTK for ParaView
ggd_to_vtk.convert("./rusty_sundials_output", output_dir="./vtk_output")
```

---

## 🔬 Tier 2: Research-Grade (requires setup, high scientific value)

### 4. FreeGS / FreeGSNKE — Tokamak Equilibrium Solver

| Field | Detail |
|-------|--------|
| **Source** | UKAEA / Fusion Computing Lab |
| **GitHub** | [freegs-plasma/freegs](https://github.com/freegs-plasma/freegs) |
| **License** | LGPL |
| **Install** | `pip install freegs` |

**Integration:** Generate ITER-geometry Grad-Shafranov equilibria as initial conditions for rusty-SUNDIALS MHD simulations. FreeGSNKE adds time evolution — direct comparison target.

```python
import freegs

# Create ITER-like tokamak equilibrium
tokamak = freegs.machine.ITER()
eq = freegs.Equilibrium(tokamak, ...)
freegs.solve(eq, profiles)

# Export ψ(R,Z) → use as IC for rusty-SUNDIALS tearing mode sim
psi_grid = eq.psi()
```

---

### 5. disruption-py — MIT PSFC Disruption Framework

| Field | Detail |
|-------|--------|
| **Source** | MIT Plasma Science & Fusion Center |
| **GitHub** | [MIT-PSFC/disruption-py](https://github.com/MIT-PSFC/disruption-py) |
| **License** | Open source |
| **Content** | Physics-based disruption analysis for C-Mod, DIII-D, EAST |

**Integration:** Extract tearing mode stability parameters (Δ') from real tokamak data. Use as ODE boundary conditions for reduced MHD tearing mode growth in rusty-SUNDIALS.

---

### 6. PlasmaPy — Community Python Package

| Field | Detail |
|-------|--------|
| **Source** | PlasmaPy Community |
| **GitHub** | [PlasmaPy/PlasmaPy](https://github.com/PlasmaPy/PlasmaPy) |
| **License** | BSD-3 |
| **Install** | `pip install plasmapy` |

**Use case:** Compute plasma parameters (Alfvén speed, resistivity, Lundquist number) from experimental data → feed into rusty-SUNDIALS MHD RHS coefficients.

---

## 🛰️ Tier 3: Advanced / Institutional (future v10 roadmap)

### 8. JOREK — Non-linear Extended MHD Code + IMAS-ParaView Visualization

| Field | Detail |
|-------|--------|
| **Source** | CEA / EUROfusion |
| **Website** | [jorek.eu](https://jorek.eu/) |
| **Access** | Research collaboration required |
| **Visualization** | IMAS-ParaView renders JOREK disruption data as 3D animations |

**Why it matters:** JOREK is the gold-standard for non-linear tearing mode and ELM simulations. Its disruption simulations — visualized via IMAS-ParaView — show induced currents in ITER's vacuum vessel and plasma electron temperature during the thermal quench phase. Published benchmark data (2/1 tearing mode linear growth rates) can be used as validation targets for rusty-SUNDIALS reduced MHD.

**JOREK + IMAS-ParaView Disruption Visualization:**
- **Induced currents in vacuum vessel**: 3D rendering of eddy currents generated during vertical displacement events (VDEs)
- **Electron temperature (T_e)**: Time-resolved 3D field showing thermal quench propagation
- **Magnetic flux surfaces**: Stochastization and island formation during tearing mode growth
- These visualizations are directly referenced in the [EUROfusion 3D visualization initiative](https://euro-fusion.org/member-news/3d-visualization-brings-nuclear-fusion-to-life/)

**Published benchmark values (from literature):**
- Circular tokamak, aspect ratio R/a = 10, Lundquist S = 10⁴–10⁸
- 2/1 tearing mode linear growth rate γ vs. S
- Agreement with CASTOR3D linear code within 1–2%

---

### 9. ITER Scenario Database (restricted)
- Curated ITER plasma scenarios (15 MA baseline, hybrid, advanced)
- Accessible via OMAS + institutional credentials
- Contains coil currents, plasma shapes, heating profiles
- **Future v10 target** for federated auto-research (Component 6)

---

### 10. VR Fusion Visualization (NIFS / EUROfusion)

| Field | Detail |
|-------|--------|
| **Source** | National Institute for Fusion Science (NIFS) + EUROfusion |
| **Paper** | [Visualization of Fusion Plasmas, J. Fusion Energy (2026)](https://link.springer.com/article/10.1007/s10894-026-00549-z) |
| **Technology** | CAVE-type VR + Head-Mounted Displays (HMDs) |

**Relevance:** The latest VR visualization research enables immersive exploration of MHD simulation data in 3D. Combined with IMAS-ParaView outputs, rusty-SUNDIALS results could be rendered in VR environments for multi-researcher collaborative analysis of tearing mode dynamics and disruption events.

---

## 📊 Mapping to rusty-SUNDIALS v9/v10 Experiments

| rusty-SUNDIALS Module | Dataset | What it provides |
|----------------------|---------|-----------------|
| `tearing_mode_hero_test.rs` | TokaMark MHD tasks, JOREK benchmarks | Growth rates, mode structure validation |
| `fusion_mhd_benchmark.rs` | IMAS equilibrium IDS, FreeGS | Realistic ψ(r) initial conditions |
| `exp5_fogno_xmhd.rs` | FAIR-MAST diagnostics | B-field time series for comparison |
| `fusion_sciml_phase5.rs` | PlasmaPy parameters | Physical coefficients (η, ν, S) |
| **All MHD examples** | **IMAS-ParaView** | **3D visualization of simulation output** |
| v10 Neural ODE | TokaMark time series | Training data for plasma dynamics |
| v10 Disruption prediction | disruption-py, TokaMark Task 1 | Labeled disruption events |

---

## 🚀 Recommended Next Steps

### Quick Win (today)
1. **Download TokaMark subset** → extract MHD mode detection data
2. **Install imas-python** → explore IMAS Data Dictionary structure
3. **Install FreeGS** → generate an ITER-like equilibrium

### Integration Sprint (this week)
4. Create `data/` directory with download scripts
5. Write a Python→CSV bridge that exports equilibrium profiles for CVODE
6. Add ITER-geometry initial conditions to `tearing_mode_hero_test.rs`
7. Validate tearing growth rates against published JOREK benchmarks

### Paper Extension (for TOMS submission)
8. Add §7.2: "Validation Against MAST Experimental Data (TokaMark)"
9. Add Table 4: "Tearing Mode Growth Rates: rusty-SUNDIALS vs. JOREK vs. MAST"
10. Reference ITER IMAS open-source initiative as community alignment

---

## 🎨 Visualization Pipeline: rusty-SUNDIALS → IMAS-ParaView

```
┌──────────────────────┐    ┌─────────────────────┐    ┌──────────────────────┐
│  rusty-SUNDIALS      │    │  IMAS Infrastructure │    │  Visualization       │
│                      │    │                      │    │                      │
│  CVODE MHD solver    │───►│  imas-python         │───►│  IMAS-ParaView       │
│  - ψ(R,Z,t)         │    │  - equilibrium IDS   │    │  - 3D VTK rendering  │
│  - B(R,Z,t)         │CSV │  - core_profiles IDS │HDF5│  - Time animation    │
│  - T_e(R,Z,t)       │───►│  - mhd_linear IDS    │───►│  - Vacuum vessel     │
│  - j(R,Z,t)         │    │  - magnetics IDS     │    │  - Induced currents  │
│                      │    │                      │    │  - VR (HMD/CAVE)     │
└──────────────────────┘    └─────────────────────┘    └──────────────────────┘
        ↓                                                        ↓
   Rust (safe, no_unsafe)                              Publication figures
   + Lean 4 verification                              + Conference animations
                                                       + VR collaborative analysis
```

**End-to-end workflow:**
1. Run `cargo run --example tearing_mode_hero_test` → CSV output
2. `python3 scripts/fusion_data_integration.py` → IMAS IDS (HDF5)
3. `paraview --data=./vtk_output/` → 3D interactive visualization
4. Export frames → paper figures / MP4 animation

---

## References

1. TokaMark: UKAEA-IBM-STFC/tokamark-dataset (Hugging Face, 2025)
2. IMAS-Python: github.com/iterorganization/IMAS-Python (LGPL-3.0)
3. IMAS Data Dictionary: github.com/iterorganization/IMAS-Data-Dictionary
4. **IMAS-ParaView: github.com/iterorganization/IMAS-ParaView (LGPL-3.0, v2.3.0)**
5. FreeGS: github.com/freegs-plasma/freegs (LGPL)
6. FreeGSNKE: github.com/FusionComputingLab/freegsnke
7. disruption-py: github.com/MIT-PSFC/disruption-py
8. PlasmaPy: github.com/PlasmaPy/PlasmaPy (BSD-3)
9. JOREK: jorek.eu + published benchmarks in Phys. Plasmas
10. FAIR-MAST: fair-mast.fusion.ukaea.uk
11. OMAS: github.com/gafusion/omas
12. **EUROfusion: "3D Visualization Brings Nuclear Fusion to Life" (2026)**
    https://euro-fusion.org/member-news/3d-visualization-brings-nuclear-fusion-to-life/
13. **Visualization of Fusion Plasmas, J. Fusion Energy (2026)**
    https://link.springer.com/article/10.1007/s10894-026-00549-z
14. **ITER Organization Open-Source Announcement (2025–2026)**
    https://github.com/iterorganization
