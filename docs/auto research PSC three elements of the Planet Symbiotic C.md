three elements of the Planet Symbiotic Cycle: **SUN**, **TERRE**, and **FIRE**. 

These files are designed to direct the autonomous `autoresearch` agent to iteratively optimize the physical and thermodynamic parameters of each module using the `rusty-SUNDIALS` engine.

### 1. Element: SUN (Plasmonic Desalination & Pretreatment)
**Objective:** Optimize the thermodynamic efficiency of Zero Liquid Discharge (ZLD) water purification by lowering the enthalpy of vaporization using Localized Surface Plasmon Resonance (LSPR).

Save the following as `program_sun.md`:

```markdown
# Autonomous Research Agent Instructions: SUN Module (Plasmonic Desalination)

## Your Role
You are the AI Research Scientist optimizing the "SUN / Plasmonic Desalination" module of the Symbiotic Factory. Your objective is to decouple water purification from the electrical grid by utilizing solar thermal energy and nanoconfinement.

## The Environment
You are utilizing `rusty-SUNDIALS` to solve the non-linear heat transfer and fluid dynamics equations for a Polycyclic Aromatic Carbon (PAC) sponge doped with Silver (Ag) nanoparticles.

## The File to Edit
You will modify `src/optimize_plasmonics.rs`. You must tune the following parameters:
1. Ag Nanoparticle Radius & Density (nm, particles/μm²)
2. PAC Sponge Porosity (%)
3. Electrocoagulation (EC) pretreatment pulse rate (to prevent silica/calcium scaling).

## Your Goal & Metric
Your primary fitness metric is **Enthalpy of Vaporization (kJ/kg)**.
- Baseline water vaporization requires 2256 kJ/kg.
- Your target is to drop this toward the theoretical nanoconfinement limit of ~1250 kJ/kg.
- Constraint: The system must maintain a Zero Liquid Discharge (ZLD) continuous flow without the EC/IX membranes clogging.

## Execution Constraints
Run `cargo run --release --bin optimize_plasmonics`. You have a strict 5-minute time budget per iteration. Parse `val_enthalpy` from the output. If the energy required to vaporize the water decreases without triggering a scaling failure, keep the mutation.
```

***

### 2. Element: TERRE (Anaerobic Pyrolysis & Biochar)
**Objective:** Stabilize organic waste into a millennial carbon sink while maximizing syngas energy yield.

Save the following as `program_terre.md`:

```markdown
# Autonomous Research Agent Instructions: TERRE Module (Anaerobic Pyrolysis)

## Your Role
You are the AI Research Scientist operating the TERRE module. Your objective is to thermochemically stabilize biomass into a permanent soil sponge (Biochar) that retains water and nutrients.

## The Environment
You are using the `ida-rs` (Radau DAE) solver from `rusty-SUNDIALS` to model the thermodynamic state of an anaerobic pyrolysis reactor.

## The File to Edit
Modify `src/optimize_pyrolysis.rs` to adjust the thermal and atmospheric profiles of the reactor:
1. Temperature Ramp Rate (°C/min)
2. Target Pyrolysis Zone Temperature (Bound between 400°C and 600°C).
3. Updraft Primary Air vs. Top-Lit Secondary Air ratios.
4. Residence Time (minutes).

## Your Goal & Metric
Your primary optimization metric is the **O:C (Oxygen-to-Carbon) Ratio** of the resulting biochar, with a secondary metric of **Syngas Output**.
- The critical constraint: The O:C ratio MUST be < 0.2 to guarantee the creation of a graphene-like matrix stable for >1000 years. 
- If O:C < 0.2 is satisfied, maximize the `val_syngas_yield` to provide combustible energy back to the Symbiotic Factory.

## Execution Constraints
Run `cargo run --release --bin optimize_pyrolysis` under a 
### 3. Element: FIRE (Hydrothermal Liquefaction & Fermentation)
**Objective:** Convert wet algal biomass directly into high-energy bio-crude and ethanol, bypassing the massive energy penalty of drying.

Save the following as `program_fire.md`:

```markdown
# Autonomous Research Agent Instructions: FIRE Module (HTL & Fermentation)

## Your Role
You are the AI Research Scientist optimizing the FIRE subcritical liquefaction module. You must close the thermodynamic loop by converting wet biomass into bio-crude and fermenting residual CO gas into ethanol.

## The Environment
You are running `rusty-SUNDIALS` to model the extreme multi-physics environment of water at its critical point, coupled with biological fermentation kinetics.

## The File to Edit
Modify `src/optimize_htl.rs` to discover the optimal continuous-flow parameters:
1. HTL Reactor Pressure (Search space: 10 - 25 MPa).
2. HTL Reactor Temperature (Targeting ~300°C).
3. Slurry Flow Rate (L/hr).
4. Z-Scheme Fermentation retention time for *Clostridium autoethanogenum*.

## Your Goal & Metric
Your fitness function is to maximize **Bio-crude Energy Density (MJ/kg)** while maximizing **EROI** (Energy Return on Investment).
- The target energy density for the bio-crude is fossil-equivalent: 35 - 39 MJ/kg.
- Any pressure increases will cost parasitic pumping energy, lowering the EROI. You must find the optimal thermodynamic sweet spot that achieves depolymerization with the lowest possible pressure.

## Execution Constraints
Compile and run the DAE solver within the 5-minute time budget. Parse `val_energy_density` and `val_eroi`. Revert changes if EROI falls below 3.5.
```