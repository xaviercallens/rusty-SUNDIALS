# P2 gate G1: falsification of QU-1 (harmonic zero-point energy, planar Bravais lattices)

Tier X (floating point) throughout. Nothing here is a proof. Date 2026-10-25. Prior (preregistration.json) that QU-1 survives: 0.6.

## Verdict

**REFUTED(gauss:1, tau = 0.5 + 0.966025 i, margin dE_zp = -1.31e-2).** QU-1 as stated (triangular minimises E_zp among unit-covolume planar Bravais lattices for every completely monotone potential) is false numerically: for the Gaussian g(r^2) = exp(-pi r^2), which is completely monotone in r^2 and is the Montgomery case, the rhombic lattice tau = 0.5 + 0.966025 i is dynamically stable (no imaginary frequency on k-grids n = 96, 192, 384) and has E_zp = 1.4902783 against 1.5033523 for the triangular lattice (grids n = 384, same code); the k-grid error at n = 96 is 1.3e-6, so the margin is about 10^4 times the numerical error. The largest margin found (dense arc scan, stable at n = 384) is dE_zp = -5.12e-2 (3.4 % of E_zp) at tau = 0.295 + 0.9555 i; the sampled-grid value was -4.87e-2 at tau = 0.3 + 0.9539 i. The elastic (k -> 0) limit of the headline competitor is stable: min eigenvalue of D(k)/|k|^2 = 0.105 against 0.238 for the triangular lattice (independent of any k-grid). The result was reproduced by an independent numpy implementation (explicit real-space sum, no folding, Gamma-centred and half-shifted k-grids, section Controls).

Refuted for 8 of 12 tested potentials by a stable competitor with lower E_zp: `dipolar:0.1`, `dipolar:0.5`, `gauss:0.5`, `gauss:1`, `riesz:3`, `riesz:4`, `yukawa:1`, `yukawa:2`. Not refuted (no stable lower lattice found on the 567-point scan nor on dense 0.005-step scans along the arc |tau| = 1 and the line Re tau = 1/2; triangular is a strict local minimum of E_zp): `gauss:0.25`, `gauss:2`, `gauss:4`, `riesz:6`. The conjecture 'for every CM g' is therefore refuted; a restricted version (narrow potentials) is not contradicted by this scan but the stable set there is small, so this is evidence from two 1-D paths plus a sparse 2-D sample, not a confirmation.

Mechanism seen in the data (interpretation, Tier C, not tested separately): around the triangular point the static energy has positive curvature (Montgomery, consistent with the scan) while the harmonic zero-point energy has isotropic curvature of sign that depends on the potential; where it is negative, E_static + Lambda E_zp has a negative-curvature (shear-softened) triangular point for Lambda above a threshold. The triangular lattice minimises E_static + Lambda E_zp over the stable sample only for the potentials and Lambda marked in the coupling table.

## What was computed

- Lattice shape tau = x + i y (|tau| >= 1, 0 <= x <= 1/2, y <= 2.4 scanned; E_zp is invariant under tau -> tau + 1 and tau -> -1/tau, tested), basis a1 = (1/sqrt y, 0), a2 = (x/sqrt y, sqrt y), covolume 1.
- D(k) = sum_{R != 0} (1 - cos k.R) Hess V(R); E_zp = (1/2) <sum over 2 branches sqrt(eig D(k))> over the Brillouin zone (hbar = m = 1, V with unit amplitude; the sign of dE_zp does not depend on units).
- Potentials (all radial, V = g(r^2)): Gaussian exp(-pi a r^2), a in {0.25, 0.5, 1, 2, 4}; Yukawa exp(-kappa r)/r, kappa in {1, 2}; dipolar exciton U = V/d^2 = 2/(r s (r + s)), s = sqrt(r^2 + d^2), d in {0.1, 0.5} (untruncated, 1/r^3 tail); Riesz r^-s, s in {3, 4, 6}.
- **Excluded: Coulomb (Riesz s = 1) and Keldysh.** They need a neutralising background and plasmon handling of the k -> 0 limit, which this code does not implement. No claim is made for them.
- **Unstable lattices are never clipped.** A lattice with an eigenvalue of D(k) below -1e-9 x scale at any grid point is dynamically unstable and its E_zp is reported as NaN and excluded from the comparison. This policy is not in the preregistration; it is a scope decision made here because most non-triangular lattices are unstable (table below). A competitor-favouring clipped value E_zp,clip (imaginary frequencies set to 0) is reported separately and is never used to claim a minimum.
- Method: k = (i/n) b1 + (j/n) b2 (Gamma-centred; k = 0 is in the average and contributes zero energy); k.R = 2 pi (i m + j l)/n so H(R) is folded into n x n cells and D(k) for all k is a separable transform, O(n^3). Sum over |R| < rc = 400 (natural cutoff for exponentially decaying potentials) completed by the continuum tail -pi rc V'(rc) I. No Ewald is needed for D because Hess V decays like r^(-s-2).
- Static energy: (1/2) sum_{R != 0} V(R) with the uniform-continuum tail pi rc^(2-s)/(s-2) (Riesz) or the exact dipolar tail; compared with Epstein closed forms and the Ewald sum below.

## Controls

**Positive control 1, dynamical matrix against an independent energy (frozen phonon).** For 33 cases (Gaussian, Yukawa, Riesz s = 3, 4, 6, Lennard-Jones, dipolar d = 0.1, 0.5 at several shapes, k and polarisations) the second difference [E(eps) + E(-eps) - 2E(0)]/eps^2 of the pair-sum energy of a 5 x 5 supercell with displacements eps e cos(k.R) (only values of V, no derivatives; for the dipolar potential the Ewald supercell energy of `dipolar_ewald.rs`) equals (1/2) e^T D(k) e. Maximum relative difference over all cases: **2.136e-07**.

**Positive control 2, static Riesz energies against Epstein-zeta closed forms** (4 zeta(s/2) beta(s/2) for the square lattice, 6 zeta(s/2) L(s/2, chi_-3) for the hexagonal lattice, mpmath, rescaled to covolume 1; energy per particle = half the lattice sum):

| s | lattice | E exact | E code (rc=400 + tail) | diff |
|---|---|---|---|---|
| 3 | square | 4.516810841550 | 4.516810452130 | -3.894e-07 |
| 3 | hexagonal | 4.446372320187 | 4.446372567382 | 2.472e-07 |
| 4 | square | 3.013406019846 | 3.013406018872 | -9.737e-10 |
| 4 | hexagonal | 2.891679649839 | 2.891679649882 | 4.253e-11 |
| 6 | square | 2.329456807802 | 2.329456807799 | -2.940e-12 |
| 6 | hexagonal | 2.070628273602 | 2.070628273599 | -3.094e-12 |

The s = 3 residual (about 4e-7) is the lattice-point truncation residual of the plain cutoff, as documented in `dipolar_ewald.rs`.

**Positive control 3, dipolar static energy against the Ewald sum** (`DipolarEwald`, U units): max |diff| = 3.894e-07 over d in {0.1, 0.5} and three shapes.

**Cutoff convergence.** E_zp at rc = 100, 200, 400, 800 for Riesz 3, 6, dipolar 0.1, 0.5, Yukawa 1, Gaussian 0.5 at the triangular point and at tau = 0.45 + 0.93 i: the relative change from rc = 400 to 800 is at most 2.191e-13 (see controls.json).

**Modular invariance.** E_zp and E_static at tau, tau + 1 and -1/tau (different bases, different folding) agree to about 1e-15 on the same k-set (the set of grid points modulo reciprocal lattice is basis independent; this tests the implementation, not the grid quality), `controls.json: modular_invariance`; the Rust unit test also checks static energies against theta functions (square: theta_3^2; triangular: self-duality theta_L(a) = theta_L(1/a)/a).

**Independent implementation (numpy, explicit sum over R, no folding; Gamma-centred n = 128 and half-shifted n = 254 grids):**

| potential | tau | E_zp (n=128, centred) | E_zp (n=254, shifted) |
|---|---|---|---|
| gauss:1.0 | (0.5000, 0.8660) | 1.5033518 | 1.5033524 |
| gauss:1.0 | (0.5000, 0.9660) | 1.4902778 | 1.4902783 |
| gauss:1.0 | (0.3000, 0.9539) | 1.4546847 | 1.4546850 |
| gauss:1.0 | (0.5500, 0.8660) | 1.5001432 | 1.5001437 |
| gauss:1.0 | (0.4500, 0.8160) | 1.4960026 | 1.4960032 |
| yukawa:1.0 | (0.5000, 0.8660) | 1.7658974 | 1.7658984 |
| yukawa:1.0 | (0.5000, 0.9660) | 1.7603874 | 1.7603884 |
| yukawa:1.0 | (0.3000, 0.9539) | 1.7426547 | 1.7426554 |

Rust values at the triangular point: gauss:1: extrapolated 1.5033524 (n = 96: 1.5033511); yukawa:1: extrapolated 1.7658983 (n = 96: 1.7658962). The two codes agree to 1e-6 (the k-grid error).

**k-grid convergence.** E_zp(n) - E_zp(inf) is proportional to n^-p with fitted p = 3.00, 3.00, 3.01, 3.00, 3.00, 3.00, 3.00, 3.00, 3.00, 3.00, 3.00, 3.00 (n = 48, 96, 192, triangular point, one value per potential in the table order below); p = 3 is the expected rate for a conical singularity at Gamma in 2D. The n = 96 error at the triangular point is between 2.632e-08 and 6.452e-06 (extrapolated values are used for the curvature table; n = 96 values for the global scan).

**Positive control 4, static energy (Tier L consistency).** For all 12 completely monotone potentials the static minimiser of the scan (567 lattices at n = 96) is the triangular lattice (column 'min dE_static' is 0 with the minimiser at tau = rho), as Montgomery's theorem (Glasgow Math. J. 30 (1988) 75-85, doi:10.1017/S0017089500007047) and its Bernstein-mixture extensions predict; a violation would have signalled a code bug.

## Negative control (non-completely-monotone potentials)

The search must be able to find a counterexample, otherwise 'survives' is meaningless. Family `wells`: V = -ring(r1, w) - c ring(r2, w) with ring(r0, w) = exp(-(r - r0)^2/(2 w^2)), r1 = 1 and r2 = sqrt 2 (the square lattice's first two neighbour shells), scanned over w in {0.08, 0.12, 0.2, 0.3}, c in {0, 0.5, 1} on the coarse domain grid at n = 48 (`neg_search.json`; nothing tuned beyond that grid). Two members were taken to the full scan, together with Lennard-Jones (sigma = 0.9), Morse and a Gaussian difference:

| potential | tri stable | stable/points | min dE_static (any lattice) | min dE_zp over stable competitors |
|---|---|---|---|---|
| gdiff:0.3;1.5;0.5 | False | 0/567 | -2.396e-02 at (0.021, 2.137) | none stable |
| lj:0.9 | True | 5/567 | 0.000e+00 at (0.500, 0.866) | 1.262e-01 at (0.450, 0.893) |
| morse:3;1.0 | True | 5/567 | 0.000e+00 at (0.500, 0.866) | -2.043e-02 at (0.500, 0.966) |
| wells:1.0;1.414213562373;0.12;0.5 | True | 28/567 | -5.057e-01 at (0.000, 1.000) | -3.032e+00 at (0.150, 1.189) |
| wells:1.0;1.414213562373;0.2;0.0 | True | 22/567 | 0.000e+00 at (0.500, 0.866) | -2.040e+00 at (0.441, 1.119) |

`wells:1.0;1.414213562373;0.12;0.5`: the square lattice is a stable static minimiser lower than the triangular one by 0.506 per particle AND a stable rhombic lattice has E_zp lower by 3.03; `wells:...;0.2;0.0`: the triangular lattice is the static minimiser but a stable lattice has E_zp lower by 2.04 (static and zero-point optima need not coincide). `lj:0.9` behaves like the survivors (triangular lowest in both), `gdiff` has an unstable triangular lattice (E_zp undefined). Morse also shows a stable lower-E_zp competitor. So the pipeline does find counterexamples when they exist; the CM refutations above are therefore not a defect of the stable-set bookkeeping.

## Verification at the stability edge and dense edge scans (`phonon_edge_check.py`, `phonon_longwave_check.py`)

Stability on an n = 96 k-grid is optimistic near the stability boundary: soft long-wave modes need a finer grid to be seen. Every competitor used for a verdict below was therefore recomputed on n = 96, 192, 384, and the two candidate points also at rc = 400 and 800, and it counts only if stable on all of them. Dense 1-D scans (step 0.005, n = 96) were run along the arc |tau| = 1 from x = 0.5 to 0.2 and along Re tau = 1/2 from y = sqrt(3)/2 to 1.1; the lowest stable point of each path was refined at n = 192, 384.

| potential | robust min dE_zp (n=384) | at tau | verdict | points stable at n=96 but unstable on a finer grid (dropped) |
|---|---|---|---|---|
| dipolar:0.1 | -3.3437e-02 | (0.3000, 0.9539) candidate | REFUTED | none |
| dipolar:0.5 | -4.0679e-02 | (0.3000, 0.9539) candidate | REFUTED | none |
| gauss:0.25 | +6.2302e-06 | (0.5000, 0.8710) dense line | survives (dense edge scan) | none |
| gauss:0.5 | -1.4588e-02 | (0.5000, 1.0160) dense line | REFUTED | (0.300, 0.9539): n=96 stable, unstable at n=192/384; (0.300, 0.9539): dense arc minimum, unstable at n=192/384 |
| gauss:1 | -5.1230e-02 | (0.2950, 0.9555) dense arc | REFUTED | none |
| gauss:2 | +1.6293e-05 | (0.5000, 0.8710) dense line | survives (dense edge scan) | none |
| gauss:4 | +1.0391e-05 | (0.5000, 0.8710) dense line | survives (dense edge scan) | none |
| riesz:3 | -3.2737e-02 | (0.3000, 0.9539) candidate | REFUTED | (0.500, 1.0360): dense line minimum, unstable at n=192/384 |
| riesz:4 | -1.2380e-02 | (0.3000, 0.9539) candidate | REFUTED | none |
| riesz:6 | +1.1337e-04 | (0.5000, 0.8710) dense line | survives (dense edge scan) | none |
| yukawa:1 | -2.3243e-02 | (0.3000, 0.9539) candidate | REFUTED | (0.295, 0.9555): dense arc minimum, unstable at n=192/384 |
| yukawa:2 | -1.9717e-02 | (0.3000, 0.9539) candidate | REFUTED | none |

Elastic-limit check (explicit sum, k = eps (cos t, sin t), 721 directions): min over t of lambda_min(D)/eps^2, positive = stable at long wavelength, independent of the k-grid. The triangular lattice (y = 0.8660) is the reference.

| potential | tau | eps = 1e-3 | eps = 1e-2 |
|---|---|---|---|
| gauss:1.0 | (0.500, 0.9660) | +0.1047 | +0.1047 |
| gauss:1.0 | (0.500, 1.0160) | +0.0408 | +0.0408 |
| gauss:1.0 | (0.500, 1.0360) | +0.0160 | +0.0160 |
| gauss:1.0 | (0.500, 0.8660) | +0.2383 | +0.2383 |
| gauss:0.5 | (0.500, 0.9660) | +0.0269 | +0.0269 |
| gauss:0.5 | (0.500, 1.0160) | +0.0014 | +0.0014 |
| gauss:0.5 | (0.500, 1.0360) | -0.0087 | -0.0087 |
| gauss:0.5 | (0.500, 0.8660) | +0.0808 | +0.0808 |
| yukawa:1.0 | (0.500, 0.9660) | +0.0909 | +0.0909 |
| yukawa:1.0 | (0.500, 1.0160) | +0.0270 | +0.0270 |
| yukawa:1.0 | (0.500, 1.0360) | +0.0020 | +0.0020 |
| yukawa:1.0 | (0.500, 0.8660) | +0.2250 | +0.2250 |
| yukawa:2.0 | (0.500, 0.9660) | +0.0710 | +0.0710 |
| yukawa:2.0 | (0.500, 1.0160) | +0.0212 | +0.0212 |
| yukawa:2.0 | (0.500, 1.0360) | +0.0018 | +0.0018 |
| yukawa:2.0 | (0.500, 0.8660) | +0.1753 | +0.1753 |

The long-wave values explain the edge: at gauss:0.5 and tau = 0.5 + 1.036 i a sound-speed eigenvalue is negative (-0.0087), so that lattice is elastically unstable, and tau = 0.5 + 1.016 i is only just stable (0.0014); the gauss:0.5 verdict does not rest on that point (the lattice 0.5 + 0.966 i has 0.027 > 0 and dE_zp = -6.31e-3 at n = 384). The long-wave check covers Gaussian and Yukawa only; for the dipolar and Riesz potentials stability rests on the k-grids up to n = 384.

## Results for the completely monotone potentials (n = 96, rc = 400; 167 coarse + 400 random shapes, all with 0 <= Re tau <= 1/2, y in [0.85, 2.4])

| potential | tri stable | stable / sampled | E_zp(tri) extrap. | min dE_zp over the n=96-stable sample (not tri; see edge verification for the robust values) | at tau | verdict | min dE_zp,clip (all, favours competitor) |
|---|---|---|---|---|---|---|---|
| dipolar:0.1 | True | 15/567 | 4.2096272 | -3.343e-02 | (0.300, 0.954) | REFUTED | -2.23e-01 (unstable) |
| dipolar:0.5 | True | 15/567 | 3.5284909 | -4.068e-02 | (0.300, 0.954) | REFUTED | -2.45e-01 (unstable) |
| gauss:0.25 | True | 11/567 | 0.9619934 | +7.653e-04 | (0.450, 0.893) | survives (dense edge scan) | -1.16e-01 (unstable) |
| gauss:0.5 | True | 13/567 | 1.5062895 | -2.178e-02 | (0.300, 0.954) | REFUTED | -5.06e-01 (unstable) |
| gauss:1 | True | 16/567 | 1.5033524 | -4.867e-02 | (0.300, 0.954) | REFUTED | -3.64e-01 (unstable) |
| gauss:2 | True | 11/567 | 0.5419325 | +2.124e-03 | (0.450, 0.893) | survives (dense edge scan) | -7.80e-03 (unstable) |
| gauss:4 | True | 6/567 | 0.0300814 | +1.226e-03 | (0.450, 0.893) | survives (dense edge scan) | +0.00e+00 (stable) |
| riesz:3 | True | 15/567 | 4.2449757 | -3.273e-02 | (0.300, 0.954) | REFUTED | -2.20e-01 (unstable) |
| riesz:4 | True | 14/567 | 5.3229783 | -1.238e-02 | (0.300, 0.954) | REFUTED | -1.86e-01 (unstable) |
| riesz:6 | True | 12/567 | 7.3147543 | +1.356e-02 | (0.450, 0.893) | survives (dense edge scan) | -2.45e-02 (unstable) |
| yukawa:1 | True | 15/567 | 1.7658983 | -2.324e-02 | (0.300, 0.954) | REFUTED | -1.43e-01 (unstable) |
| yukawa:2 | True | 15/567 | 1.4592747 | -1.972e-02 | (0.300, 0.954) | REFUTED | -1.17e-01 (unstable) |

The square lattice tau = i is dynamically unstable for every potential here (negative eigenvalue of D(k)), and the stable region is a small neighbourhood of the triangular point (all stable sampled lattices lie within 0.23 of rho in tau, with Im tau <= 1.03 and Re tau >= 0.30; measured by phonon_stable_extent.py on the scan CSVs). dE_zp,clip is lower than the triangular value for unstable lattices for most potentials (imaginary modes contribute 0), so clipping would manufacture counterexamples; those are not used.

### Curvature of E_zp at the triangular point (k-grid extrapolated n = 48, 96, 192)

c(r) = [E_zp(tau) - E_zp(rho)] / (r^2 / y_rho^2), r = Euclidean distance |tau - rho|, 12 directions per ring; the modular group makes rho a critical point with isotropic leading curvature (spread over angles = cubic terms). Ring entries that are dynamically unstable are dropped and counted.

| potential | c(r=0.01) [min,max over 12 angles] | c(r=0.05) | triangular point |
|---|---|---|---|
| dipolar:0.1 | [-0.335, -0.316] | [-0.401, -0.299] | local MAX of E_zp (refuted locally) |
| dipolar:0.5 | [-0.607, -0.582] | [-0.679, -0.549] | local MAX of E_zp (refuted locally) |
| gauss:0.25 | [+0.186, +0.190] | [+0.174, +0.195] | local MIN of E_zp |
| gauss:0.5 | [-0.469, -0.452] | [-0.508, -0.421] | local MAX of E_zp (refuted locally) |
| gauss:1 | [-0.979, -0.945] | [-1.056, -0.885] | local MAX of E_zp (refuted locally) |
| gauss:2 | [+0.483, +0.505] | [+0.428, +0.540] | local MIN of E_zp |
| gauss:4 | [+0.308, +0.316] | [+0.288, +0.326] | local MIN of E_zp |
| riesz:3 | [-0.313, -0.294] | [-0.378, -0.279] | local MAX of E_zp (refuted locally) |
| riesz:4 | [+0.338, +0.356] | [+0.270, +0.357] | local MIN of E_zp |
| riesz:6 | [+3.378, +3.446] | [+3.181, +3.507] | local MIN of E_zp |
| yukawa:1 | [-0.395, -0.381] | [-0.432, -0.359] | local MAX of E_zp (refuted locally) |
| yukawa:2 | [-0.324, -0.312] | [-0.356, -0.294] | local MAX of E_zp (refuted locally) |

### E_static + Lambda E_zp

With V -> eps V, E_static scales as eps and E_zp as sqrt(eps), so the physically meaningful family is E_static + Lambda E_zp with Lambda proportional to eps^(-1/2) (hbar^2/m relative to the interaction scale); the harmonic approximation fails for large Lambda (strong quantum motion, melting) so large-Lambda rows are a statement about the functional only. Entries are the minimum over the stable sample of E_total(tau) - E_total(rho); 0 means the triangular lattice is the minimiser.

| potential | Lambda=0.1 | Lambda=0.3 | Lambda=1.0 | Lambda=3.0 | Lambda=10.0 |
|---|---|---|---|---|---|
| dipolar:0.1 | 0 (tri) | 0 (tri) | -2.47e-03 at (0.30,0.95) | -6.93e-02 at (0.30,0.95) | -3.03e-01 at (0.30,0.95) |
| dipolar:0.5 | 0 (tri) | 0 (tri) | -2.03e-02 at (0.30,0.95) | -1.02e-01 at (0.30,0.95) | -3.86e-01 at (0.30,0.95) |
| gauss:0.25 | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) |
| gauss:0.5 | -7.04e-04 at (0.30,0.95) | -5.06e-03 at (0.30,0.95) | -2.03e-02 at (0.30,0.95) | -6.39e-02 at (0.30,0.95) | -2.16e-01 at (0.30,0.95) |
| gauss:1 | -2.53e-04 at (0.30,0.95) | -9.99e-03 at (0.30,0.95) | -4.41e-02 at (0.30,0.95) | -1.41e-01 at (0.30,0.95) | -4.82e-01 at (0.30,0.95) |
| gauss:2 | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) |
| gauss:4 | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) |
| riesz:3 | 0 (tri) | 0 (tri) | -1.16e-03 at (0.30,0.95) | -6.66e-02 at (0.30,0.95) | -2.96e-01 at (0.30,0.95) |
| riesz:4 | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) | -6.92e-02 at (0.30,0.95) |
| riesz:6 | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) | 0 (tri) |
| yukawa:1 | 0 (tri) | -2.70e-03 at (0.30,0.95) | -1.90e-02 at (0.30,0.95) | -6.55e-02 at (0.30,0.95) | -2.28e-01 at (0.30,0.95) |
| yukawa:2 | 0 (tri) | -2.59e-03 at (0.30,0.95) | -1.64e-02 at (0.30,0.95) | -5.58e-02 at (0.30,0.95) | -1.94e-01 at (0.30,0.95) |

## Figures (generated by `phonon_analyze.py`)

- `fig_cm_stability_dEzp.png`: dE_zp of the dynamically stable lattices around the triangular point (blue = lower than triangular), grey x = unstable lattice, per CM potential.
- `fig_cm_rho_curvature.png`: c(r) (min and max over angles) versus r, symlog; negative = triangular is a local maximum of E_zp.
- `fig_cm_kconv.png`: k-grid convergence at the triangular point (n^-3).
- `fig_neg_stability_dEzp.png`, `fig_neg_rho_curvature.png`, `fig_neg_kconv.png`: the same for the non-CM controls.

## Caveats and limits (what this does not show)

- Tier X only: floating-point numerics. The refutation is numerical; it is a robust one (margin about 10^4 times the grid error, two independent implementations, stable on k-grids up to n = 384, frozen-phonon agreement 2e-7), but it is not a Lean statement.
- Only 2-dimensional Bravais lattices (one particle per cell) and the harmonic approximation. Nothing is said about non-Bravais structures (QU-2), anharmonic corrections or the true quantum ground state.
- The competitor lattices are not stress-free: they are not static-energy minima over lattice shape (the triangular lattice has the lower static energy, dE_static > 0), so they are not metastable crystals at fixed density. QU-1 as stated compares the functional E_zp over lattice shapes and is refuted regardless; a version asking about E_total at its own equilibrium shape is a different statement.
- The stable set is small and sampled: 6 to 16 lattices out of 567 for the CM potentials at n = 96 (fewer on finer grids). The 'survives' verdicts (gauss 0.25, 2, 4 and riesz 6) rest on the 2-D sample plus two dense 1-D paths, not on an exhaustive 2-D search. For riesz 4 the triangular lattice is a local minimum of E_zp (c > 0) yet a stable lattice at tau = 0.3 + 0.954 i (stable at n = 384, rc = 800) and another at tau = 0.5 + 1.031 i lie lower, so local curvature alone does not decide a verdict.
- Several of the lowest n = 96 competitors sit at the stability edge and turned unstable on finer k-grids (listed above) and were dropped; the verdicts use only points that are stable on n = 96, 192, 384. The headline competitor tau = 0.5 + 0.966 i is well inside the stable region (elastic eigenvalue 0.105 against 0.238 for the triangular lattice). Not claimed: that any listed point is the global minimiser of E_zp over the stable set.
- Coulomb and Keldysh are not covered (see above); unit amplitude and hbar = m = 1 only rescale E_zp.
- Anything resting on openai/math universal-optimality results (the premise that the Gaussian is 'admissible' for the static theorem) is conditional on them (AI-generated, not peer reviewed); the refutation itself does not use them: it only needs the Gaussian to be completely monotone in r^2, which is elementary.
- Consequence for the programme: step 1 refutes QU-1 for 8 of 12 potentials, so step 2 (human proof for Gaussian g) as formulated cannot succeed (a proof of a false statement). The numerics suggest a restricted question (for which g is the triangular lattice a local minimum of E_zp, i.e. sign of c) as a possible QU-1'; that is a conjecture suggested by this table, Tier C, untested beyond the table.

## Commands (all run on cores 4-7, two threads per job)

```
# code: rusty-SUNDIALS worktree physics/p2-zero-point-optimality-2026-10-25, crates/qf-yukawa-crystal
bash build.sh                      # cargo build --release -j 2 --offline -p qf-yukawa-crystal --bin lattice_phonon
bash run_tests.sh                  # 12 phonon unit tests (frozen phonon, Epstein, theta, modular, QU-1 regression, negative control)
python -I scripts/phonon_gen_taus.py taus
python -I scripts/phonon_run_gate.py BIN taus data cm        # coarse + random (n=96), rho/i patches (n=48,96,192), rc=400
python -I scripts/phonon_run_gate.py BIN taus data_neg neg   # non-CM controls
python -I scripts/phonon_neg_search.py BIN taus/coarse.txt neg_search.json
python -I scripts/phonon_controls.py BIN controls.json
python -I scripts/phonon_independent_check.py independent_check.json
python -I scripts/phonon_analyze.py data analysis_cm ; python -I scripts/phonon_analyze.py data_neg analysis_neg
python -I scripts/phonon_edge_check.py BIN .             # stability-edge verification and dense 1-D scans -> edge_check.json
python -I scripts/phonon_longwave_check.py longwave_check.json
python -I scripts/phonon_make_gate.py . GATE.md
bin/lattice_phonon point gauss:1 0.5 0.966025403784 96,192,384 400   # the headline competitor
```

Code and raw results: rusty-SUNDIALS branch `physics/p2-zero-point-optimality-2026-10-25` (`src/phonon.rs`, `src/bin/lattice_phonon.rs`, `scripts/phonon_*.py`, `results/phonon_gate/`). Copies of the tables, JSON and figures are next to this file.
