//! Flow of a superfluid past a static potential in the frame of the obstacle, with absorbing layers: the model of the
//! reference run of Kwon & Shin (*Dynamic similarity of vortex shedding in a superfluid flowing past a penetrable
//! obstacle*, Phys. Rev. Research 2026; Zenodo 10.5281/zenodo.20068724), solved here with an **independent scheme**
//! (explicit RK4 on the whole right-hand side with spectral derivatives, double precision) so that agreement with
//! their published numbers is a statement about the physics and not about a shared discretisation.
//!
//! ```text
//! d psi/dt = v(t) d_x psi  -  (i + Gamma(x,y)) [ -1/2 lap + |psi|^2 - 1 ] psi  -  i V(x,y) psi
//! V = V0 exp(-2 ((x - x_obs)^2 + y^2) / sigma^2),   units hbar = m = mu = 1 (length xi, time tau)
//! Gamma = gamma0 max_{x,y}( (2 + tanh((x - W1)/15) - tanh((x - W2)/15))/2 ),  v(t) = v_fin min(t / t_acc, 1)
//! ```
//!
//! `V` is unitary (it does not enter the damping), as in the reference. The reference holds `v` at its value at the
//! *end* of each step ([`Ramp::StepEnd`]); that O(dt) convention shifts the obstacle by 0.00275 xi during the 0.1 tau
//! ramp and changes the force by 3.5e-3, which is why it is reproduced as an option ([`Ramp::StageTimes`] is the
//! natural RK4 reading).
use crate::rect::Grid2D;
use num_complex::Complex64;

/// How `v(t)` enters an RK4 step from `t` to `t + dt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ramp {
    /// `v(t + dt)` in every stage (the reference code's convention)
    StepEnd,
    /// `v` at the stage times `t`, `t + dt/2`, `t + dt/2`, `t + dt`
    StageTimes,
}

/// Parameters of the obstacle flow (defaults: the reference run V0 = 0.9, sigma = 20, v = 0.55).
#[derive(Debug, Clone)]
pub struct FlowParams {
    pub v0: f64,
    pub sigma: f64,
    pub x_obs: f64,
    pub v_fin: f64,
    pub t_acc: f64,
    pub gamma0: f64,
    /// half-widths of the absorbing layers along x and y
    pub wx: f64,
    pub wy: f64,
    pub layer_width: f64,
}

impl Default for FlowParams {
    fn default() -> Self {
        FlowParams {
            v0: 0.9,
            sigma: 20.0,
            x_obs: 100.0,
            v_fin: 0.55,
            t_acc: 0.1,
            gamma0: 0.1,
            wx: 50.0,
            wy: 40.0,
            layer_width: 15.0,
        }
    }
}

/// Scratch buffers of one RHS/RK4 evaluation.
pub struct FlowWorkspace {
    rhs: FlowWorkspaceRhs,
    k1: Vec<Complex64>,
    k2: Vec<Complex64>,
    k3: Vec<Complex64>,
    k4: Vec<Complex64>,
    st: Vec<Complex64>,
}

impl FlowWorkspace {
    pub fn new(n: usize) -> Self {
        let z = vec![Complex64::new(0.0, 0.0); n];
        FlowWorkspace {
            rhs: FlowWorkspaceRhs::new(n),
            k1: z.clone(),
            k2: z.clone(),
            k3: z.clone(),
            k4: z.clone(),
            st: z,
        }
    }
}

/// The solver: a rectangular periodic grid centred on the origin, the potential, the damping profile and the force probe.
pub struct FlowSolver {
    pub grid: Grid2D,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
    pub params: FlowParams,
    pub potential: Vec<f64>,
    pub gamma: Vec<f64>,
    k2: Vec<f64>,
    crop: (usize, usize, usize, usize), // x0, x1, y0, y1 (end exclusive), the interior used for the force
    dvx_crop: Vec<f64>,
}

impl FlowSolver {
    /// The reference geometry: `nx x ny` points on `[-rx, rx) x [-ry, ry)`.
    pub fn new(nx: usize, ny: usize, rx: f64, ry: f64, params: FlowParams) -> Self {
        let grid = Grid2D::new(nx, ny, 2.0 * rx, 2.0 * ry);
        let (x, y) = (grid.x_centered(), grid.y_centered());
        let n = grid.len();
        let mut potential = vec![0.0; n];
        let mut gamma = vec![0.0; n];
        let (w1x, w2x, w1y, w2y) = (
            rx - params.wx,
            -(rx - params.wx),
            ry - params.wy,
            -(ry - params.wy),
        );
        let s = params.layer_width;
        for iy in 0..ny {
            for ix in 0..nx {
                let (xx, yy) = (x[ix], y[iy]);
                potential[iy * nx + ix] = params.v0
                    * (-2.0 * ((xx - params.x_obs).powi(2) + yy * yy) / params.sigma.powi(2)).exp();
                let gx =
                    params.gamma0 * (2.0 + ((xx - w1x) / s).tanh() - ((xx - w2x) / s).tanh()) / 2.0;
                let gy =
                    params.gamma0 * (2.0 + ((yy - w1y) / s).tanh() - ((yy - w2y) / s).tanh()) / 2.0;
                gamma[iy * nx + ix] = if gx > gy { gx } else { gy };
            }
        }
        let mut k2 = vec![0.0; n];
        for iy in 0..ny {
            for ix in 0..nx {
                k2[iy * nx + ix] = grid.kx[ix].powi(2) + grid.ky[iy].powi(2);
            }
        }
        // interior used for the force: indices with x in [w2x, w1x) and y in [w2y, w1y) (np.where(x == value))
        let find = |v: &[f64], target: f64| {
            v.iter()
                .position(|&a| (a - target).abs() < 1e-9)
                .expect("layer edge on the grid")
        };
        let crop = (find(&x, w2x), find(&x, w1x), find(&y, w2y), find(&y, w1y));
        let (cx, cy) = (crop.1 - crop.0, crop.3 - crop.2);
        // d V / d x on the cropped potential with numpy.gradient (central inside, one-sided at the edges)
        let mut dvx_crop = vec![0.0; cx * cy];
        for j in 0..cy {
            let row = |i: usize| potential[(crop.2 + j) * nx + crop.0 + i];
            for i in 0..cx {
                dvx_crop[j * cx + i] = if i == 0 {
                    (row(1) - row(0)) / grid.dx
                } else if i == cx - 1 {
                    (row(cx - 1) - row(cx - 2)) / grid.dx
                } else {
                    (row(i + 1) - row(i - 1)) / (2.0 * grid.dx)
                };
            }
        }
        FlowSolver {
            grid,
            x,
            y,
            params,
            potential,
            gamma,
            k2,
            crop,
            dvx_crop,
        }
    }

    /// `v(t)`: linear ramp to `v_fin` over `t_acc`.
    pub fn velocity(&self, t: f64) -> f64 {
        (self.params.v_fin * t / self.params.t_acc).min(self.params.v_fin)
    }

    /// Right-hand side at frame velocity `v`, written into `out`.
    pub fn rhs(&self, psi: &[Complex64], v: f64, out: &mut [Complex64], ws: &mut FlowWorkspaceRhs) {
        let g = &self.grid;
        ws.ph.copy_from_slice(psi);
        g.fft2(&mut ws.ph, &mut ws.tmp);
        for iy in 0..g.ny {
            for ix in 0..g.nx {
                let i = iy * g.nx + ix;
                ws.lap[i] = ws.ph[i] * (-self.k2[i]);
                ws.dxp[i] = Complex64::new(0.0, g.kx[ix]) * ws.ph[i];
            }
        }
        g.ifft2(&mut ws.lap, &mut ws.tmp);
        g.ifft2(&mut ws.dxp, &mut ws.tmp);
        for i in 0..psi.len() {
            let p = psi[i];
            let h = ws.lap[i] * (-0.5) + p * (p.norm_sqr() - 1.0);
            out[i] = ws.dxp[i] * v
                - (Complex64::new(self.gamma[i], 1.0)) * h
                - Complex64::new(0.0, self.potential[i]) * p;
        }
    }

    /// One RK4 step from `t` (in place).
    pub fn step(&self, psi: &mut [Complex64], t: f64, dt: f64, ramp: Ramp, ws: &mut FlowWorkspace) {
        let (v1, v2, v3, v4) = match ramp {
            Ramp::StepEnd => {
                let v = self.velocity(t + dt);
                (v, v, v, v)
            }
            Ramp::StageTimes => (
                self.velocity(t),
                self.velocity(t + 0.5 * dt),
                self.velocity(t + 0.5 * dt),
                self.velocity(t + dt),
            ),
        };
        let mut r = std::mem::replace(&mut ws.rhs, FlowWorkspaceRhs::empty());
        self.rhs(psi, v1, &mut ws.k1, &mut r);
        for ((st, p), k) in ws.st.iter_mut().zip(psi.iter()).zip(&ws.k1) {
            *st = *p + *k * (0.5 * dt);
        }
        self.rhs(&ws.st, v2, &mut ws.k2, &mut r);
        for ((st, p), k) in ws.st.iter_mut().zip(psi.iter()).zip(&ws.k2) {
            *st = *p + *k * (0.5 * dt);
        }
        self.rhs(&ws.st, v3, &mut ws.k3, &mut r);
        for ((st, p), k) in ws.st.iter_mut().zip(psi.iter()).zip(&ws.k3) {
            *st = *p + *k * dt;
        }
        self.rhs(&ws.st, v4, &mut ws.k4, &mut r);
        for ((((p, k1), k2), k3), k4) in psi
            .iter_mut()
            .zip(&ws.k1)
            .zip(&ws.k2)
            .zip(&ws.k3)
            .zip(&ws.k4)
        {
            *p += (*k1 + (*k2 + *k3) * 2.0 + *k4) * (dt / 6.0);
        }
        ws.rhs = r;
    }

    /// `F_x = int dV/dx |psi|^2` over the interior of the absorbing layers (trapezoid in x then y, as the reference).
    pub fn force_x(&self, psi: &[Complex64]) -> f64 {
        let (x0, x1, y0, y1) = self.crop;
        let (cx, cy) = (x1 - x0, y1 - y0);
        let nx = self.grid.nx;
        let trap = |f: &dyn Fn(usize) -> f64, n: usize, h: f64| -> f64 {
            (0..n - 1).map(|i| 0.5 * (f(i) + f(i + 1)) * h).sum()
        };
        let row = |j: usize| {
            trap(
                &|i| self.dvx_crop[j * cx + i] * psi[(y0 + j) * nx + x0 + i].norm_sqr(),
                cx,
                self.grid.dx,
            )
        };
        trap(&row, cy, self.grid.dy)
    }
}

/// The four scratch arrays of one RHS evaluation (split out so that `step` can borrow the RK stages separately).
pub struct FlowWorkspaceRhs {
    ph: Vec<Complex64>,
    lap: Vec<Complex64>,
    dxp: Vec<Complex64>,
    tmp: Vec<Complex64>,
}

impl FlowWorkspaceRhs {
    fn empty() -> Self {
        FlowWorkspaceRhs {
            ph: Vec::new(),
            lap: Vec::new(),
            dxp: Vec::new(),
            tmp: Vec::new(),
        }
    }

    pub fn new(n: usize) -> Self {
        let z = vec![Complex64::new(0.0, 0.0); n];
        FlowWorkspaceRhs {
            ph: z.clone(),
            lap: z.clone(),
            dxp: z.clone(),
            tmp: z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> FlowSolver {
        // 2:1 grid in the reference geometry scaled down; edges of the layers must fall on grid points
        let p = FlowParams {
            x_obs: 10.0,
            sigma: 4.0,
            wx: 5.0,
            wy: 4.0,
            layer_width: 1.5,
            ..FlowParams::default()
        };
        FlowSolver::new(96, 48, 24.0, 12.0, p)
    }

    /// The uniform state |psi| = 1 is stationary in the absence of potential and flow (the mu offset is built in).
    #[test]
    fn uniform_state_is_stationary_without_obstacle_or_flow() {
        let mut s = small();
        s.potential.iter_mut().for_each(|v| *v = 0.0);
        let n = s.grid.len();
        let mut psi = vec![Complex64::new(1.0, 0.0); n];
        let mut ws = FlowWorkspace::new(n);
        for k in 0..10 {
            s.step(
                &mut psi,
                0.05 * k as f64 + 1.0,
                0.05,
                Ramp::StageTimes,
                &mut ws,
            );
        }
        assert!(
            psi.iter()
                .all(|p| (p - Complex64::new(1.0, 0.0)).norm() < 1e-12)
        );
    }

    /// A plane wave in a undamped, potential-free, flow-free box is an exact solution: phase exp(-i k^2/2 t).
    #[test]
    fn plane_wave_is_exact_without_potential_damping_and_flow() {
        let mut s = small();
        s.potential.iter_mut().for_each(|v| *v = 0.0);
        s.gamma.iter_mut().for_each(|v| *v = 0.0);
        s.params.v_fin = 0.0;
        let g = &s.grid;
        let k = g.kx[3];
        let n = g.len();
        let mut psi: Vec<Complex64> = (0..n)
            .map(|i| Complex64::from_polar(1.0, k * (i % g.nx) as f64 * g.dx))
            .collect();
        let psi0 = psi.clone();
        let mut ws = FlowWorkspace::new(n);
        let dt = 0.01;
        for i in 0..200 {
            s.step(&mut psi, i as f64 * dt, dt, Ramp::StageTimes, &mut ws);
        }
        let ph = Complex64::from_polar(1.0, -0.5 * k * k * 2.0);
        let err = psi
            .iter()
            .zip(&psi0)
            .map(|(a, b)| (a - b * ph).norm())
            .fold(0.0, f64::max);
        assert!(err < 1e-9, "plane-wave error {err:e}");
    }

    /// In the damped layers an over-dense field relaxes toward |psi| = 1: the damping has the right sign.
    #[test]
    fn damping_relaxes_overdensity_inside_the_layers() {
        let mut s = small();
        s.potential.iter_mut().for_each(|v| *v = 0.0);
        s.params.v_fin = 0.0;
        let n = s.grid.len();
        let idx = (0..n)
            .max_by(|&a, &b| s.gamma[a].partial_cmp(&s.gamma[b]).unwrap())
            .unwrap();
        assert!(s.gamma[idx] > 0.09);
        let mut psi = vec![Complex64::new(1.0, 0.0); n];
        psi.iter_mut().for_each(|p| *p = Complex64::new(1.1, 0.0));
        let mut ws = FlowWorkspace::new(n);
        let before = psi[idx].norm();
        for i in 0..100 {
            s.step(&mut psi, i as f64 * 0.01, 0.01, Ramp::StageTimes, &mut ws);
        }
        assert!(
            psi[idx].norm() < before,
            "{} -> {}",
            before,
            psi[idx].norm()
        );
    }
}
