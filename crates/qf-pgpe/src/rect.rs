//! Rectangular periodic grids and their 2D FFTs (numpy `fft2` conventions), the building block of the flow solver
//! ([`crate::flow`]) that reproduces external GPE runs on non-square domains.
//!
//! Layout: row-major `data[iy * nx + ix]` (numpy shape `(ny, nx)`); `x = ix * dx`, `y = iy * dy`.
//! `fft2` is rows (length `nx`), out-of-place tiled transpose, rows (length `ny`), transpose back; unnormalised forward,
//! `ifft2` divides by `nx * ny`, exactly as numpy.
use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// A doubly-periodic rectangular grid of `nx x ny` points on `[0, lx) x [0, ly)` with its FFT plans.
pub struct Grid2D {
    pub nx: usize,
    pub ny: usize,
    pub lx: f64,
    pub ly: f64,
    pub dx: f64,
    pub dy: f64,
    /// angular wavenumbers `2 pi fftfreq(nx, dx)` and `2 pi fftfreq(ny, dy)`
    pub kx: Vec<f64>,
    pub ky: Vec<f64>,
    fx_fwd: Arc<dyn Fft<f64>>,
    fx_inv: Arc<dyn Fft<f64>>,
    fy_fwd: Arc<dyn Fft<f64>>,
    fy_inv: Arc<dyn Fft<f64>>,
}

fn angular_fftfreq(k: usize, n: usize, d: f64) -> f64 {
    let signed = if k < n.div_ceil(2) {
        k as i64
    } else {
        k as i64 - n as i64
    };
    2.0 * std::f64::consts::PI * signed as f64 / (n as f64 * d)
}

/// Out-of-place tiled transpose of an `r x c` row-major matrix into a `c x r` one.
fn transpose(src: &[Complex64], dst: &mut [Complex64], r: usize, c: usize) {
    const B: usize = 16;
    for bi in (0..r).step_by(B) {
        for bj in (0..c).step_by(B) {
            for i in bi..(bi + B).min(r) {
                for j in bj..(bj + B).min(c) {
                    dst[j * r + i] = src[i * c + j];
                }
            }
        }
    }
}

impl Grid2D {
    pub fn new(nx: usize, ny: usize, lx: f64, ly: f64) -> Self {
        let (dx, dy) = (lx / nx as f64, ly / ny as f64);
        let mut planner = FftPlanner::new();
        Grid2D {
            nx,
            ny,
            lx,
            ly,
            dx,
            dy,
            kx: (0..nx).map(|k| angular_fftfreq(k, nx, dx)).collect(),
            ky: (0..ny).map(|k| angular_fftfreq(k, ny, dy)).collect(),
            fx_fwd: planner.plan_fft_forward(nx),
            fx_inv: planner.plan_fft_inverse(nx),
            fy_fwd: planner.plan_fft_forward(ny),
            fy_inv: planner.plan_fft_inverse(ny),
        }
    }

    pub fn len(&self) -> usize {
        self.nx * self.ny
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn rows(fft: &Arc<dyn Fft<f64>>, data: &mut [Complex64], n: usize) {
        let mut scratch = vec![Complex64::new(0.0, 0.0); fft.get_inplace_scratch_len()];
        for row in data.chunks_mut(n) {
            fft.process_with_scratch(row, &mut scratch);
        }
    }

    fn fft2_with(
        &self,
        data: &mut [Complex64],
        tmp: &mut [Complex64],
        fx: &Arc<dyn Fft<f64>>,
        fy: &Arc<dyn Fft<f64>>,
    ) {
        Self::rows(fx, data, self.nx);
        transpose(data, tmp, self.ny, self.nx); // tmp is (nx, ny)
        Self::rows(fy, tmp, self.ny);
        transpose(tmp, data, self.nx, self.ny); // back to (ny, nx)
    }

    /// In-place unnormalised forward 2D FFT (`numpy.fft.fft2`); `tmp` is a scratch buffer of the same length.
    pub fn fft2(&self, data: &mut [Complex64], tmp: &mut [Complex64]) {
        self.fft2_with(data, tmp, &self.fx_fwd, &self.fy_fwd);
    }

    /// In-place normalised inverse 2D FFT (`numpy.fft.ifft2`).
    pub fn ifft2(&self, data: &mut [Complex64], tmp: &mut [Complex64]) {
        self.fft2_with(data, tmp, &self.fx_inv, &self.fy_inv);
        let s = 1.0 / self.len() as f64;
        data.iter_mut().for_each(|v| *v *= s);
    }

    /// `x` coordinates shifted to `[-lx/2, lx/2)` as in `np.arange(-R, R, dx)`.
    pub fn x_centered(&self) -> Vec<f64> {
        (0..self.nx)
            .map(|i| -0.5 * self.lx + i as f64 * self.dx)
            .collect()
    }

    pub fn y_centered(&self) -> Vec<f64> {
        (0..self.ny)
            .map(|j| -0.5 * self.ly + j as f64 * self.dy)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fft2 of a single Fourier mode is a delta at that mode; ifft2 inverts fft2 (rectangular grid, odd/even mixes).
    #[test]
    fn fft2_roundtrip_and_mode_on_a_rectangle() {
        let g = Grid2D::new(48, 20, 24.0, 10.0);
        let mut tmp = vec![Complex64::new(0.0, 0.0); g.len()];
        let (mx, my) = (3usize, 2usize);
        let mut psi: Vec<Complex64> = (0..g.len())
            .map(|idx| {
                let (ix, iy) = (idx % g.nx, idx / g.nx);
                Complex64::from_polar(
                    1.0,
                    2.0 * std::f64::consts::PI
                        * (mx as f64 * ix as f64 / g.nx as f64
                            + my as f64 * iy as f64 / g.ny as f64),
                )
            })
            .collect();
        let orig = psi.clone();
        g.fft2(&mut psi, &mut tmp);
        let peak = psi[my * g.nx + mx];
        assert!(
            (peak.re - g.len() as f64).abs() < 1e-8 && peak.im.abs() < 1e-8,
            "peak {peak}"
        );
        let off: f64 = psi
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != my * g.nx + mx)
            .map(|(_, v)| v.norm())
            .fold(0.0, f64::max);
        assert!(off < 1e-8, "off-peak {off}");
        g.ifft2(&mut psi, &mut tmp);
        assert!(psi.iter().zip(&orig).all(|(a, b)| (a - b).norm() < 1e-12));
    }
}
