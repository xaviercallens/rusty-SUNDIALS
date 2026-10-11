//! Ground-truth energy floor and global optimizers for the periodic 2D Yukawa crystal.
//!
//! # Energy floor (a one-sided correctness check)
//!
//! For a pair potential `f(r^2)` with `f` completely monotone, planar universal optimality (the statement cited in
//! `results/preregistration.json`) says that no periodic configuration of density `rho` has a lower energy per
//! particle than the triangular lattice of the same density. The exact Yukawa interaction `exp(-kappa r)/r` is in
//! that class; the shifted-force potential used by [`System`] for dynamics is **not** (it has compact support and is
//! everywhere *below* the exact potential), so the floor must never be compared with `System::energy`.
//!
//! [`floor_check`] therefore recomputes the energy of a periodic configuration with the exact potential, summed over
//! all periodic images closer than `r_exact`, and compares it with the exact triangular lattice sum truncated at the
//! same radius. Because every term is positive, truncation can only lower either side, and the dropped tail of the
//! configuration is bounded rigorously (see [`tail_bound`]). A configuration is reported `violated` only when
//!
//! ```text
//! e_config(r_exact) + tail_bound + roundoff  <  e_tri(r_exact) - roundoff  <=  e_tri(exact)
//! ```
//!
//! i.e. only when its exact energy is certainly below the triangular lattice. That cannot happen for a correct
//! pipeline, so `violated == true` means a bug (sign flip, wrong density, truncation used as if exact, overlapping
//! images, ...). The check is one-sided: `violated == false` proves nothing about whether a run found the minimum.
//! It applies to `Geometry::Periodic` only: finite clusters (`Bowl`, `Wall`) have boundaries and their energy per
//! particle is not bounded by the bulk lattice value.
//!
//! Scope of the cited statement (`AutoevolveAI` worktree `openai-math-discovery`,
//! `formal_cert/Triangular`, `universal_energy_minimum` and `AnyDensity.lean`): `g` smooth, non-negative and
//! completely monotone in `t = r^2`; any locally finite configuration of density `rho`; energy per particle over
//! ordered pairs (this module uses the conventional factor 1/2 on both sides, which does not change the comparison).
//! That repository records that the proof is AI-generated and not peer reviewed. The 1/r jellium of [`crate::jellium`]
//! is outside this statement (Riesz `s = 1` with a background), so no floor is provided for it.
//!
//! # Global optimizers
//!
//! The CVODE gradient flow ([`relax`]) and the overdamped Langevin anneal ([`anneal`]) are local or quasi-local
//! searches; on a flat (soft, small-`kappa`) landscape they can stop in defected local minima. [`basin_hopping`]
//! (random perturbation + CVODE quench + Metropolis acceptance on quenched energies) and [`parallel_tempering`]
//! (replica-exchange Langevin with periodic CVODE quenches of the coldest replica) are added as heuristic global
//! searches built on the same primitives. They come with no guarantee of reaching the lattice; results produced
//! with them are exploratory (they postdate the YUK-TRI-1 preregistration).

use crate::{Geometry, Relaxed, Rng, System, anneal, relax};

/// Exact (untruncated) Yukawa pair potential.
pub fn yukawa(kappa: f64, r: f64) -> f64 {
    (-kappa * r).exp() / r
}

/// Unit roundoff of `f64`.
const U: f64 = f64::EPSILON / 2.0;

/// Bound on the accumulated relative roundoff of a sum of `terms` positive terms, each computed with a few
/// correctly rounded operations (`exp`, division): `2 (terms + 1) u`.
fn sum_roundoff(terms: usize, value: f64) -> f64 {
    2.0 * (terms as f64 + 1.0) * U * value.abs()
}

/// Upper bound on the exact-energy tail per particle, `(1/2N) sum_i sum_{images, r >= r_exact} V(r)`, for any
/// configuration of `N` points in a periodic `lx x ly` cell (density `rho = N / (lx ly)`).
///
/// The number of image points within distance `r` of a particle is at most `pi rho (r + D)^2` with `D` the cell
/// diagonal (each image of a cell lies inside the disk of radius `r + D`). Integrating by parts against the
/// decreasing `V` and using `(r + D)^2 / r^2 <= (1 + D/R)^2` for `r >= R` gives
/// `sum_{r >= R} V <= pi rho (1 + D/R)^2 exp(-kappa R) (R + 2/kappa)`.
pub fn tail_bound(kappa: f64, rho: f64, lx: f64, ly: f64, r_exact: f64) -> f64 {
    let d = (lx * lx + ly * ly).sqrt();
    0.5 * std::f64::consts::PI
        * rho
        * (1.0 + d / r_exact).powi(2)
        * (-kappa * r_exact).exp()
        * (r_exact + 2.0 / kappa)
}

/// Exact Yukawa energy per particle of a periodic configuration, `(1/2N) sum_i sum_j sum_{v}' V(|x_i - x_j + v|)`
/// over cell translations `v` with distance `< r_exact` (the self term `i = j, v = 0` excluded).
/// Returns the energy and the number of terms summed.
pub fn periodic_energy_exact(
    kappa: f64,
    lx: f64,
    ly: f64,
    x: &[f64],
    r_exact: f64,
) -> (f64, usize) {
    let n = x.len() / 2;
    let mx = (r_exact / lx).ceil() as i64 + 1;
    let my = (r_exact / ly).ceil() as i64 + 1;
    let r2max = r_exact * r_exact;
    let (mut s, mut terms) = (0.0, 0usize);
    for i in 0..n {
        for j in 0..n {
            let (mut dx0, mut dy0) = (x[2 * i] - x[2 * j], x[2 * i + 1] - x[2 * j + 1]);
            dx0 -= lx * (dx0 / lx).round();
            dy0 -= ly * (dy0 / ly).round();
            for p in -mx..=mx {
                for q in -my..=my {
                    if i == j && p == 0 && q == 0 {
                        continue;
                    }
                    let dx = dx0 + p as f64 * lx;
                    let dy = dy0 + q as f64 * ly;
                    let r2 = dx * dx + dy * dy;
                    if r2 < r2max {
                        s += yukawa(kappa, r2.sqrt());
                        terms += 1;
                    }
                }
            }
        }
    }
    (0.5 * s / n as f64, terms)
}

/// Exact Yukawa energy per particle of the triangular lattice of density `rho`, truncated at `r_exact`
/// (a lower bound on the untruncated value). Returns the energy and the number of terms summed.
pub fn triangular_energy_exact(kappa: f64, rho: f64, r_exact: f64) -> (f64, usize) {
    let a = (2.0 / (3.0_f64.sqrt() * rho)).sqrt();
    let (b0, b1) = ([a, 0.0], [0.5 * a, 0.5 * 3.0_f64.sqrt() * a]);
    let m = (r_exact / b1[1]).ceil() as i64 + 2;
    let r2max = r_exact * r_exact;
    let (mut s, mut terms) = (0.0, 0usize);
    for i in -m..=m {
        for j in -m..=m {
            if i == 0 && j == 0 {
                continue;
            }
            let vx = i as f64 * b0[0] + j as f64 * b1[0];
            let vy = i as f64 * b0[1] + j as f64 * b1[1];
            let r2 = vx * vx + vy * vy;
            if r2 < r2max {
                s += yukawa(kappa, r2.sqrt());
                terms += 1;
            }
        }
    }
    (0.5 * s, terms)
}

/// Outcome of a comparison against the triangular-lattice floor.
#[derive(Clone, Copy, Debug)]
pub struct FloorCheck {
    /// Number density `N / (lx ly)`.
    pub density: f64,
    /// Exact energy per particle of the configuration, truncated at `r_exact` (or the reported value).
    pub e_config: f64,
    /// Exact triangular-lattice energy per particle at the same density, truncated at `r_exact`.
    pub e_floor: f64,
    /// Rigorous bound on the configuration's dropped tail (zero for a reported energy).
    pub tail_bound: f64,
    /// Total slack allowed before declaring a violation (tail + roundoff of both sums).
    pub tolerance: f64,
    /// `e_config - e_floor`; positive for any configuration other than the lattice (up to the tolerance).
    pub margin: f64,
    /// `true` when the exact energy is certainly below the triangular lattice: a bug, never physics.
    pub violated: bool,
}

/// Certify that a periodic configuration's exact Yukawa energy is not below the triangular lattice (see module docs).
pub fn floor_check(sys: &System, x: &[f64], r_exact: f64) -> Result<FloorCheck, String> {
    let Geometry::Periodic { lx, ly } = sys.geometry else {
        return Err("floor check applies to periodic configurations only".into());
    };
    let n = x.len() / 2;
    if n == 0 || r_exact <= 0.0 || sys.kappa <= 0.0 {
        return Err("floor check needs particles, r_exact > 0 and kappa > 0".into());
    }
    let rho = n as f64 / (lx * ly);
    let (e_config, n_cfg) = periodic_energy_exact(sys.kappa, lx, ly, x, r_exact);
    let (e_floor, n_lat) = triangular_energy_exact(sys.kappa, rho, r_exact);
    let tail = tail_bound(sys.kappa, rho, lx, ly, r_exact);
    let tolerance = tail + sum_roundoff(n_cfg, e_config) + sum_roundoff(n_lat, e_floor);
    Ok(FloorCheck {
        density: rho,
        e_config,
        e_floor,
        tail_bound: tail,
        tolerance,
        margin: e_config - e_floor,
        violated: e_config + tolerance < e_floor,
    })
}

/// Check an energy per particle *reported by some solver* as the exact Yukawa energy of a periodic configuration of
/// density `rho`. `violated == true` means the report is impossible (below the triangular lattice). Do not pass
/// energies of the shifted-force potential: they are systematically lower than the exact ones and will be flagged.
pub fn check_reported_energy(kappa: f64, rho: f64, e_reported: f64, r_exact: f64) -> FloorCheck {
    let (e_floor, n_lat) = triangular_energy_exact(kappa, rho, r_exact);
    let tolerance = sum_roundoff(n_lat, e_floor) + 4.0 * U * e_reported.abs();
    FloorCheck {
        density: rho,
        e_config: e_reported,
        e_floor,
        tail_bound: 0.0,
        tolerance,
        margin: e_reported - e_floor,
        violated: e_reported + tolerance < e_floor,
    }
}

/// Result of a global search. Energies are totals of the dynamics potential (`System::energy`).
#[derive(Clone, Debug)]
pub struct GlobalResult {
    /// Lowest quenched configuration found.
    pub best: Relaxed,
    pub best_energy: f64,
    /// Energy of the plain CVODE quench of the starting configuration (the baseline being improved on).
    pub start_energy: f64,
    /// Best energy so far, recorded after each hop / quench.
    pub trace: Vec<f64>,
    pub accepted: usize,
    pub proposals: usize,
    /// CVODE quenches that returned an error (skipped).
    pub failures: usize,
}

/// Basin-hopping parameters.
#[derive(Clone, Copy, Debug)]
pub struct HopParams {
    pub hops: usize,
    /// Standard deviation of the Gaussian displacement of each moved particle.
    pub sigma: f64,
    /// Move only particles within this distance of a random particle (`None`: move all).
    pub radius: Option<f64>,
    /// Reject a perturbation (without quenching) that brings a moved particle closer than this to any other one;
    /// near-overlaps make the gradient flow extremely stiff.
    pub min_sep: f64,
    /// Metropolis temperature for the quenched (total) energies.
    pub temperature: f64,
    pub ftol: f64,
    pub tmax: f64,
    pub seed: u64,
}

/// Basin hopping: perturb the current minimum, quench with CVODE, accept by Metropolis, keep the best.
pub fn basin_hopping(sys: &System, x0: &[f64], p: &HopParams) -> Result<GlobalResult, String> {
    let mut rng = Rng::new(p.seed);
    let n = x0.len() / 2;
    let first = relax(sys, x0, p.ftol, p.tmax)?;
    let start_energy = sys.energy(&first.x);
    let (mut cur_x, mut cur_e) = (first.x.clone(), start_energy);
    let (mut best, mut best_energy) = (first, start_energy);
    let mut trace = vec![best_energy];
    let (mut accepted, mut failures) = (0usize, 0usize);
    for _ in 0..p.hops {
        let mut trial = cur_x.clone();
        let mut moved_idx = Vec::new();
        let c = ((rng.uniform() * n as f64) as usize).min(n - 1);
        for i in 0..n {
            let moved = match p.radius {
                None => true,
                Some(rad) => {
                    let (dx, dy) = sys.delta(
                        cur_x[2 * i],
                        cur_x[2 * i + 1],
                        cur_x[2 * c],
                        cur_x[2 * c + 1],
                    );
                    dx * dx + dy * dy <= rad * rad
                }
            };
            if moved {
                trial[2 * i] += p.sigma * rng.gauss();
                trial[2 * i + 1] += p.sigma * rng.gauss();
                moved_idx.push(i);
            }
        }
        sys.wrap(&mut trial);
        let overlap = moved_idx.iter().any(|&i| {
            (0..n).any(|j| {
                j != i && {
                    let (dx, dy) = sys.delta(
                        trial[2 * i],
                        trial[2 * i + 1],
                        trial[2 * j],
                        trial[2 * j + 1],
                    );
                    dx * dx + dy * dy < p.min_sep * p.min_sep
                }
            })
        });
        if overlap {
            trace.push(best_energy);
            continue;
        }
        let r = match relax(sys, &trial, p.ftol, p.tmax) {
            Ok(r) => r,
            Err(_) => {
                failures += 1;
                trace.push(best_energy);
                continue;
            }
        };
        let e = sys.energy(&r.x);
        if e <= cur_e || rng.uniform() < (-(e - cur_e) / p.temperature).exp() {
            accepted += 1;
            cur_x = r.x.clone();
            cur_e = e;
        }
        if e < best_energy {
            best_energy = e;
            best = r;
        }
        trace.push(best_energy);
    }
    Ok(GlobalResult {
        best,
        best_energy,
        start_energy,
        trace,
        accepted,
        proposals: p.hops,
        failures,
    })
}

/// Geometric temperature ladder `t_min .. t_max` with `n >= 2` rungs, ascending.
///
/// Swaps between neighbouring rungs are accepted with probability about `exp(-N (r - 1)^2 / r)` for ratio `r`
/// (total-energy fluctuations grow with the particle number `N`), so `r` must be about `1 + 1/sqrt(N)`; a coarse
/// ladder (e.g. 8 rungs over two decades at `N = 288`) accepts essentially no swaps.
pub fn temperature_ladder(t_min: f64, t_max: f64, n: usize) -> Vec<f64> {
    assert!(n >= 2 && t_min > 0.0 && t_max > t_min);
    let ratio = (t_max / t_min).powf(1.0 / (n - 1) as f64);
    (0..n).map(|k| t_min * ratio.powi(k as i32)).collect()
}

/// Replica-exchange acceptance for swapping the configurations held at inverse temperatures `beta_i`, `beta_j`
/// with energies `e_i`, `e_j`: `min(1, exp((beta_i - beta_j)(e_i - e_j)))`.
pub fn swap_probability(beta_i: f64, beta_j: f64, e_i: f64, e_j: f64) -> f64 {
    ((beta_i - beta_j) * (e_i - e_j)).exp().min(1.0)
}

/// Parallel-tempering parameters.
#[derive(Clone, Debug)]
pub struct TemperParams {
    /// Ascending temperatures, one replica each (`temps[0]` is the coldest).
    pub temps: Vec<f64>,
    pub rounds: usize,
    /// Langevin steps per replica between swap attempts.
    pub steps_per_round: usize,
    pub dt: f64,
    /// Per-step displacement cap (as in [`anneal`]).
    pub cap: f64,
    /// Quench the coldest replica with CVODE every this many rounds (and after the last round).
    pub quench_every: usize,
    pub ftol: f64,
    pub tmax: f64,
    pub seed: u64,
    /// Worker threads for propagating replicas.
    pub threads: usize,
}

/// Advance each replica by constant-temperature overdamped Langevin dynamics.
#[allow(clippy::too_many_arguments)]
fn propagate(
    sys: &System,
    reps: &mut [Vec<f64>],
    temps: &[f64],
    seeds: &[u64],
    steps: usize,
    dt: f64,
    cap: f64,
    threads: usize,
) {
    let chunk = reps.len().div_ceil(threads.max(1)).max(1);
    std::thread::scope(|sc| {
        for ((xs, ts), ss) in reps
            .chunks_mut(chunk)
            .zip(temps.chunks(chunk))
            .zip(seeds.chunks(chunk))
        {
            sc.spawn(move || {
                for ((x, &t), &s) in xs.iter_mut().zip(ts).zip(ss) {
                    let y = anneal(sys, x, t, t, steps, dt, cap, s);
                    *x = y;
                }
            });
        }
    });
}

/// Parallel tempering (replica exchange) on overdamped Langevin dynamics, all replicas starting from the CVODE
/// quench of `x0`; neighbouring temperatures attempt swaps after every round (even/odd pairs alternating), and the
/// coldest replica is quenched with CVODE every `quench_every` rounds. Returns the best quenched configuration.
pub fn parallel_tempering(
    sys: &System,
    x0: &[f64],
    p: &TemperParams,
) -> Result<GlobalResult, String> {
    let nrep = p.temps.len();
    if nrep < 2 || p.temps.windows(2).any(|w| w[1] <= w[0]) {
        return Err("parallel tempering needs >= 2 strictly ascending temperatures".into());
    }
    let first = relax(sys, x0, p.ftol, p.tmax)?;
    let start_energy = sys.energy(&first.x);
    let mut reps = vec![first.x.clone(); nrep];
    let (mut best, mut best_energy) = (first, start_energy);
    let mut trace = vec![best_energy];
    let mut rng = Rng::new(p.seed ^ 0x5BD1_E995);
    let (mut accepted, mut proposals, mut failures) = (0usize, 0usize, 0usize);
    let every = p.quench_every.max(1);
    for round in 0..p.rounds {
        let seeds: Vec<u64> = (0..nrep)
            .map(|k| {
                p.seed
                    .wrapping_mul(1_000_003)
                    .wrapping_add((round * nrep + k) as u64 + 1)
            })
            .collect();
        propagate(
            sys,
            &mut reps,
            &p.temps,
            &seeds,
            p.steps_per_round,
            p.dt,
            p.cap,
            p.threads,
        );
        let mut e: Vec<f64> = reps.iter().map(|x| sys.energy(x)).collect();
        let mut k = round % 2;
        while k + 1 < nrep {
            proposals += 1;
            let pr = swap_probability(1.0 / p.temps[k], 1.0 / p.temps[k + 1], e[k], e[k + 1]);
            if rng.uniform() < pr {
                accepted += 1;
                reps.swap(k, k + 1);
                e.swap(k, k + 1);
            }
            k += 2;
        }
        if (round + 1) % every == 0 || round + 1 == p.rounds {
            match relax(sys, &reps[0], p.ftol, p.tmax) {
                Ok(r) => {
                    let eq = sys.energy(&r.x);
                    if eq < best_energy {
                        best_energy = eq;
                        best = r;
                    }
                }
                Err(_) => failures += 1,
            }
            trace.push(best_energy);
        }
    }
    Ok(GlobalResult {
        best,
        best_energy,
        start_energy,
        trace,
        accepted,
        proposals,
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{spacing_density_one, triangular, uniform_config};

    /// 36 particles holding a 6 x 6 triangular lattice at density 1; `rc = 2.5 < min(lx, ly) / 2 ~ 2.79`
    /// (no pair at the minimum-image tie `ly / 2`).
    fn small_box(kappa: f64) -> (System, Vec<f64>) {
        let a = spacing_density_one();
        let sys = System {
            kappa,
            rc: 2.5,
            geometry: Geometry::Periodic {
                lx: 6.0 * a,
                ly: 3.0 * 3.0_f64.sqrt() * a,
            },
        };
        (sys, triangular(6, 6, a))
    }

    const N_SMALL: usize = 36;

    const R_EXACT: f64 = 12.3;

    #[test]
    fn perfect_lattice_sits_on_the_floor() {
        for kappa in [1.0, 2.0] {
            let (sys, x) = small_box(kappa);
            let c = floor_check(&sys, &x, R_EXACT).unwrap();
            assert!((c.density - 1.0).abs() < 1e-12, "density {}", c.density);
            assert!(!c.violated, "{c:?}");
            assert!(c.margin.abs() < 1e-9 * c.e_floor, "{c:?}");
        }
    }

    #[test]
    fn tail_bound_is_negligible_at_the_working_radius() {
        let a = spacing_density_one();
        let t = tail_bound(1.0, 1.0, 16.0 * a, 9.0 * 3.0_f64.sqrt() * a, 40.0);
        assert!(t > 0.0 && t < 1e-14, "tail {t}");
    }

    #[test]
    fn random_and_relaxed_configurations_stay_above_the_floor() {
        let (sys, x) = small_box(2.0);
        let a = spacing_density_one();
        let xr = uniform_config(&sys, N_SMALL, 0.5 * a, None, 21);
        let c = floor_check(&sys, &xr, R_EXACT).unwrap();
        assert!(!c.violated && c.margin > 0.0, "{c:?}");

        let mut xn = x.clone();
        let mut rng = Rng::new(4);
        xn.iter_mut().for_each(|v| *v += 0.1 * a * rng.gauss());
        let r = relax(&sys, &xn, 1e-8, 1e7).unwrap();
        let c = floor_check(&sys, &r.x, R_EXACT).unwrap();
        assert!(!c.violated, "{c:?}");
    }

    #[test]
    fn sign_flip_is_caught() {
        let (e_tri, _) = triangular_energy_exact(2.0, 1.0, R_EXACT);
        assert!(check_reported_energy(2.0, 1.0, -e_tri, R_EXACT).violated);
        assert!(!check_reported_energy(2.0, 1.0, e_tri * (1.0 + 1e-6), R_EXACT).violated);
    }

    #[test]
    fn shifted_force_energy_is_not_a_valid_input_to_the_floor() {
        // V_sf < V for every r, so reporting the dynamics energy as if exact is flagged: the misuse the module warns
        // against (and the reason `floor_check` recomputes energies itself).
        let (sys, x) = small_box(1.0);
        let e_sf = sys.energy(&x) / N_SMALL as f64;
        assert!(check_reported_energy(1.0, 1.0, e_sf, R_EXACT).violated);
    }

    #[test]
    fn floor_check_rejects_finite_clusters() {
        let sys = System {
            kappa: 2.0,
            rc: 2.0,
            geometry: Geometry::Bowl { k: 0.02 },
        };
        assert!(floor_check(&sys, &[0.0, 0.0, 1.0, 0.0], R_EXACT).is_err());
    }

    #[test]
    fn swap_rule_matches_metropolis() {
        // colder replica (larger beta) holding the higher energy: always swap
        assert_eq!(swap_probability(10.0, 5.0, 2.0, 1.0), 1.0);
        let p = swap_probability(10.0, 5.0, 1.0, 2.0);
        assert!((p - (-5.0_f64).exp()).abs() < 1e-15);
        let t = temperature_ladder(1e-3, 1e-1, 5);
        assert!((t[0] - 1e-3).abs() < 1e-15 && (t[4] - 1e-1).abs() < 1e-12);
    }

    #[test]
    fn basin_hopping_never_worsens_the_quench() {
        let (sys, _) = small_box(2.0);
        let a = spacing_density_one();
        let x0 = uniform_config(&sys, N_SMALL, 0.5 * a, None, 8);
        let p = HopParams {
            hops: 10,
            sigma: 0.15 * a,
            radius: None,
            min_sep: 0.3 * a,
            temperature: 0.02,
            ftol: 1e-8,
            tmax: 1e7,
            seed: 1,
        };
        let r = basin_hopping(&sys, &x0, &p).unwrap();
        assert!(r.best_energy <= r.start_energy);
        assert!(r.trace.windows(2).all(|w| w[1] <= w[0]));
        assert!(!floor_check(&sys, &r.best.x, R_EXACT).unwrap().violated);
    }

    #[test]
    fn parallel_tempering_never_worsens_the_quench() {
        let (sys, _) = small_box(2.0);
        let a = spacing_density_one();
        let x0 = uniform_config(&sys, N_SMALL, 0.5 * a, None, 9);
        let p = TemperParams {
            temps: temperature_ladder(1e-3, 3e-2, 4),
            rounds: 20,
            steps_per_round: 50,
            dt: 0.01,
            cap: 0.05,
            quench_every: 5,
            ftol: 1e-8,
            tmax: 1e7,
            seed: 2,
            threads: 2,
        };
        let r = parallel_tempering(&sys, &x0, &p).unwrap();
        assert!(r.best_energy <= r.start_energy);
        // 4 replicas: even rounds try pairs (0,1),(2,3), odd rounds (1,2); 10 + 10 rounds.
        assert_eq!(r.proposals, 30);
        assert!(!floor_check(&sys, &r.best.x, R_EXACT).unwrap().violated);
    }

    /// Heuristic (not guaranteed): basin hopping finds the 36-particle triangular lattice from a random start.
    #[test]
    #[ignore]
    fn basin_hopping_reaches_the_small_lattice() {
        let (sys, x) = small_box(2.0);
        let a = spacing_density_one();
        let e_tri = sys.energy(&x);
        let x0 = uniform_config(&sys, N_SMALL, 0.5 * a, None, 8);
        let p = HopParams {
            hops: 200,
            sigma: 0.2 * a,
            radius: None,
            min_sep: 0.3 * a,
            temperature: 0.02,
            ftol: 1e-9,
            tmax: 1e7,
            seed: 3,
        };
        let r = basin_hopping(&sys, &x0, &p).unwrap();
        assert!(
            (r.best_energy - e_tri).abs() < 1e-8 * e_tri.abs(),
            "best {} tri {e_tri}",
            r.best_energy
        );
    }
}
