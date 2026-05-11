# Scientific Use Cases: Rusty-SUNDIALS Benchmark Suite

This document catalogues the **10 complex scientific examples** implemented in
Rusty-SUNDIALS, spanning six major scientific domains. Each benchmark is drawn
from peer-reviewed academic literature and represents a canonical test case for
ODE integrators in the SUNDIALS ecosystem.

---

## 1. Lorenz Attractor — Deterministic Chaos
**File:** `examples/lorenz.rs`  
**Domain:** Atmospheric science, chaos theory  
**Reference:** Lorenz, E.N. (1963). *J. Atmos. Sci.* 20(2), 130–141.

The three coupled ODEs model thermal convection rolls in the atmosphere.
With σ=10, ρ=28, β=8/3, the system exhibits the famous butterfly-shaped
strange attractor — the founding example of deterministic chaos.

---

## 2. Hodgkin-Huxley Neuron — Nobel Prize 1963
**File:** `examples/hodgkin_huxley.rs`  
**Domain:** Computational neuroscience  
**Reference:** Hodgkin, A.L. & Huxley, A.F. (1952). *J. Physiol.* 117(4), 500–544.

Four stiff coupled ODEs model action potential generation in the squid giant
axon: membrane voltage V and three ion-channel gating variables (m, h, n).
The exponential rate functions create extreme stiffness requiring implicit BDF.

---

## 3. SIR Epidemic Model
**File:** `examples/sir_epidemic.rs`  
**Domain:** Mathematical epidemiology  
**Reference:** Kermack, W.O. & McKendrick, A.G. (1927). *Proc. R. Soc. London A* 115(772), 700–721.

The foundational compartmental model of epidemic spread. With R₀=β/γ=3.0,
the simulation tracks the classic S-shaped infection curve and herd immunity
threshold.

---

## 4. Lotka-Volterra Predator-Prey
**File:** `examples/lotka_volterra.rs`  
**Domain:** Ecology, population dynamics  
**Reference:** Lotka, A.J. (1925). *Elements of Physical Biology.*  
Volterra, V. (1926). *Mem. Accad. Lincei.*

Tests conservation of the Hamiltonian H = δx − γ ln(x) + βy − αln(y).
The solver's symplectic drift is monitored as ΔH/H₀ over long integration.

---

## 5. HIRES — High Irradiance RESponse
**File:** `examples/hires.rs`  
**Domain:** Plant biology, photochemistry  
**Reference:** Hairer & Wanner, *Solving ODEs II*, §IV.10.

Eight-component stiff chemical kinetics system from the Hairer-Wanner standard
test set. Models light-driven signalling cascades in plant cells. Includes a
mass conservation check (Σyᵢ = 1.0057).

---

## 6. Double Pendulum — Chaotic Mechanics
**File:** `examples/double_pendulum.rs`  
**Domain:** Classical mechanics, nonlinear dynamics  
**Reference:** Shinbrot, T. et al. (1992). *Am. J. Phys.* 60(6), 491–499.

A paradigmatic system exhibiting sensitive dependence on initial conditions.
The Lagrangian-derived equations of motion produce four coupled ODEs.

---

## 7. Euler Rigid Body Equations
**File:** `examples/rigid_body.rs`  
**Domain:** Classical mechanics, spacecraft dynamics  
**Reference:** Hairer, Nørsett & Wanner, *Solving ODEs I*, §I.14.

Tests conservation of both kinetic energy E and angular momentum magnitude L
under torque-free rotation with asymmetric moments of inertia (I₁=0.5, I₂=1, I₃=2).

---

## 8. Rössler Attractor
**File:** `examples/rossler.rs`  
**Domain:** Nonlinear dynamics  
**Reference:** Rössler, O.E. (1976). *Phys. Lett. A* 57(5), 397–398.

A simpler chaotic attractor than Lorenz, with only one nonlinear term (xz).
Standard parameters a=0.2, b=0.2, c=5.7 produce a folded-band attractor with
period-doubling route to chaos.

---

## 9. FitzHugh-Nagumo — Simplified Neuron
**File:** `examples/fitzhugh_nagumo.rs`  
**Domain:** Mathematical biology  
**Reference:** FitzHugh, R. (1961). *Biophys. J.* 1(6), 445–466.  
Nagumo, J. et al. (1962). *Proc. IRE* 50(10), 2061–2070.

A two-variable reduction of the Hodgkin-Huxley model capturing excitability
and oscillation. The cubic nullcline produces relaxation oscillations visible
in the phase plane.

---

## 10. Three-Body Gravitational Problem
**File:** `examples/three_body.rs`  
**Domain:** Celestial mechanics  
**Reference:** Hairer et al., *Solving ODEs I*, §I.16.

Twelve coupled ODEs (6 positions + 6 velocities) for three equal-mass bodies
interacting via Newtonian gravity. Initialized near the Lagrange equilateral
triangle configuration with a small velocity perturbation.

---

## Summary Table

| # | Example | Domain | N_eq | Method | Stiff? |
|---|---------|--------|------|--------|--------|
| 1 | Lorenz | Chaos theory | 3 | BDF | Mild |
| 2 | Hodgkin-Huxley | Neuroscience | 4 | BDF | Yes |
| 3 | SIR Epidemic | Epidemiology | 3 | Adams | No |
| 4 | Lotka-Volterra | Ecology | 2 | Adams | No |
| 5 | HIRES | Photochemistry | 8 | BDF | Yes |
| 6 | Double Pendulum | Mechanics | 4 | BDF | Mild |
| 7 | Rigid Body | Mechanics | 3 | Adams | No |
| 8 | Rössler | Chaos theory | 3 | BDF | Mild |
| 9 | FitzHugh-Nagumo | Neuroscience | 2 | Adams | No |
| 10 | Three-Body | Celestial mech. | 12 | Adams | No |
