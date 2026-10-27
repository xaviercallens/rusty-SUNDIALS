//! Worked examples for the textbook "De la conjecture au noyau Lean" (2026), appendix F.
//!
//! Every function here drives this repository's CVODE (BDF) on a small problem whose answer is
//! known in closed form, so that each run carries its own positive control:
//!
//! * [`prothero_robinson`]: the stiff scalar test `y' = lambda (y - cos t) - sin t`, exact solution
//!   `cos t + (y0 - 1) e^{lambda t}`; [`explicit_euler`] is the negative control (unstable for
//!   `h > 2 / |lambda|`).
//! * [`moran_kernel`] and [`absorb`]: the Kolmogorov forward equation `d pi / dt = pi (P - I)` of the
//!   Moran chain, integrated until the interior mass is negligible; the mass absorbed at `N` must
//!   equal `(1 - r^{-i}) / (1 - r^{-N})` ([`moran_exact`]).
//! * [`relax_traced`]: overdamped relaxation `x' = F(x)` of a periodic Yukawa box
//!   ([`qf_yukawa_crystal::exact::ExactBox`]), recording the energy per particle along the way.
//!
//! Scope: floating-point illustrations (Tier X). They support no theorem.

use cvode::{Cvode, DenseMat, Method, Task};
use nvector::SerialVector;
use qf_yukawa_crystal::exact::ExactBox;

/// Result of one CVODE run of the Prothero-Robinson problem.
#[derive(Debug, Clone)]
pub struct StiffRun {
    /// `(t, y_cvode, y_exact)` at each requested output time.
    pub samples: Vec<(f64, f64, f64)>,
    pub steps: usize,
    pub rhs_evals: usize,
}

/// Exact solution of `y' = lambda (y - cos t) - sin t`, `y(0) = y0`.
pub fn prothero_exact(lambda: f64, y0: f64, t: f64) -> f64 {
    t.cos() + (y0 - 1.0) * (lambda * t).exp()
}

/// Integrate the Prothero-Robinson problem with CVODE BDF (analytic Jacobian `lambda`).
pub fn prothero_robinson(
    lambda: f64,
    y0: f64,
    t_out: &[f64],
    rtol: f64,
) -> Result<StiffRun, String> {
    let rhs = move |t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        ydot[0] = lambda * (y[0] - t.cos()) - t.sin();
        Ok(())
    };
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(rtol)
        .atol(rtol * 1e-2)
        .max_steps(1_000_000)
        .jacobian(move |_t, _y, j: &mut DenseMat| {
            j.cols[0][0] = lambda;
            Ok(())
        })
        .build(rhs, 0.0, SerialVector::from_slice(&[y0]))
        .map_err(|e| format!("CVODE setup failed: {e}"))?;
    let mut samples = Vec::with_capacity(t_out.len());
    for &tout in t_out {
        let (t, y) = solver
            .solve(tout, Task::Normal)
            .map_err(|e| format!("CVODE failed at t = {tout}: {e}"))?;
        samples.push((t, y[0], prothero_exact(lambda, y0, t)));
    }
    Ok(StiffRun {
        samples,
        steps: solver.num_steps(),
        rhs_evals: solver.num_rhs_evals(),
    })
}

/// Explicit Euler with fixed step `h` on the same problem; returns `y(t_end)`.
pub fn explicit_euler(lambda: f64, y0: f64, h: f64, t_end: f64) -> f64 {
    let steps = (t_end / h).round() as usize;
    (0..steps).fold(y0, |y, k| {
        let t = k as f64 * h;
        y + h * (lambda * (y - t.cos()) - t.sin())
    })
}

/// Row-stochastic Moran kernel on `{0, ..., n}` with relative fitness `r` (`r = 1`: neutral).
pub fn moran_kernel(n: usize, r: f64) -> Vec<Vec<f64>> {
    let nf = n as f64;
    (0..=n)
        .map(|i| {
            let x = i as f64;
            let up = r * x / (r * x + nf - x) * (nf - x) / nf;
            let down = (nf - x) / (r * x + nf - x) * x / nf;
            (0..=n)
                .map(|j| match j {
                    _ if j + 1 == i => down,
                    _ if j == i + 1 => up,
                    _ if j == i => 1.0 - up - down,
                    _ => 0.0,
                })
                .collect()
        })
        .collect()
}

/// `(1 - r^{-i}) / (1 - r^{-n})`, or `i / n` when `r = 1`.
pub fn moran_exact(n: usize, r: f64, i: usize) -> f64 {
    if r == 1.0 {
        i as f64 / n as f64
    } else {
        (1.0 - r.powi(-(i as i32))) / (1.0 - r.powi(-(n as i32)))
    }
}

/// Outcome of a Kolmogorov forward integration.
#[derive(Debug, Clone)]
pub struct Absorption {
    /// Mass absorbed at the last state (the fixation probability once absorbed).
    pub p_fix: f64,
    /// Mass still on the interior states at the final time.
    pub interior: f64,
    /// `|sum pi - 1|` at the final time.
    pub mass_defect: f64,
    pub t: f64,
    pub steps: usize,
    /// `(t, mass at the last state, interior mass)` at each output time.
    pub trace: Vec<(f64, f64, f64)>,
}

/// Integrate `d pi / dt = pi (P - I)` from `pi(0) = e_i` with CVODE BDF (exact Jacobian
/// `(P - I)^T`), doubling the output time until the interior mass is below `tol` or `t > tmax`.
pub fn absorb(kernel: &[Vec<f64>], i: usize, tol: f64, tmax: f64) -> Result<Absorption, String> {
    let m = kernel.len();
    let k1 = kernel.to_vec();
    let k2 = kernel.to_vec();
    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        for (j, d) in ydot.iter_mut().enumerate() {
            *d = y.iter().zip(&k1).map(|(yi, row)| yi * row[j]).sum::<f64>() - y[j];
        }
        Ok(())
    };
    let jac = move |_t: f64, _y: &[f64], j: &mut DenseMat| -> Result<(), String> {
        for (c, col) in j.cols.iter_mut().enumerate() {
            for (r, v) in col.iter_mut().enumerate() {
                *v = k2[c][r] - if r == c { 1.0 } else { 0.0 };
            }
        }
        Ok(())
    };
    let mut y0 = vec![0.0; m];
    y0[i] = 1.0;
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-11)
        .atol(1e-15)
        .max_steps(5_000_000)
        .jacobian(jac)
        .build(rhs, 0.0, SerialVector::from_slice(&y0))
        .map_err(|e| format!("CVODE setup failed: {e}"))?;
    let mut trace = Vec::new();
    let mut tout = 0.25;
    loop {
        let (t, y) = solver
            .solve(tout, Task::Normal)
            .map_err(|e| format!("CVODE failed at t = {tout}: {e}"))?;
        let interior: f64 = y[1..m - 1].iter().sum();
        trace.push((t, y[m - 1], interior));
        if interior < tol || t >= tmax {
            return Ok(Absorption {
                p_fix: y[m - 1],
                interior,
                mass_defect: (y.iter().sum::<f64>() - 1.0).abs(),
                t,
                steps: solver.num_steps(),
                trace,
            });
        }
        tout *= 2.0;
    }
}

/// One relaxation sample: `(t, E/N, max |F|)`.
pub type RelaxSample = (f64, f64, f64);

/// Overdamped relaxation `x' = F(x)` in an [`ExactBox`] with CVODE BDF (Jacobian `-Hessian`),
/// doubling the output time until `max |F| < ftol` or `t > tmax`.
/// Returns the final positions and `(t, E/N, max |F|)` at each output time (starting at `t = 0`).
pub fn relax_traced(
    sys: &ExactBox,
    x0: &[f64],
    ftol: f64,
    tmax: f64,
) -> Result<(Vec<f64>, Vec<RelaxSample>), String> {
    let n = (x0.len() / 2) as f64;
    let (s1, s2) = (sys.clone(), sys.clone());
    let rhs = move |_t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        s1.force(y, ydot);
        Ok(())
    };
    let jac = move |_t: f64, y: &[f64], j: &mut DenseMat| -> Result<(), String> {
        let h = s2.hessian(y);
        let m = y.len();
        for (c, col) in j.cols.iter_mut().enumerate() {
            for (r, v) in col.iter_mut().enumerate() {
                *v = -h[r * m + c];
            }
        }
        Ok(())
    };
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(1e-10)
        .atol(1e-12)
        .max_steps(2_000_000)
        .jacobian(jac)
        .build(rhs, 0.0, SerialVector::from_slice(x0))
        .map_err(|e| format!("CVODE setup failed: {e}"))?;
    let mut f = vec![0.0; x0.len()];
    sys.force(x0, &mut f);
    let fmax = |f: &[f64]| f.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
    let mut trace = vec![(0.0, sys.energy(x0) / n, fmax(&f))];
    let mut tout = 0.01;
    loop {
        let (t, y) = solver
            .solve(tout, Task::Normal)
            .map_err(|e| format!("CVODE failed at t = {tout}: {e}"))?;
        let y = y.to_vec();
        sys.force(&y, &mut f);
        trace.push((t, sys.energy(&y) / n, fmax(&f)));
        if fmax(&f) < ftol || t >= tmax {
            let mut x = y;
            sys.wrap(&mut x);
            return Ok((x, trace));
        }
        tout *= 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prothero_matches_exact_solution() {
        let run = prothero_robinson(-1e4, 2.0, &[0.5, 2.0], 1e-8).unwrap();
        for &(_, y, ex) in &run.samples {
            assert!((y - ex).abs() < 1e-6, "y = {y}, exact = {ex}");
        }
        assert!(
            run.steps < 5_000,
            "BDF should not need many steps: {}",
            run.steps
        );
    }

    #[test]
    fn explicit_euler_blows_up_beyond_stability_limit() {
        let stable = explicit_euler(-1e4, 2.0, 1e-4, 1.0);
        let unstable = explicit_euler(-1e4, 2.0, 2.5e-4, 1.0);
        assert!((stable - 1f64.cos()).abs() < 1e-3);
        assert!(unstable.abs() > 1e6 || !unstable.is_finite());
    }

    #[test]
    fn moran_kernel_is_stochastic_and_absorbing() {
        let p = moran_kernel(5, 1.5);
        for row in &p {
            assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-15);
            assert!(row.iter().all(|&x| x >= 0.0));
        }
        assert_eq!(p[0][0], 1.0);
        assert_eq!(p[5][5], 1.0);
    }

    #[test]
    fn forward_equation_recovers_four_sevenths() {
        let a = absorb(&moran_kernel(3, 2.0), 1, 1e-12, 1e6).unwrap();
        assert!((a.p_fix - 4.0 / 7.0).abs() < 1e-9, "p_fix = {}", a.p_fix);
        assert!(a.mass_defect < 1e-10);
        // Negative control: the neutral value 1/3 is not what the selected chain gives.
        assert!((a.p_fix - 1.0 / 3.0).abs() > 0.2);
    }

    #[test]
    fn identity_kernel_never_absorbs() {
        let id: Vec<Vec<f64>> = (0..4)
            .map(|i| (0..4).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
        let a = absorb(&id, 1, 1e-12, 100.0).unwrap();
        assert!(a.interior > 0.99);
        assert!(a.p_fix.abs() < 1e-12);
    }
}
