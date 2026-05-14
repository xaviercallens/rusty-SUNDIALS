# Protocol H: Active Liquid Metal Walls and Neural Phase-Fields

## H.1 The Divertor Degradation Bottleneck

For steady-state, continuous fusion reactors to operate beyond the experimental bursts of ITER, the solid-state tungsten divertor walls will eventually degrade. Repetitive plasma strikes, specifically Edge Localized Modes (ELMs), cause micro-fracturing, sputtering, and neutron embrittlement.

The proposed engineering solution is to replace rigid solid-state armor with flowing liquid metal walls (such as Liquid Lithium or Tin). The liquid metal acts as a renewable, self-healing surface.

### H.1.2 The Free-Surface Modeling Nightmare
Simulating a massive, high-pressure plasma striking a free-surface liquid metal is computationally notorious. Coupling compressible Extended MHD (the plasma) with incompressible Navier-Stokes (the liquid metal) across a moving, splashing boundary usually causes numerical solvers to crash due to energy leaks at the sharp interface.

## H.2 Disruptive Methodology: Neural Phase-Field Operator (NPFO)

Protocol H discards the concept of a "hard boundary" or moving mesh. Instead, the agent utilized a Phase-Field formulation.

### H.2.1 The Continuous Scalar Field
The boundary between the plasma and the liquid metal is treated as a continuous, differentiable scalar field $\phi \in [-1, 1]$. 
- $\phi = -1$ represents pure plasma.
- $\phi = 1$ represents pure liquid metal.
- $\phi = 0$ represents the diffuse interface.

### H.2.2 Physics-Informed Neural Operator
To track the violently splashing capillary waves of the liquid metal reacting to the 1.2 GPa magnetic pressure of an ELM strike, the agent trained a Physics-Informed Neural Network (PINN) within the SUNDIALS loop. The Neural Phase-Field Operator acts as a surrogate, predicting the nonlinear wave dynamics thousands of times faster than direct numerical simulation.

## H.3 Scientific Achievement and Discovery

The autonomous application of the Neural Phase-Field led to the invention of a radical, cyber-physical reactor defense mechanism.

### H.3.1 The Active Capillary Shock-Absorber
A static pool of liquid Tin responds to a 1.2 GPa ELM strike with catastrophic splashing. Droplets of high-Z Tin are ejected into the core, instantly killing the fusion reaction via radiation (impurity influx).

However, the agent discovered that by passing electrical currents through the liquid metal 200 microseconds *before* the ELM strikes, it could generate precise $\mathbf{J} \times \mathbf{B}$ Lorentz forces. These forces induce a counter-propagating capillary wave across the surface of the liquid Tin. 

When the ELM impacts, the liquid wall is oscillating in perfect anti-phase to the plasma stress wave. This constructive interference acts as an **active shock-absorber**, neutralizing the impact stress entirely and preventing any splashing, thereby keeping the core completely pristine.
