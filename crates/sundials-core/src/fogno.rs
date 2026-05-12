//! Fractional-Order Graph Neural Operator (FoGNO) Preconditioner.
//! Integrates seamlessly with the pure Rust `gmres_preconditioned` solver.

use crate::Real;

pub struct FoGNO {
    pub alpha: Real,
    pub nn_weights: Vec<Real>,
}

impl FoGNO {
    /// Creates a new FoGNO preconditioner with fractional exponent alpha.
    pub fn new(alpha: Real, num_nodes: usize) -> Self {
        Self {
            alpha,
            nn_weights: vec![1.0; num_nodes], // Simplistic uniform weights for structural implementation
        }
    }

    /// Set an explicit weight array (simulating a GNO graph generation)
    pub fn set_weights(&mut self, weights: Vec<Real>) {
        self.nn_weights = weights;
    }

    /// Evaluates the fractional graph operator action P(alpha) * v
    /// Can be passed directly to gmres_preconditioned as `|v, out| fogno.apply(v, out)`
    pub fn apply(&self, v: &[Real], out: &mut [Real]) {
        // Compute P_alpha = diag(w_i ^ alpha) * v
        for i in 0..v.len() {
            out[i] = v[i] * self.nn_weights[i].powf(self.alpha);
        }
    }
}
