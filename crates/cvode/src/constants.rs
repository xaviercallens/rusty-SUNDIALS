//! CVODE constants — method types, task modes, and solver parameters.

use sundials_core::Real;

/// Linear multistep method selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Adams-Moulton methods (orders 1-12) for non-stiff problems.
    Adams,
    /// BDF methods (orders 1-5) for stiff problems.
    Bdf,
}

impl Method {
    /// Maximum order for this method.
    pub fn max_order(self) -> usize {
        match self {
            Method::Adams => 12,
            Method::Bdf => 5,
        }
    }
}

/// Task mode for the solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    /// Integrate to tout and interpolate the solution there.
    Normal,
    /// Take one internal step and return.
    OneStep,
}

// --- Internal solver constants ---

/// Maximum number of error test failures before step size reduction.
pub(crate) const MAX_ERR_TEST_FAILS: usize = 7;

/// Maximum number of convergence failures.
pub(crate) const MAX_CONV_FAILS: usize = 10;

/// Maximum number of nonlinear solver iterations per step.
pub(crate) const MAX_NLS_ITERS: usize = 3;

/// Safety factor for step size selection.
pub(crate) const SAFETY: Real = 0.9;

/// Maximum step size growth factor (LLNL ETAMAX = 10).
pub(crate) const ETA_MAX: Real = 10.0;

/// Minimum step size reduction factor.
pub(crate) const ETA_MIN: Real = 0.1;

/// Maximum step growth after a failed step (LLNL ETAMXF = 0.2).
pub(crate) const ETA_MAX_FAIL: Real = 0.2;

/// Step size growth factor after first step.
pub(crate) const ETA_MAX_FIRST: Real = 10000.0;

/// Threshold for "too close" detection.
pub(crate) const HMIN_INV: Real = 1.0e12;

/// Nordsieck array size (max order + 1).
pub(crate) const NORDSIECK_SIZE: usize = 13; // Adams max order + 1

/// Steps between forced Jacobian recomputes (LLNL MSBJ = 51).
/// Our previous value of 20 caused 2.5× more Jacobian evaluations than LLNL.
pub(crate) const JAC_RECOMPUTE_INTERVAL: usize = 51;

/// gamma change threshold for lsetup (LLNL DGMAX = 0.2).
pub(crate) const DGMAX_LSETUP: Real = 0.2;
