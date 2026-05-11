//! Dual numbers for Forward-Mode Automatic Differentiation.
//!
//! Dual numbers are of the form `a + bε`, where `ε² = 0`.
//! They allow for the exact computation of first derivatives without truncation error.
//! In the context of Jacobian-Free Newton Krylov (JFNK), they provide exact
//! Jacobian-vector products (Jv) in a single pass of the RHS function.
//!
//! If `f` is our ODE RHS, then:
//!   `f(y + vε) = f(y) + Jv * ε`
//! By passing Dual numbers into the RHS function, we get `f(y)` in the real part
//! and exactly `Jv` in the dual part.
//!
//! Reference: Revels, J., Lubin, M., & Papamarkou, T. (2016). Forward-Mode Automatic
//! Differentiation in Julia. arXiv:1607.07892.

use std::ops::{Add, AddAssign, Div, Mul, Sub};

/// A dual number representing `real + dual * ε`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dual {
    pub real: f64,
    pub dual: f64,
}

impl Dual {
    /// Create a new Dual number.
    pub fn new(real: f64, dual: f64) -> Self {
        Self { real, dual }
    }

    /// Create a constant Dual number (derivative is zero).
    pub fn constant(real: f64) -> Self {
        Self { real, dual: 0.0 }
    }

    pub fn sin(self) -> Self {
        Self {
            real: self.real.sin(),
            dual: self.dual * self.real.cos(),
        }
    }

    pub fn cos(self) -> Self {
        Self {
            real: self.real.cos(),
            dual: -self.dual * self.real.sin(),
        }
    }

    pub fn exp(self) -> Self {
        let e = self.real.exp();
        Self {
            real: e,
            dual: self.dual * e,
        }
    }
}

// Implement standard arithmetic operations

impl Add for Dual {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            real: self.real + rhs.real,
            dual: self.dual + rhs.dual,
        }
    }
}

impl Sub for Dual {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            real: self.real - rhs.real,
            dual: self.dual - rhs.dual,
        }
    }
}

impl Mul for Dual {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        // (a + bε)(c + dε) = ac + (ad + bc)ε
        Self {
            real: self.real * rhs.real,
            dual: self.real * rhs.dual + self.dual * rhs.real,
        }
    }
}

impl Div for Dual {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        // (a + bε)/(c + dε) = (a/c) + (bc - ad)/(c^2) ε
        Self {
            real: self.real / rhs.real,
            dual: (self.dual * rhs.real - self.real * rhs.dual) / (rhs.real * rhs.real),
        }
    }
}

impl AddAssign for Dual {
    fn add_assign(&mut self, rhs: Self) {
        self.real += rhs.real;
        self.dual += rhs.dual;
    }
}

// Also allow interacting with f64 constants directly
impl Mul<f64> for Dual {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            real: self.real * rhs,
            dual: self.dual * rhs,
        }
    }
}

impl Mul<Dual> for f64 {
    type Output = Dual;
    fn mul(self, rhs: Dual) -> Self::Output {
        Dual {
            real: self * rhs.real,
            dual: self * rhs.dual,
        }
    }
}

impl Add<f64> for Dual {
    type Output = Self;
    fn add(self, rhs: f64) -> Self::Output {
        Self {
            real: self.real + rhs,
            dual: self.dual,
        }
    }
}

impl Sub<f64> for Dual {
    type Output = Self;
    fn sub(self, rhs: f64) -> Self::Output {
        Self {
            real: self.real - rhs,
            dual: self.dual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_arithmetic() {
        let x = Dual::new(3.0, 1.0); // f(x) = x at x=3, f'(x) = 1
        let y = x * x; // f(x) = x^2, f'(x) = 2x
        assert_eq!(y.real, 9.0);
        assert_eq!(y.dual, 6.0);
    }

    #[test]
    fn test_exact_jacobian_vector_product() {
        // Let our system be:
        // f_0(y) = y_0 * y_1
        // f_1(y) = y_0 + sin(y_1)
        // 
        // We want to compute exactly J * v at point y
        // y = [2.0, pi/2]
        // v = [0.5, 1.0]

        let y0 = 2.0;
        let y1 = std::f64::consts::PI / 2.0;
        let v0 = 0.5;
        let v1 = 1.0;

        // Seed the dual numbers: y_dual = y + v * ε
        let dy0 = Dual::new(y0, v0);
        let dy1 = Dual::new(y1, v1);

        // Evaluate the RHS generically over Dual
        let f0 = dy0 * dy1;
        let f1 = dy0 + dy1.sin();

        // Analytical Jacobian at [2, pi/2]:
        // J = [[y_1, y_0], [1, cos(y_1)]]
        // J = [[pi/2, 2], [1, 0]]
        //
        // J * v = [[pi/2, 2], [1, 0]] * [0.5, 1]
        //       = [pi/4 + 2, 0.5]

        let expected_jv0 = std::f64::consts::PI / 4.0 + 2.0;
        let expected_jv1 = 0.5;

        // The dual part contains exactly J * v !
        assert!((f0.dual - expected_jv0).abs() < 1e-14);
        assert!((f1.dual - expected_jv1).abs() < 1e-14);

        // And the real part contains exactly f(y)
        assert!((f0.real - y0 * y1).abs() < 1e-14);
        assert!((f1.real - (y0 + y1.sin())).abs() < 1e-14);
    }
}
