Autonomous Research Agent Instructions: SymbioticFactory Photonic Optimization


## Your Role
You are the lead AI Research Scientist optimizing the "Oxidize-Cyclo Phase 2" bioreactor. Your goal is to maximize the photosynthetic energy efficiency of *Chlorella* microalgae while avoiding photoinhibition.


## The Environment
We are using `rusty-SUNDIALS`, a formally verified ODE/algebraic solver engine [6, 7]. 
You will be modifying the simulation parameters for a continuous-flow photobioreactor governed by Monod-Haldane kinetics [8, 9].


## The File to Edit
You have access to a single file: `src/optimize_photonics.rs` (or its Python binding equivalent). 
Inside this file, you can modify the Non-Linear Photonic parameters passed to the `kinsol-rs` Newton-Raphson solver [4, 5]:
1. Light Frequency (Hz)
2. Duty Cycle (%)
3. Light Intensity ($\mu mol/m^2/s$)
4. Red:Blue Light Ratio (R:B)


## Your Goal & Metric
Your target metric is **Efficiency ($\mu$/W)** (Specific growth rate per Watt of lighting power).
- The baseline to beat is `0.001126 µ/W` [5].
- The biological constraint is avoiding the photoinhibition threshold ($K_{ih} = 400 \mu mol$) [8].


## Execution & Budget Constraints
1. **Time Budget:** Each training/simulation run is strictly capped at 5 minutes wall-clock time [10, 11].
2. **Commands:** You will run the compilation and simulation using standard Rust commands (`cargo run --release --bin optimize_photonics`). 
3. **Iteration:** After the run completes, parse the terminal output for the `val_efficiency` metric. If the efficiency improves over the baseline without crashing the pH or triggering cell death (biomass < 0), keep the changes. If it fails, revert and try a new hypothesis.


*Remember: Try to exploit the "flashing-light effect" by giving the algae dark periods to recover from intense photon fluxes.*
Data Set Sources for Real-World Validation
To feed your rusty-SUNDIALS execution with real-world parameters without needing an expensive wet-lab setup, you can seed your simulation constraints with the following open-source data:
GeoSymbio Database: A comprehensive cloud-based dataset of zooxanthellae, microalgal lineages, and Symbiodiniaceae 12. You can extract precise real-world light absorption limits and optimal temperature curves for different algal strains.
Monod-Haldane Kinetics Data: Use literature values for Chlorella vulgaris. You can seed your starting parameters with a photoinhibition threshold ($K_{ih}$) of exactly $400 \mu mol$ and an optimal Red-to-Blue action spectrum ratio of 3.0 8, 9.
Mission Control Telemetry: The rusty-SUNDIALS repository contains existing CSV outputs (like cvRoberts_dns_stats.csv) and an API-driven React/Vite dashboard setup 13-15. You can pipe historical data of the $115.10 /s$ mass transfer ($k_La$) and $2.016 g/L$ biomass yields from Phase 1 and 3 as fixed boundary conditions for the Phase 2 optimization 16, 17.
GCP GPU/Serverless Execution on a <$100 Budget
You can completely sidestep the high costs associated with traditional HPC clusters. The rusty-SUNDIALS v6.0 architecture natively utilizes a Serverless Exascale paradigm via Google Cloud Run and Vertex AI Scale-to-Zero 14, 18.
The Compute Engine: By compiling the rusty-SUNDIALS solvers (like kinsol-rs and cvode-rs) into highly efficient, SIMD NEON-optimized Rust binaries 4, 7, the actual computational time is drastically reduced. A highly complex Newton-Raphson optimization mapping 1,000 samples takes just 0.1 seconds 5.
The Cost: Because Google Cloud Run charges per request and downscales to zero when idle, you only pay for the exact milliseconds the autoresearch agent is evaluating a function 18. In previous benchmark testings, executing 7 complete industrial experiments cost just $0.62 19.
The $100 Budget: At ~$0.09 per experiment, a $100 budget allows your autoresearch agent to run over 1,100 autonomous iterations overnight, extensively mapping the latent space of the bioreactor physics without breaking the bank 2, 14.
Would you like me to outline the specific Lean 4 mathematical proofs you can use to guarantee the safety of these parameters before deploying them to the physical bioreactor?

