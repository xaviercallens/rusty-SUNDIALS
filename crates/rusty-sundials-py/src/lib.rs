//! Python bindings for Rusty-SUNDIALS.
//!
//! Provides a seamless Python API matching the SciML SciPy ecosystem,
//! allowing `rusty-SUNDIALS` to be used directly from Python without
//! compromising zero-cost performance.

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyList;

use cvode::solver::Cvode;
use cvode::constants::{Method, Task};
use nvector::SerialVector;
use sundials_core::Real;

/// Python wrapper for the CVODE Builder
#[pyclass(name = "CvodeSolver")]
pub struct PyCvodeSolver {
    method: Method,
    rtol: Real,
    atol: Real,
    max_steps: usize,
}

#[pymethods]
impl PyCvodeSolver {
    #[new]
    #[pyo3(signature = (method = "bdf", rtol = 1e-4, atol = 1e-8, max_steps = 500))]
    fn new(method: &str, rtol: Real, atol: Real, max_steps: usize) -> PyResult<Self> {
        let method = match method.to_lowercase().as_str() {
            "bdf" => Method::Bdf,
            "adams" => Method::Adams,
            _ => return Err(PyRuntimeError::new_err("Invalid method. Use 'bdf' or 'adams'.")),
        };
        Ok(Self { method, rtol, atol, max_steps })
    }

    /// Solves an ODE system.
    /// `rhs_func` must be a Python callable `def rhs(t, y)` returning `ydot`.
    #[pyo3(signature = (rhs_func, t0, y0, t_out))]
    fn solve<'py>(
        &self,
        py: Python<'py>,
        rhs_func: PyObject,
        t0: Real,
        y0: Vec<Real>,
        t_out: Real,
    ) -> PyResult<(Real, Vec<Real>)> {
        let initial_state = SerialVector::from_slice(&y0);

        // Closure that bridges Rust RHS signature to Python RHS callable
        let rhs = |t: Real, y: &[Real], ydot: &mut [Real]| -> Result<(), String> {
            Python::with_gil(|py_gil| {
                let py_y = PyList::new(py_gil, y);
                let args = (t, py_y);
                match rhs_func.call1(py_gil, args) {
                    Ok(res) => {
                        if let Ok(py_ydot) = res.extract::<Vec<Real>>(py_gil) {
                            if py_ydot.len() != ydot.len() {
                                return Err("RHS returned vector of incorrect length".to_string());
                            }
                            ydot.copy_from_slice(&py_ydot);
                            Ok(())
                        } else {
                            Err("RHS must return a list of floats".to_string())
                        }
                    }
                    Err(e) => Err(format!("Python RHS error: {}", e)),
                }
            })
        };

        // Construct the solver inside the gil or without it, but PyO3 functions are already inside gil
        // We drop GIL if possible to let other python threads run, but our RHS needs GIL.
        let mut solver = Cvode::builder(self.method)
            .rtol(self.rtol)
            .atol(self.atol)
            .max_steps(self.max_steps)
            .build(rhs, t0, initial_state)
            .map_err(|e| PyRuntimeError::new_err(format!("Solver build failed: {}", e)))?;

        let (t_reached, y_reached) = solver.solve(t_out, Task::Normal)
            .map_err(|e| PyRuntimeError::new_err(format!("Solver failed: {}", e)))?;

        Ok((t_reached, y_reached.to_vec()))
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn rusty_sundials(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyCvodeSolver>()?;
    Ok(())
}
