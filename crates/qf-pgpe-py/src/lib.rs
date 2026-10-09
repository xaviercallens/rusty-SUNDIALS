//! `qf_pgpe`: Python extension for the quantum-fluids engine of rusty-SUNDIALS (`qf-pgpe`).
//!
//! The engine, the vortex instrument and the registered transport estimators run in Rust on numpy arrays, with no
//! per-step Python round trip (`Pgpe.run` evolves a field for `t_end` time units inside one call). Fields are
//! C-contiguous `complex128` arrays of shape `(n, n)` holding the projected Fourier amplitudes `c` of
//! `pgpe.PGPE` (numpy `fft2` convention), so Python scripts written against `pgpe.py` can swap the engine without
//! changing their data layout:
//!
//! ```python
//! import numpy as np, qf_pgpe
//! s = qf_pgpe.Pgpe(128, 64.0)                       # n, L, g=1, dt=0.01, kcut_frac=0.5
//! c = s.imprint_v2(s.uniform(), np.array([[35., 20.], [25., 20.]]), np.array([1, -1]))
//! c = s.run(c, 100.0)
//! pos, q = s.detect(c)
//! ```
// pyo3 0.20's #[pymethods]/#[pyfunction] expansions trip `non_local_definitions` on newer rustc (fixed in pyo3 >= 0.21).
#![allow(non_local_definitions)]
#![cfg(not(target_arch = "wasm32"))]

use num_complex::Complex64;
use numpy::{IntoPyArray, PyArray1, PyArray2, PyReadonlyArray1, PyReadonlyArray2};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use qf_engine::transport::{self, Options, Track};
use qf_engine::{vortex, ComplexField2D};

fn to_vec(a: &PyReadonlyArray2<Complex64>, n: usize) -> PyResult<Vec<Complex64>> {
    let v = a.as_array();
    if v.shape() != [n, n] {
        return Err(PyValueError::new_err(format!(
            "expected a ({n}, {n}) complex128 array, got {:?}",
            v.shape()
        )));
    }
    Ok(v.iter().copied().collect())
}

fn to_array<'py>(py: Python<'py>, v: Vec<Complex64>, n: usize) -> &'py PyArray2<Complex64> {
    numpy::ndarray::Array2::from_shape_vec((n, n), v)
        .expect("n*n elements")
        .into_pyarray(py)
}

/// The projected Gross-Pitaevskii engine on an `n x n` periodic grid of side `l`.
#[pyclass(name = "Pgpe")]
pub struct PyPgpe {
    f: ComplexField2D,
}

#[pymethods]
impl PyPgpe {
    #[new]
    #[pyo3(signature = (n, l, g = 1.0, dt = 0.01, kcut_frac = 0.5))]
    fn new(n: usize, l: f64, g: f64, dt: f64, kcut_frac: f64) -> PyResult<Self> {
        if n < 4 || !n.is_multiple_of(2) {
            return Err(PyValueError::new_err("n must be an even integer >= 4"));
        }
        Ok(PyPgpe {
            f: ComplexField2D::with_kcut_frac(n, l, g, dt, kcut_frac),
        })
    }

    #[getter]
    fn n(&self) -> usize {
        self.f.n
    }
    #[getter]
    fn l(&self) -> f64 {
        self.f.l
    }
    #[getter]
    fn dx(&self) -> f64 {
        self.f.dx
    }
    #[getter]
    fn kcut(&self) -> f64 {
        self.f.kcut
    }
    #[getter]
    fn n_modes(&self) -> usize {
        self.f.n_modes()
    }

    /// The uniform condensate `psi = 1` (density 1) as projected amplitudes.
    fn uniform<'py>(&self, py: Python<'py>) -> &'py PyArray2<Complex64> {
        to_array(py, vortex::uniform_condensate(&self.f), self.f.n)
    }

    /// `P . fft2(psi)`.
    fn modes<'py>(
        &self,
        py: Python<'py>,
        psi: PyReadonlyArray2<Complex64>,
    ) -> PyResult<&'py PyArray2<Complex64>> {
        Ok(to_array(
            py,
            self.f.modes(&to_vec(&psi, self.f.n)?),
            self.f.n,
        ))
    }

    /// `ifft2(c)`.
    fn psi<'py>(
        &self,
        py: Python<'py>,
        c: PyReadonlyArray2<Complex64>,
    ) -> PyResult<&'py PyArray2<Complex64>> {
        Ok(to_array(py, self.f.psi(&to_vec(&c, self.f.n)?), self.f.n))
    }

    /// Evolve for `t_end` time units (`round(t_end/dt)` IF-RK4 steps) inside one call; the GIL is released.
    fn run<'py>(
        &self,
        py: Python<'py>,
        c: PyReadonlyArray2<Complex64>,
        t_end: f64,
    ) -> PyResult<&'py PyArray2<Complex64>> {
        let c0 = to_vec(&c, self.f.n)?;
        let out = py.allow_threads(|| self.f.run(&c0, t_end));
        Ok(to_array(py, out, self.f.n))
    }

    fn step<'py>(
        &self,
        py: Python<'py>,
        c: PyReadonlyArray2<Complex64>,
    ) -> PyResult<&'py PyArray2<Complex64>> {
        Ok(to_array(py, self.f.step(&to_vec(&c, self.f.n)?), self.f.n))
    }

    fn norm(&self, c: PyReadonlyArray2<Complex64>) -> PyResult<f64> {
        Ok(self.f.norm(&to_vec(&c, self.f.n)?))
    }

    fn energy(&self, c: PyReadonlyArray2<Complex64>) -> PyResult<f64> {
        Ok(self.f.energy(&to_vec(&c, self.f.n)?))
    }

    fn momentum(&self, c: PyReadonlyArray2<Complex64>) -> PyResult<(f64, f64)> {
        Ok(self.f.momentum(&to_vec(&c, self.f.n)?))
    }

    /// Periodic theta-function imprint of vortices at `pos` (shape `(m, 2)`) with integer charges `q`.
    fn imprint_v2<'py>(
        &self,
        py: Python<'py>,
        c: PyReadonlyArray2<Complex64>,
        pos: PyReadonlyArray2<f64>,
        q: PyReadonlyArray1<i64>,
    ) -> PyResult<&'py PyArray2<Complex64>> {
        let p = pos.as_array();
        if p.ncols() != 2 || p.nrows() != q.len() {
            return Err(PyValueError::new_err(
                "pos must have shape (m, 2) with m = len(q)",
            ));
        }
        let pts: Vec<(f64, f64)> = p.rows().into_iter().map(|r| (r[0], r[1])).collect();
        let qs: Vec<i32> = q.as_array().iter().map(|&x| x as i32).collect();
        Ok(to_array(
            py,
            vortex::imprint_v2(&self.f, &to_vec(&c, self.f.n)?, &pts, &qs),
            self.f.n,
        ))
    }

    /// Raw plaquette vortices with sub-grid refinement: `(pos (k, 2), q (k,))`.
    fn detect<'py>(
        &self,
        py: Python<'py>,
        c: PyReadonlyArray2<Complex64>,
    ) -> PyResult<(&'py PyArray2<f64>, &'py PyArray1<i64>)> {
        let det = vortex::detect(&self.f, &to_vec(&c, self.f.n)?);
        let flat: Vec<f64> = det.iter().flat_map(|d| [d.0, d.1]).collect();
        let pos = numpy::ndarray::Array2::from_shape_vec((det.len(), 2), flat)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let q: Vec<i64> = det.iter().map(|d| d.2 as i64).collect();
        Ok((pos.into_pyarray(py), q.into_pyarray(py)))
    }

    /// Band momentum `(Px, Py)` of the modes with `|k| > kmin` (the phonon band for `kmin = 1`).
    fn momentum_band(&self, c: PyReadonlyArray2<Complex64>, kmin: f64) -> PyResult<(f64, f64)> {
        Ok(vortex::momentum_band(&self.f, &to_vec(&c, self.f.n)?, kmin))
    }
}

/// The registered transport estimators on tracked positions: `tracks` is a list of `(t (nt,), R (nt, nv, 2), q (nv,))`;
/// returns a dict with the six estimates and their jackknife errors (names as in `transport_estimators.analyse_tracks`).
#[pyfunction]
#[pyo3(signature = (tracks, l, lag = 10, t_settle = 100.0, d_valid = 4.0))]
fn analyse_tracks(
    py: Python<'_>,
    tracks: Vec<(
        PyReadonlyArray1<f64>,
        &numpy::PyArray3<f64>,
        PyReadonlyArray1<i64>,
    )>,
    l: f64,
    lag: usize,
    t_settle: f64,
    d_valid: f64,
) -> PyResult<PyObject> {
    let mut ts = Vec::new();
    for (t, r, q) in &tracks {
        let ra = r.readonly();
        let a = ra.as_array();
        let (nt, nv) = (a.shape()[0], a.shape()[1]);
        let rows: Vec<Vec<(f64, f64)>> = (0..nt)
            .map(|k| (0..nv).map(|i| (a[[k, i, 0]], a[[k, i, 1]])).collect())
            .collect();
        ts.push(Track {
            t: t.as_array().to_vec(),
            r: rows,
            q: q.as_array().iter().map(|&x| x as i32).collect(),
        });
    }
    let opts = Options {
        lag,
        t_settle,
        d_valid,
        ..Options::default()
    };
    let e = py
        .allow_threads(|| transport::analyse_tracks(&ts, l, &opts))
        .map_err(PyRuntimeError::new_err)?;
    let d = pyo3::types::PyDict::new(py);
    let vals = e.values();
    for (k, name) in transport::NAMES.iter().enumerate() {
        d.set_item(*name, vals[k])?;
    }
    let ses = [
        e.one_minus_alpha_prime_se,
        e.alpha_regression_se,
        e.alpha_energy_se,
        e.eta_se,
        e.msd_exponent_se,
        e.msd_offset_se,
    ];
    for (k, name) in transport::NAMES.iter().enumerate() {
        d.set_item(format!("{name}_se"), ses[k])?;
    }
    d.set_item("n_blocks", e.n_blocks)?;
    d.set_item("lag", e.lag)?;
    d.set_item("msd", e.msd)?;
    Ok(d.into())
}

#[pymodule]
fn qf_pgpe(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPgpe>()?;
    m.add_function(wrap_pyfunction!(analyse_tracks, m)?)?;
    Ok(())
}
