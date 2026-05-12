//! Differential-Algebraic Equation (DAE) Solver.
//!
//! Provides the mathematical framework for solving implicit DAEs of the form
//! F(t, y, y') = 0.
//!
//! This is essential for constrained physical systems where some states
//! are bound by algebraic constraints (e.g. conservation of energy/momentum
//! implicitly defined by the structure).

pub mod solver;
