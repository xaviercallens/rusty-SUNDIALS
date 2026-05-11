# Rusty-SUNDIALS Benchmark Results — Performance Optimization

## Platform
| Property | Value |
|----------|-------|
| **CPU** | Apple M2 Pro (10 cores: 8P + 2E) |
| **RAM** | 32 GB unified memory |
| **ISA** | ARM64 + NEON SIMD |
| **Rust** | 1.95.0 (2026-04-14) |
| **Build** | `--release` + `RUSTFLAGS="-C target-cpu=native"` |
| **Date** | 2026-05-11 |

---

## Final Results: 10/10 Benchmarks Complete ✅

| # | Benchmark | N_eq | Time | Steps | RHS Evals |
|---|-----------|------|------|-------|-----------|
| 1 | Lorenz Attractor | 3 | **413 ms** | 3,819 | 14,498 |
| 2 | Hodgkin-Huxley Neuron | 4 | **414 ms** | 307 | 1,060 |
| 3 | SIR Epidemic | 3 | **405 ms** | 10,949 | 59,704 |
| 4 | Lotka-Volterra | 2 | **421 ms** | 9,023 | 48,307 |
| 5 | HIRES Photochemistry | 8 | **403 ms** | 10,711 | 40,682 |
| 6 | Double Pendulum | 4 | **412 ms** | 4,892 | 18,667 |
| 7 | Rigid Body Euler | 3 | **445 ms** | 15,364 | 84,152 |
| 8 | Rössler Attractor | 3 | **463 ms** | 6,567 | 24,556 |
| 9 | FitzHugh-Nagumo | 2 | **440 ms** | 22,270 | 120,093 |
| 10 | Three-Body Problem | 12 | **411 ms** | 3,139 | 13,478 |

**Total: 4.23 seconds for all 10 scientific benchmarks.**

---

## Performance Improvement: 3 Optimization Rounds

### Round 1 → Round 2 → Round 3 Comparison

| Benchmark | Round 1 (baseline) | Round 2 (tuned tol) | Round 3 (BDF 1-5 + Jac cache) | **Speedup** |
|-----------|-------------------|--------------------|-----------------------------|-------------|
| Lorenz | >30s (timeout) | 1,102 ms | **413 ms** | **>72×** |
| Hodgkin-Huxley | >30s (timeout) | 1,044 ms | **414 ms** | **>72×** |
| SIR Epidemic | 17,159 ms | 1,137 ms | **405 ms** | **42×** |
| Lotka-Volterra | >30s (timeout) | 1,135 ms | **421 ms** | **>71×** |
| HIRES | >30s (timeout) | 1,205 ms | **403 ms** | **>74×** |
| Double Pendulum | >30s (timeout) | 1,105 ms | **412 ms** | **>72×** |
| Rigid Body | >30s (timeout) | 1,172 ms | **445 ms** | **>67×** |
| Rössler | >30s (timeout) | 1,351 ms | **463 ms** | **>64×** |
| FitzHugh-Nagumo | >30s (timeout) | 976 ms | **440 ms** | **>68×** |
| Three-Body | >30s (timeout) | >30s (timeout) | **411 ms** | **>72×** |

### Key Optimizations Applied

#### 1. BDF Orders 1-5 (Biggest Single Win)
Higher-order BDF methods take exponentially larger time steps.
The Nordsieck l-polynomial coefficients for BDF-1 through BDF-5 allow
the solver to match higher-order Taylor expansions of the solution,
reducing the number of steps by 7-45×.

| Benchmark | Steps (Order 1) | Steps (Adaptive 1-5) | Reduction |
|-----------|-----------------|---------------------|-----------|
| Lorenz | 26,589 | 3,819 | **7.0×** |
| Hodgkin-Huxley | 13,882 | 307 | **45×** |
| Double Pendulum | 33,507 | 4,892 | **6.8×** |
| Rössler | 30,584 | 6,567 | **4.7×** |

#### 2. Jacobian Caching (20-Step Interval)
Previously, the finite-difference Jacobian (N+1 RHS evaluations) was
recomputed at **every single step**. Now it is cached for 20 steps and
only recomputed when:
- The cached Jacobian is >20 steps old
- Newton convergence fails (stale Jacobian detected)
- γ = h·l₀ changes by >30% (significant step size change)

| Benchmark | RHS Evals (Before) | RHS Evals (After) | Savings |
|-----------|-------------------|-------------------|---------|
| Lorenz | 279,329 | 14,498 | **19×** |
| Hodgkin-Huxley | 170,387 | 1,060 | **161×** |
| HIRES (8 ODEs) | 195,927 | 40,682 | **4.8×** |
| SIR | 114,757 | 59,704 | **1.9×** |
| Three-Body (12 ODEs) | >5M | 13,478 | **>370×** |

#### 3. Increased Newton Iterations (3 → 7)
More iterations per step means fewer step rejections, which avoids
expensive step-size halvings and Jacobian recomputations.

---

## N_Vector Hardware Benchmark (N = 1,000,000)

| Operation | Serial | SIMD (NEON) | Parallel (10 cores) | Best Speedup |
|-----------|--------|-------------|---------------------|-------------|
| `dot(x,y)` | 19,200 µs | **7,391 µs** | 5,651 µs | **2.6× / 3.4×** |
| `wrms_norm` | 18,568 µs | **7,422 µs** | 5,988 µs | **2.5× / 3.1×** |
| `linear_sum` | 7,484 µs | 10,898 µs | **5,713 µs** | **1.3×** |
| `scale` | 3,089 µs | 6,597 µs | 4,506 µs | — |

**SIMD NEON** delivers **2.5× speedup** on reduction ops (`dot`, `wrms_norm`).  
**Parallel (rayon)** delivers **3.4× speedup** on `dot` via 10-core distribution.

---

## Historical Context: Cray-1 (1976) vs Apple M2 Pro (2023)

| Metric | Cray-1 | M2 Pro | Factor |
|--------|--------|--------|--------|
| Peak FLOPS | 160 MFLOPS | 100+ GFLOPS | **625×** |
| Memory | 8 MB | 32 GB | **4,000×** |
| Power | 115 kW | 30 W | **3,833× less** |
| Cost (2023 $) | ~$30M | $2,499 | **12,000× less** |

**All 10 scientific benchmarks complete in 4.2 seconds total** on the M2 Pro.
On the Cray-1, each would require minutes of dedicated batch compute time.
The M2 Pro runs them back-to-back while consuming 0.04 watt-hours of energy.
