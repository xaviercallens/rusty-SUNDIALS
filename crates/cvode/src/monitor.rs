//! Opt-in, read-only atol-dominance monitor (issue 17).
//!
//! The monitor looks at the accepted solution, the scalar tolerances and the
//! cached Jacobian after each accepted step and counts, per component, how
//! often the absolute tolerance dominates `rtol * |y_i|` while the Jacobian
//! column of that component is much larger than the median column. It never
//! writes solver state and never triggers a Jacobian or RHS evaluation.
//!
//! It emits a flag only. It does not estimate the size of the global error.
//! Rule and constants: `docs/verification/PREREG_ATOL_MONITOR_2026-11-19.md`.

use sundials_core::Real;

/// Factor by which atol must exceed `rtol * |y_i|` (c1 of the preregistration).
pub const ATOL_DOMINANCE_FACTOR: Real = 10.0;
/// Factor by which a Jacobian column norm must exceed the median column norm (c2).
pub const STIFF_COLUMN_FACTOR: Real = 10.0;
/// Fraction of monitored steps that must be hits for a component to be flagged.
pub const MIN_HIT_FRACTION: Real = 0.5;
/// Minimum number of monitored steps before a component can be flagged.
pub const MIN_MONITORED_STEPS: usize = 10;

/// Per-component counters and flag after a solve.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentReport {
    /// Accepted steps observed by the monitor.
    pub steps_monitored: usize,
    /// Steps with `atol > c1 * rtol * |y_i|`.
    pub steps_atol_dominated: usize,
    /// Steps with a cached Jacobian whose column i is at least c2 times the median column norm.
    pub steps_jac_stiff: usize,
    /// Steps where both conditions held.
    pub steps_hit: usize,
    /// True iff `steps_monitored >= 10` and `steps_hit >= 0.5 * steps_monitored`.
    pub flagged: bool,
}

/// Snapshot of the monitor state.
#[derive(Debug, Clone, PartialEq)]
pub struct AtolMonitorReport {
    /// One entry per solution component.
    pub components: Vec<ComponentReport>,
}

impl AtolMonitorReport {
    /// Indices of flagged components.
    pub fn flagged_components(&self) -> Vec<usize> {
        self.components
            .iter()
            .enumerate()
            .filter(|(_, c)| c.flagged)
            .map(|(i, _)| i)
            .collect()
    }

    /// True if any component is flagged.
    pub fn any_flagged(&self) -> bool {
        self.components.iter().any(|c| c.flagged)
    }
}

/// Internal counter state.
#[derive(Debug, Clone)]
pub(crate) struct AtolMonitor {
    counts: Vec<[usize; 4]>, // monitored, atol_dominated, jac_stiff, hit
}

/// Median of a slice (mean of the two middle values for even length, 0 if empty).
fn median(values: &[Real]) -> Real {
    let mut v: Vec<Real> = values.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        0.5 * (v[n / 2 - 1] + v[n / 2])
    }
}

impl AtolMonitor {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            counts: vec![[0; 4]; n],
        }
    }

    /// Record one accepted step. `jac_cols[j][k]` is d f_k / d y_j (column-major), if cached.
    pub(crate) fn observe(
        &mut self,
        y: &[Real],
        rtol: Real,
        atol: Real,
        jac_cols: Option<&[Vec<Real>]>,
    ) {
        let n = self.counts.len().min(y.len());
        let col_norms: Option<Vec<Real>> = jac_cols.filter(|c| c.len() >= n).map(|cols| {
            cols.iter()
                .take(n)
                .map(|col| col.iter().fold(0.0, |m: Real, x| m.max(x.abs())))
                .collect()
        });
        let med = col_norms.as_deref().map(median);
        for i in 0..n {
            let a = atol > ATOL_DOMINANCE_FACTOR * rtol * y[i].abs();
            let j = match (&col_norms, med) {
                (Some(norms), Some(m)) => norms[i] > 0.0 && norms[i] >= STIFF_COLUMN_FACTOR * m,
                _ => false,
            };
            let c = &mut self.counts[i];
            c[0] += 1;
            c[1] += usize::from(a);
            c[2] += usize::from(j);
            c[3] += usize::from(a && j);
        }
    }

    pub(crate) fn report(&self) -> AtolMonitorReport {
        AtolMonitorReport {
            components: self
                .counts
                .iter()
                .map(|c| ComponentReport {
                    steps_monitored: c[0],
                    steps_atol_dominated: c[1],
                    steps_jac_stiff: c[2],
                    steps_hit: c[3],
                    flagged: c[0] >= MIN_MONITORED_STEPS
                        && (c[3] as Real) >= MIN_HIT_FRACTION * c[0] as Real,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jac_cols() -> Vec<Vec<Real>> {
        // column 1 is 1000x the others
        vec![vec![0.04, 0.04], vec![1000.0, 1000.0], vec![0.3, 0.0]]
    }

    #[test]
    fn flags_small_stiff_component_under_atol() {
        let mut m = AtolMonitor::new(3);
        let y = [0.5, 1e-5, 0.4];
        for _ in 0..20 {
            m.observe(&y, 1e-4, 1e-7, Some(&jac_cols()));
        }
        let r = m.report();
        assert_eq!(r.flagged_components(), vec![1]);
        assert_eq!(r.components[1].steps_hit, 20);
        assert_eq!(r.components[0].steps_atol_dominated, 0);
    }

    #[test]
    fn does_not_flag_when_rtol_dominated_or_no_jacobian() {
        let mut m = AtolMonitor::new(3);
        let y = [0.5, 1e-5, 0.4];
        for _ in 0..20 {
            // atol far below rtol*|y|: not atol dominated
            m.observe(&y, 1e-4, 1e-13, Some(&jac_cols()));
        }
        assert!(!m.report().any_flagged());
        assert_eq!(m.report().components[1].steps_jac_stiff, 20);
        let mut m2 = AtolMonitor::new(3);
        for _ in 0..20 {
            // atol dominated but no Jacobian: cannot flag
            m2.observe(&y, 1e-4, 1e-7, None);
        }
        let r2 = m2.report();
        assert!(!r2.any_flagged());
        assert_eq!(r2.components[1].steps_atol_dominated, 20);
    }

    #[test]
    fn needs_minimum_steps_and_hit_fraction() {
        let mut m = AtolMonitor::new(3);
        let y = [0.5, 1e-5, 0.4];
        for _ in 0..9 {
            m.observe(&y, 1e-4, 1e-7, Some(&jac_cols()));
        }
        assert!(!m.report().any_flagged());
        let big = [0.5, 1.0, 0.4];
        for _ in 0..12 {
            m.observe(&big, 1e-4, 1e-7, Some(&jac_cols()));
        }
        // 9 hits of 21 steps < 50%
        assert!(!m.report().components[1].flagged);
    }

    #[test]
    fn median_even_and_odd() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
    }
}
