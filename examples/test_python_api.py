"""
Example script demonstrating the Rusty-SUNDIALS Python API (PyO3).

To run this, you must build the Python extension module using maturin:
    cd crates/rusty-sundials-py
    maturin develop --release
    python ../../examples/test_python_api.py
"""

import math
try:
    from rusty_sundials import CvodeSolver
except ImportError:
    print("rusty_sundials not found. Please run 'maturin develop' in crates/rusty-sundials-py")
    exit(1)

def main():
    print("Testing Rusty-SUNDIALS SciML Python API")

    # The Van der Pol oscillator (stiff parameter mu = 1000)
    mu = 1000.0
    
    # Python RHS callback
    # y = [position, velocity]
    def vanderpol_rhs(t, y):
        # y[0]' = y[1]
        # y[1]' = mu * (1 - y[0]^2) * y[1] - y[0]
        y0 = y[0]
        y1 = y[1]
        return [
            y1,
            mu * (1.0 - y0 * y0) * y1 - y0
        ]

    # Initialize the Rust solver natively from Python
    # We use BDF (Backward Differentiation Formula) for stiff equations
    solver = CvodeSolver(method="bdf", rtol=1e-4, atol=1e-8, max_steps=50000)
    
    t0 = 0.0
    y0 = [2.0, 0.0]
    t_out = 3000.0 # Solve for a long time

    print(f"Solving Van der Pol (mu={mu}) to t={t_out}...")
    
    # Solve passes the callback to the Rust SciML Engine
    # Rust manages the BDF iterations, Newton solver, Nordsieck interpolations, 
    # and calls Python only to evaluate the RHS.
    t_final, y_final = solver.solve(vanderpol_rhs, t0, y0, t_out)
    
    print(f"Integration successful!")
    print(f"Reached t = {t_final:.2f}")
    print(f"Final State = [{y_final[0]:.6f}, {y_final[1]:.6f}]")

if __name__ == "__main__":
    main()
