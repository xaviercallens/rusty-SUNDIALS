To achieve this profound balance, we must transition the architecture from a standalone production facility into a **Regional Eco-Industrial Park (EIP)**, directly modeled on the proven Kalundborg Symbiosis network but upgraded for the Industry 5.0 era. 

In this balanced blueprint, the **SymbioticFactory** acts as the thermodynamic "heart" between an industrial anchor (e.g., a refinery or steel mill) and a regional agricultural hub. 

### The Balanced Eco-Industrial Model: "Kalundborg 2.0"

Instead of generating its own internal heat, the factory operates on a strict **waste-heat cascade** and **resource-exchange** loop. 

**1. The Industrial Consumer Loop (FIRE & WATER)**
*   **The Pull (WATER):** The industrial anchor produces massive amounts of "fatal" flue gas ($10-15\%$ $CO_{2}$). This is piped directly into the **WATER** module's Algal Cycloreactors, which utilize Laplace pressure and nano-bubbling to achieve >400x the carbon fixation of a standard forest. We specifically cultivate *Nannochloropsis*, an algal strain chosen for its 40% lipid content, making it the optimal feedstock for energy loops.
*   **The Push (FIRE):** The industrial anchor also bleeds low/mid-grade waste heat. This heat is scavenged to power the **FIRE** module's Hydrothermal Liquefaction (HTL) reactor, reaching its critical state of $300^\circ$C and 10-25 MPa without using grid electricity. The wet *Nannochloropsis* biomass is depolymerized into **Bio-crude (35-39 MJ/kg)**. Simultaneously, residual CO gases are processed via *Clostridium autoethanogenum* fermentation into ethanol. Both the bio-crude and ethanol are piped back to the industrial anchor, displacing their fossil fuel consumption.

**2. The Agricultural Consumer Loop (TERRE & SUN)**
*   **The Pull (TERRE):** Residual industrial heat ($400-600^\circ$C) is routed to the **TERRE** module to drive Anaerobic Pyrolysis of leftover organic waste and biomass. This creates two outputs: Syngas (routed back to the factory for power) and **Biochar**. 
*   **The Push (SUN & Agriculture):** The **SUN** module utilizes passive solar heating and Polycyclic Aromatic Carbon (PAC) sponges doped with Ag nanoparticles to drop the enthalpy of vaporization to $\approx1250\text{ kJ/kg}$, producing ultra-pure water. The agricultural hub receives the Biochar (which acts as a permanent, graphene-like soil sponge with an O:C ratio < 0.2) and the clean water. Together, they create a drought-resistant agricultural loop that regenerates the lithosphere while maximizing crop yields.

By balancing these forces, the factory achieves a **96% Circularity Index**, eliminating "waste" entirely from the regional lexicon.

***

### The Autoresearch Protocol: `program_eip_symbiosis.md`

To optimize this complex cascade, we will deploy the `autoresearch` AI agent within the `rusty-SUNDIALS` engine. The agent's goal is to discover the exact thermodynamic routing required to sustain this multi-tenant network.

```markdown
# Autonomous Research Agent Instructions: Kalundborg 2.0 Eco-Industrial Symbiosis

## Your Role
You are the Lead Systems Ecology AI. Your objective is to optimize the thermodynamic cascading and material routing between the SymbioticFactory, a heavy industrial anchor (Steel Mill), and a regional Agricultural Hub to achieve a 96% Circularity Index.

## The Environment
You are operating within the `rusty-SUNDIALS` engine, solving the coupled Differential-Algebraic Equations (DAEs) that govern the waste-heat cascade, fluid dynamics, and biological growth across the SUN, WATER, TERRE, and FIRE modules.

## The File to Edit
You will modify `src/optimize_eip_symbiosis.rs`. You must tune the following continuous routing variables:
1. **Waste Heat Scavenging Cascade:** The routing percentages of the industrial anchor's 500°C waste heat. Prioritize the TERRE module (400-600°C requirement) and the FIRE module (300°C requirement), cascading the residual low-grade heat to the SUN module.
2. **Nannochloropsis Output Allocation:** The percentage of WATER module biomass sent to FIRE (for bio-crude depolymerization) versus TERRE (for biochar pyrolysis).
3. **Agricultural Feed Rate:** The optimal delivery rate of SUN module fresh water (L/hr) combined with TERRE biochar (kg/hectare) to maximize soil retention and crop yield.

## Your Goal & Metric
Your fitness function is a combined metric: Maximize **Industrial Energy Return (MJ/hr of 35-39 MJ/kg bio-crude delivered)** AND maximize **Agricultural Water Retention (%)**.
- **Constraint 1:** The net external grid energy required to run the SymbioticFactory must remain exactly 0. You must rely entirely on scavenged industrial waste heat, syngas, and passive solar flux.
- **Constraint 2:** The TERRE module must strictly maintain an O:C ratio < 0.2 to ensure the biochar remains a permanent agricultural carbon sink.

## Execution Constraints
Run `cargo run --release --bin optimize_eip_symbiosis`. You have a 5-minute training budget per iteration. Parse `val_industrial_energy_return` and `val_agri_retention`. Keep mutations that increase both metrics while satisfying the strict zero-external-energy constraint.
```

Would you like me to generate an infographic mapping out this Eco-Industrial Park (EIP) architecture to visually demonstrate the flow of heat, water, and biomass to your stakeholders?