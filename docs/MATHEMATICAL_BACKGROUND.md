# Mathematical Background & Algorithmic Foundations

This document provides a rigorous mathematical overview of the numerical methods implemented in **Rusty-SUNDIALS**, a direct translation and extension of the classic CVODE solver. 

## 1. The Initial Value Problem (IVP)

Rusty-SUNDIALS is designed to solve systems of ordinary differential equations (ODEs) of the form:

$$ \dot{y} = f(t, y), \quad y(t_0) = y_0 $$

where $y \in \mathbb{R}^N$ is the state vector, $t$ is the independent variable (typically time), and $f : \mathbb{R} \times \mathbb{R}^N \to \mathbb{R}^N$ is the right-hand side (RHS) function defining the system dynamics.

## 2. Linear Multistep Methods (LMMs)

To march forward in time from $t_{n-1}$ to $t_n = t_{n-1} + h_n$, Rusty-SUNDIALS utilizes **Linear Multistep Methods**. The general form of a $q$-step LMM is:

$$ \sum_{i=0}^q \alpha_i y_{n-i} = h_n \sum_{i=0}^q \beta_i f(t_{n-i}, y_{n-i}) $$

The code implements two specific families of LMMs:
1. **Adams-Moulton Methods (Orders 1-12)**: Used for non-stiff problems. They are implicit formulas where $\beta_0 \neq 0$ and $\alpha_0 = 1, \alpha_1 = -1$, and $\alpha_i = 0$ for $i \ge 2$.
2. **Backward Differentiation Formulas (BDF) (Orders 1-5)**: Used for stiff problems. These formulas set $\beta_i = 0$ for $i > 0$, yielding:
   $$ \sum_{i=0}^q \alpha_i y_{n-i} = h_n \beta_0 f(t_n, y_n) $$

## 3. The Nordsieck History Array

Instead of storing raw historical values ($y_n, y_{n-1}, \dots, y_{n-q}$), Rusty-SUNDIALS stores the history of the solution as a **Nordsieck Array**. At any time $t_n$, the array $z_n$ is defined as:

$$ z_n = \left[ y_n, \ h_n \dot{y}_n, \ \frac{h_n^2}{2!} \ddot{y}_n, \ \dots, \ \frac{h_n^q}{q!} y^{(q)}_n \right]^T $$

### Why Nordsieck?
The Nordsieck representation provides extreme flexibility for adaptive solvers:
* **Step Size Changes**: To change the step size by a ratio $\eta = h_{new} / h_{old}$, one simply multiplies the $j$-th component of the array by $\eta^j$.
* **Order Changes**: Increasing or decreasing the polynomial order $q$ is numerically seamless compared to standard divided-difference arrays.

### Predictor-Corrector Framework
The integration step is split into a prediction and a correction phase.
1. **Predictor**: The predicted state $z_n^{(0)}$ is obtained via a Pascal triangle matrix multiplication $P$:
   $$ z_n^{(0)} = P \cdot z_{n-1} $$
   which corresponds to a Taylor series extrapolation.
2. **Corrector**: The corrected array is updated via:
   $$ z_n = z_n^{(0)} + l \Delta_n $$
   where $l$ is a method-specific vector (dependent on whether BDF or Adams is used) and $\Delta_n$ is a scalar correction proportional to the difference between the RHS derivative and the predicted derivative.

## 4. Solving the Implicit Equation

Because both Adams-Moulton and BDF methods are implicit ($\beta_0 \neq 0$), calculating $y_n$ requires solving a non-linear algebraic system at every time step:

$$ G(y_n) = y_n - h_n \beta_0 f(t_n, y_n) - a_n = 0 $$

where $a_n$ contains the known historical terms.

### 4.1 Functional Iteration (Non-Stiff)
For non-stiff systems, Rusty-SUNDIALS can solve $G(y_n) = 0$ using simple fixed-point (Picard) iteration:
$$ y_{n}^{(m+1)} = a_n + h_n \beta_0 f\left(t_n, y_n^{(m)}\right) $$
This converges quickly if the step size $h_n$ is small enough relative to the Lipschitz constant of $f$.

### 4.2 Newton-Raphson Iteration (Stiff)
For stiff equations, functional iteration diverges unless $h_n$ is severely restricted. Instead, Rusty-SUNDIALS employs a modified Newton-Raphson iteration:

$$ M \left( y_{n}^{(m+1)} - y_n^{(m)} \right) = -G\left( y_n^{(m)} \right) $$

where the iteration matrix $M$ is defined as:
$$ M \approx I - \gamma J $$
Here, $\gamma = h_n \beta_0$ and $J = \frac{\partial f}{\partial y}$ is the system Jacobian.

## 5. Jacobian Assembly and Linear Solvers

To solve the linear system $M \delta = -G$, Rusty-SUNDIALS leverages dense matrix routines (`sundials_dense`).

1. **Finite-Difference Jacobian**: If an analytical Jacobian is not provided, the solver computes an approximation column-by-column:
   $$ J_{ij} \approx \frac{f_i(t, y + \epsilon e_j) - f_i(t, y)}{\epsilon} $$
   where $e_j$ is the $j$-th unit basis vector and $\epsilon$ is a heuristically scaled perturbation.
2. **LU Factorization**: The matrix $M$ is factorized $M = P L U$ (`dense_getrf`), and the Newton steps are computed via back-substitution (`dense_getrs`).

## 6. Error Control and Adaptivity

Rusty-SUNDIALS aggressively controls the Local Truncation Error (LTE) to guarantee accuracy while maximizing step size.

1. **Error Weight Vector ($W$)**: Defined for each component $i$ as:
   $$ W_i = \frac{1}{\text{RTOL} \cdot |y_i| + \text{ATOL}_i} $$
2. **LTE Estimation**: The LTE is estimated using the difference between the predictor and the final corrector. The weighted root-mean-square norm of the error $E$ must satisfy $\|E\|_{WRMS} \le 1.0$.
3. **Step Size Adaptation**: If the error test passes, the solver estimates a new step size ratio $\eta$:
   $$ \eta = \left( \frac{1}{\|E\|_{WRMS}} \right)^{\frac{1}{q+1}} $$
   If the step is rejected, $\eta$ is aggressively shrunk (e.g., multiplied by $0.25$) and the step is retried.

This dynamic step-size control enables Rusty-SUNDIALS to efficiently march through transient spikes while taking massive steps during steady-state behavior.
