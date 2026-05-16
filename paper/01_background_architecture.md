## 2. Background and Related Work

### 2.1 The SUNDIALS Suite

SUNDIALS (SUite of Nonlinear and DIfferential/ALgebraic equation Solvers) [1] provides production-grade solvers for ODEs (CVODE), DAEs (IDA), sensitivity analysis (CVODES/IDAS), and nonlinear systems (KINSOL). Developed at Lawrence Livermore National Laboratory, it has accumulated over 10,000 citations and powers simulations across national laboratories worldwide.

CVODE implements variable-coefficient linear multistep methods in Nordsieck form [7]. For stiff systems, it employs BDF methods of orders 1–5; for non-stiff systems, Adams-Moulton methods of orders 1–12. The implicit algebraic system arising at each step is solved via modified Newton iteration with an LU-factored iteration matrix $M = I - \gamma J$, where $\gamma = h\beta_0$ and $J = \partial f/\partial y$.

### 2.2 Memory Safety in Scientific Computing

Memory safety violations in numerical software are a recognized source of silent errors. Hundt [8] demonstrated that C/C++ scientific codes exhibit buffer overflows at rates comparable to systems software. The 2023 NSF workshop on "Correctness in Scientific Computing" [9] identified memory safety as a priority research direction.

Recent efforts to address this include:

- **Kokkos** [10]: C++ performance portability layer with bounds-checked views
- **Julia** [11]: Garbage-collected scientific language with array bounds checking
- **Rust**: Compile-time ownership guarantees with zero runtime overhead

Rust is unique in providing memory safety *without* garbage collection pauses, making it suitable for latency-sensitive numerical kernels.

### 2.3 Rust in Scientific Computing

Rust adoption in scientific computing has accelerated since 2023:

- **faer** [12]: Dense linear algebra achieving BLAS-3 performance
- **nalgebra** [13]: Generic linear algebra with compile-time dimensions
- **rust-ndarray** [14]: N-dimensional arrays inspired by NumPy
- **diffsol** [15]: ODE solver library using sparse methods

However, no prior work has attempted a *direct behavioral equivalent* of CVODE in Rust with formal verification of the equivalence.

### 2.4 Formal Verification of Numerical Software

Lean 4 [16] has emerged as a practical proof assistant for formalizing mathematical software. Harrison's HOL Light verification of floating-point arithmetic [17] established the feasibility of verifying numerical algorithms. Our work extends this direction to ODE solver API equivalence, proving that the Rust implementation's error handling refines the C original's return-code semantics.

## 3. Architecture and Implementation

### 3.1 Workspace Organization

rusty-SUNDIALS is organized as a Cargo workspace with four crates:

```
rusty-SUNDIALS/
├── crates/
│   ├── sundials-core/    # Real type, error types, DenseMat
│   ├── nvector/          # N_Vector abstraction (serial)
│   ├── cvode/            # CVODE solver (BDF + Adams)
│   └── sunlinsol/        # Dense linear solver interface
├── examples/             # Robertson, Van der Pol, exponential
├── proofs/lean4/         # Formal verification
└── mission-control/      # Research dashboard (React)
```

### 3.2 The Nordsieck Array

Following Byrne and Hindmarsh [7], we store the solution history as a Nordsieck array:

$$z_n = \left[y_n,\; h\dot{y}_n,\; \frac{h^2}{2!}\ddot{y}_n,\; \ldots,\; \frac{h^q}{q!}y_n^{(q)}\right]^T$$

The prediction step applies the Pascal triangle matrix $P$:

$$z_n^{(0)} = P \cdot z_{n-1}$$

The correction step updates via the method-specific vector $\ell$:

$$z_n = z_n^{(0)} + \ell \cdot \Delta_n$$

### 3.3 Newton Iteration and Convergence Testing

The implicit equation $G(y_n) = y_n - h\beta_0 f(t_n, y_n) - a_n = 0$ is solved by modified Newton iteration:

$$M\delta^{(m)} = -G(y_n^{(m)}), \quad y_n^{(m+1)} = y_n^{(m)} + \delta^{(m)}$$

where $M = I - \gamma J$ is LU-factored.

**Convergence test (LLNL formulation)**. Let $\delta^{(m)}$ denote the WRMS norm of the Newton correction at iteration $m$. The convergence rate $\rho$ is tracked as:

$$\rho^{(m)} = \max\left(\texttt{CRDOWN} \cdot \rho^{(m-1)},\; \frac{\|\delta^{(m)}\|}{\|\delta^{(m-1)}\|}\right), \quad m \geq 1$$

Convergence is declared when:

$$\|\delta^{(m)}\| \cdot \min(1, \rho^{(m)}) \leq \texttt{tq}_4$$

where $\texttt{tq}_4 = \texttt{NLSCOEF} / \texttt{BDF\_ERR\_COEFF}[q]$.

### 3.4 Rust-Specific Design Decisions

**Zero `unsafe` in solver core.** The entire `cvode` crate contains no `unsafe` blocks. All array accesses are bounds-checked. The `DenseMat` type wraps a `Vec<f64>` with row-major indexing and panics on out-of-bounds access during development.

**Builder pattern.** Solver construction uses an infallible builder:

```rust
let solver = Cvode::builder(Method::Bdf)
    .rtol(1e-4)
    .atol(&[1e-8, 1e-14, 1e-6])
    .build(rhs, y0, t0)?;
```

**Feature-gated experiments.** The `experimental-nls-v2` Cargo feature enables the corrected convergence heuristics (§5) while the default path preserves stable baseline behavior:

```rust
#[cfg(feature = "experimental-nls-v2")]
nls_crate: Real,  // Persistent across steps (H7)
```

**Thread safety.** The solver requires `F: Send + Sync` for the RHS function, enabling safe concurrent usage patterns without runtime synchronization overhead.
