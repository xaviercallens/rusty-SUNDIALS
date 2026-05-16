# Fusion & ITER Open-Source Scientific Datasets for rusty-SUNDIALS

**Objective:** Identify datasets and tools from the ITER/fusion ecosystem that can provide real-world validation data for the MHD, tearing mode, and plasma dynamics experiments implemented in rusty-SUNDIALS v9/v10.

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

### 7. JOREK — Non-linear Extended MHD Code

| Field | Detail |
|-------|--------|
| **Source** | CEA / EUROfusion |
| **Website** | [jorek.eu](https://jorek.eu/) |
| **Access** | Research collaboration required |

**Why it matters:** JOREK is the gold-standard for non-linear tearing mode and ELM simulations. Published benchmark data (2/1 tearing mode linear growth rates) can be used as validation targets for rusty-SUNDIALS reduced MHD.

**Published benchmark values (from literature):**
- Circular tokamak, aspect ratio R/a = 10, Lundquist S = 10⁴–10⁸
- 2/1 tearing mode linear growth rate γ vs. S
- Agreement with CASTOR3D linear code within 1–2%

---

### 8. ITER Scenario Database (restricted)
- Curated ITER plasma scenarios (15 MA baseline, hybrid, advanced)
- Accessible via OMAS + institutional credentials
- Contains coil currents, plasma shapes, heating profiles
- **Future v10 target** for federated auto-research (Component 6)

---

## 📊 Mapping to rusty-SUNDIALS v9/v10 Experiments

| rusty-SUNDIALS Module | Dataset | What it provides |
|----------------------|---------|-----------------|
| `tearing_mode_hero_test.rs` | TokaMark MHD tasks, JOREK benchmarks | Growth rates, mode structure validation |
| `fusion_mhd_benchmark.rs` | IMAS equilibrium IDS, FreeGS | Realistic ψ(r) initial conditions |
| `exp5_fogno_xmhd.rs` | FAIR-MAST diagnostics | B-field time series for comparison |
| `fusion_sciml_phase5.rs` | PlasmaPy parameters | Physical coefficients (η, ν, S) |
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

## References

1. TokaMark: UKAEA-IBM-STFC/tokamark-dataset (Hugging Face, 2025)
2. IMAS-Python: github.com/iterorganization/IMAS-Python (LGPL-3.0)
3. IMAS Data Dictionary: github.com/iterorganization/IMAS-Data-Dictionary
4. FreeGS: github.com/freegs-plasma/freegs (LGPL)
5. FreeGSNKE: github.com/FusionComputingLab/freegsnke
6. disruption-py: github.com/MIT-PSFC/disruption-py
7. PlasmaPy: github.com/PlasmaPy/PlasmaPy (BSD-3)
8. JOREK: jorek.eu + published benchmarks in Phys. Plasmas
9. FAIR-MAST: fair-mast.fusion.ukaea.uk
10. OMAS: github.com/gafusion/omas
