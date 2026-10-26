//! CVODE cross-check (Tier X, independent of any proof assistant) of the `table` stage of the
//! finite interval certificate in the openai/math manuscript "Triangular minimality for planar
//! Coulomb renormalized energy" (2026-09-23, AI-generated, not peer reviewed; files
//! `verification/core.hpp`, SHA-256 `c7f76596...`).
//!
//! # The ODE
//!
//! `table(24000)` (core.hpp:246-267) propagates, serially from `t0 = small = p - 0.06` to `l`, a
//! state `S = (s1, s2, ps_k)` (`k = 6, 12, ..., 228`, 38 keys) by a midpoint rule with an outward
//! remainder. Reading the step (core.hpp:254-259) and its derivative form `solve` in `fh`
//! (core.hpp:230), the state solves
//!
//! ```text
//! s1'   = t b(t)
//! s2'   = -2 b~(t) + t b~(t)^2
//! ps_k' = b~(t) - k ps_k / t          (integrating factor (old/x)^k in the C++ step)
//! ```
//!
//! with `b(t) = basis(t, seg).first` (core.hpp:182-200; `seg` = the segment of the step's left
//! node) and `b~ = b' + 1/t`, from the initial state `ss0(small)` (core.hpp:215-222), with the
//! basis switching at the breakpoints `small, p, R, R+.001, R+.075, R+.076`.
//!
//! Here `b` and `b'` are evaluated in f64 with forward-mode duals from the same coefficients
//! (midpoints of the C++ intervals for `c_m`, `Tp0..Tp2`, edge values, `VR`, `alp`, `beta`), the
//! ODE is integrated with `crates/cvode` (BDF, restarted at every breakpoint), and the result is
//! compared with the certificate's exact enclosures `pointS[i]` (dumped from the unmodified C++
//! by AutoevolveAI's `results/proposal2/2026-10-26/A1-continuation/cpp/dump2_driver.cpp`, each
//! endpoint rounded outward to f64; `data/table_states.txt`).
//!
//! Two comparisons are made:
//! * **global**: one chained integration from `ss0(small)` (a fresh CVODE solve per output node,
//!   because many outputs from one BDF run were measured to degrade accuracy in this repository's
//!   CVODE, see `qf-bao-distances`);
//! * **node to node**: from the midpoint of the enclosure at node `i-1` to node `i` (the local
//!   propagation the certificate encloses at each step).
//!
//! Neither comparison proves anything about the certificate: f64 and CVODE carry no error bound.
//! An f64 value inside a 144-bit enclosure is evidence that the enclosure and the ODE reading
//! agree; a value outside it by more than the solver error would point to a wrong reading, a
//! transcription error, or an enclosure that misses the solution.

use cvode::{Cvode, Method, Task};
use nvector::SerialVector;
use std::ops::{Add, Mul, Neg, Sub};

/// Number of lattice modes `c_1..c_38`.
pub const M: usize = 38;
/// Number of state components: `s1, s2` and 38 `ps_k`.
pub const NSTATE: usize = 2 + M;

/// The 38 distinct keys `k = 6 m` of `ps_k`.
pub fn ks() -> [f64; M] {
    let mut k = [0.0; M];
    for (i, x) in k.iter_mut().enumerate() {
        *x = 6.0 * (i as f64 + 1.0);
    }
    k
}

/// A first-order forward-mode dual number `v + d eps`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dual {
    pub v: f64,
    pub d: f64,
}

impl Dual {
    pub fn var(v: f64) -> Self {
        Dual { v, d: 1.0 }
    }
    pub fn cst(v: f64) -> Self {
        Dual { v, d: 0.0 }
    }
    pub fn ln(self) -> Self {
        Dual {
            v: self.v.ln(),
            d: self.d / self.v,
        }
    }
    pub fn powi(self, k: i32) -> Self {
        if k == 0 {
            return Dual::cst(1.0);
        }
        Dual {
            v: self.v.powi(k),
            d: f64::from(k) * self.v.powi(k - 1) * self.d,
        }
    }
    pub fn scale(self, c: f64) -> Self {
        Dual {
            v: self.v * c,
            d: self.d * c,
        }
    }
}

impl Add for Dual {
    type Output = Dual;
    fn add(self, o: Dual) -> Dual {
        Dual {
            v: self.v + o.v,
            d: self.d + o.d,
        }
    }
}
impl Sub for Dual {
    type Output = Dual;
    fn sub(self, o: Dual) -> Dual {
        Dual {
            v: self.v - o.v,
            d: self.d - o.d,
        }
    }
}
impl Mul for Dual {
    type Output = Dual;
    fn mul(self, o: Dual) -> Dual {
        Dual {
            v: self.v * o.v,
            d: self.v * o.d + self.d * o.v,
        }
    }
}
impl Neg for Dual {
    type Output = Dual;
    fn neg(self) -> Dual {
        Dual {
            v: -self.v,
            d: -self.d,
        }
    }
}

/// Coefficients of mode `m` (1-based) from `init_after_co` (core.hpp:292-297).
#[derive(Clone, Debug, Default)]
pub struct Mode {
    pub co: f64,
    pub tp0: f64,
    pub tp1: f64,
    pub tp2: f64,
    pub vr: f64,
    pub alp: f64,
    pub beta: f64,
    pub edge: Vec<f64>,
}

/// Constants and per-mode coefficients of `basis`.
#[derive(Clone, Debug, Default)]
pub struct Coeffs {
    pub pi: f64,
    pub p: f64,
    pub r: f64,
    pub small: f64,
    pub cut: f64,
    pub v: f64,
    /// `-log R + (pi/2) R^2`
    pub b0c: f64,
    /// `-1/R + pi R`
    pub cr1: f64,
    /// `1/R^2 + pi`
    pub cr2: f64,
    pub modes: Vec<Mode>,
}

/// The certificate's enclosure of the state at one table node (outward f64 box).
#[derive(Clone, Debug)]
pub struct NodeBox {
    pub index: usize,
    /// segment of the node (`orig[i]`)
    pub orig: usize,
    pub t: f64,
    pub lo: Vec<f64>,
    pub hi: Vec<f64>,
}

/// Everything read from `data/table_states.txt`.
#[derive(Clone, Debug)]
pub struct TableData {
    pub coeffs: Coeffs,
    /// `ss0(small)`: `s1, s2, ps_6..ps_228` (midpoints)
    pub init: Vec<f64>,
    pub nodes: Vec<NodeBox>,
}

fn num(tok: Option<&str>, what: &str) -> Result<f64, String> {
    tok.ok_or_else(|| format!("missing {what}"))?
        .parse::<f64>()
        .map_err(|e| format!("bad {what}: {e}"))
}

/// Parse the data file.
pub fn parse(text: &str) -> Result<TableData, String> {
    let mut c = Coeffs {
        modes: vec![Mode::default(); M],
        ..Coeffs::default()
    };
    let mut init = Vec::new();
    let mut nodes = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("const") => {
                let name = it.next().ok_or("const without name")?;
                let val = num(it.next(), name)?;
                match name {
                    "pi" => c.pi = val,
                    "p" => c.p = val,
                    "R" => c.r = val,
                    "small" => c.small = val,
                    "cut" => c.cut = val,
                    "v" => c.v = val,
                    "b0c" => c.b0c = val,
                    "cR1" => c.cr1 = val,
                    "cR2" => c.cr2 = val,
                    _ => return Err(format!("unknown constant {name}")),
                }
            }
            Some("mode") => {
                let m = num(it.next(), "mode index")? as usize;
                let md = c.modes.get_mut(m - 1).ok_or("mode index out of range")?;
                md.co = num(it.next(), "co")?;
                md.tp0 = num(it.next(), "tp0")?;
                md.tp1 = num(it.next(), "tp1")?;
                md.tp2 = num(it.next(), "tp2")?;
                md.vr = num(it.next(), "vr")?;
                md.alp = num(it.next(), "alp")?;
                md.beta = num(it.next(), "beta")?;
            }
            Some("edge") => {
                let m = num(it.next(), "edge index")? as usize;
                let md = c.modes.get_mut(m - 1).ok_or("edge index out of range")?;
                md.edge = it
                    .map(|s| s.parse::<f64>().map_err(|e| e.to_string()))
                    .collect::<Result<_, _>>()?;
            }
            Some("init") => {
                init = it
                    .map(|s| s.parse::<f64>().map_err(|e| e.to_string()))
                    .collect::<Result<_, _>>()?;
            }
            Some("node") => {
                let index = num(it.next(), "node index")? as usize;
                let orig = num(it.next(), "orig")? as usize;
                let t = num(it.next(), "t")?;
                let v: Vec<f64> = it
                    .map(|s| s.parse::<f64>().map_err(|e| e.to_string()))
                    .collect::<Result<_, _>>()?;
                if v.len() != 2 * NSTATE {
                    return Err(format!("node {index}: {} numbers", v.len()));
                }
                let lo = v.iter().step_by(2).copied().collect();
                let hi = v.iter().skip(1).step_by(2).copied().collect();
                nodes.push(NodeBox {
                    index,
                    orig,
                    t,
                    lo,
                    hi,
                });
            }
            _ => {}
        }
    }
    if init.len() != NSTATE {
        return Err(format!("init has {} components", init.len()));
    }
    Ok(TableData {
        coeffs: c,
        init,
        nodes,
    })
}

/// The data file shipped with the crate.
pub fn load_default() -> Result<TableData, String> {
    parse(include_str!("../data/table_states.txt"))
}

/// `sm(x, sel)` (core.hpp:178): `0` (`sel < 0`), `1` (`sel > 0`), else `10x^3 - 15x^4 + 6x^5`.
fn sm(x: Dual, sel: i32) -> Dual {
    if sel < 0 {
        return Dual::cst(0.0);
    }
    if sel > 0 {
        return Dual::cst(1.0);
    }
    x.powi(3).scale(10.0) - x.powi(4).scale(15.0) + x.powi(5).scale(6.0)
}

/// `PQt(t)` (core.hpp:179) at `t` (here `t - R`).
fn pqt(c: &Coeffs, t: Dual) -> (Dual, Dual) {
    let h = t.scale(1.0 / c.v);
    let p = (h - h.powi(3).scale(6.0) + h.powi(4).scale(8.0) - h.powi(5).scale(3.0)).scale(c.v);
    let q = (h.powi(2) - h.powi(3).scale(3.0) + h.powi(4).scale(3.0) - h.powi(5))
        .scale(c.v * c.v / 2.0);
    (p, q)
}

/// `B(t) = basis(t, seg).first` (core.hpp:182-200) as a dual number (value and `d/dt`).
pub fn basis(c: &Coeffs, t: Dual, seg: usize) -> Dual {
    let g = -t.ln() + (t * t).scale(c.pi / 2.0);
    let (pr, qr) = pqt(c, t - Dual::cst(c.r));
    let b0 = if seg < 3 {
        g
    } else if seg < 5 {
        Dual::cst(c.b0c) + pr.scale(c.cr1) + qr.scale(c.cr2)
    } else {
        Dual::cst(c.b0c)
    };
    let sel = |zero: bool, below: bool| -> i32 {
        if zero {
            0
        } else if below {
            -1
        } else {
            1
        }
    };
    let cu = sm(
        (t - Dual::cst(c.small)).scale(1.0 / c.cut),
        sel(seg == 1, seg < 1),
    );
    let gc = sm(
        (t - Dual::cst(c.r)).scale(1.0 / 0.10),
        sel((3..=6).contains(&seg), seg < 3),
    );
    let cer = sm(
        (t - Dual::cst(c.r) - Dual::cst(0.001)).scale(1.0 / 0.075),
        sel(seg == 4 || seg == 5, seg < 4),
    );
    let one_gc = Dual::cst(1.0) - gc;
    let mut smm = Dual::cst(0.0);
    let mut sbm = Dual::cst(0.0);
    let mut sd = b0 - g;
    for (i, md) in c.modes.iter().enumerate() {
        let k = 6 * (i as i32 + 1);
        let bm = match seg {
            0 => Dual::cst(0.0),
            1 => {
                let h = t - Dual::cst(c.p);
                Dual::cst(md.tp0) + h.scale(md.tp1) + (h * h).scale(md.tp2)
            }
            2 => {
                let z = t * t - Dual::cst(c.p * c.p);
                md.edge
                    .iter()
                    .rev()
                    .fold(Dual::cst(0.0), |acc, &e| acc * z + Dual::cst(e))
            }
            3 | 4 => Dual::cst(md.vr) + pr.scale(md.alp) + qr.scale(md.beta),
            _ => Dual::cst(md.vr),
        } * cu;
        let sign = if (i + 1) % 2 == 1 { -1.0 } else { 1.0 };
        let mm = t.powi(k).scale(md.co * sign) * one_gc;
        smm = smm + mm;
        sbm = sbm + bm;
        sd = sd + (bm - mm);
    }
    if seg < 4 {
        g + smm
    } else if seg >= 6 {
        b0 + sbm
    } else {
        g + smm + cer * sd
    }
}

/// Perturbation of the right-hand side for negative controls: `b, b~` multiplied by
/// `1 + b_rel`, and `k` in `ps_k' = b~ - k ps_k / t` replaced by `k (1 + k_rel)`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Perturb {
    pub b_rel: f64,
    pub k_rel: f64,
}

/// The propagation right-hand side on segment `seg`.
pub fn rhs(c: &Coeffs, seg: usize, pert: Perturb, t: f64, y: &[f64], ydot: &mut [f64]) {
    let b = basis(c, Dual::var(t), seg);
    let bv = b.v * (1.0 + pert.b_rel);
    let bt = (b.d + 1.0 / t) * (1.0 + pert.b_rel);
    ydot[0] = t * bv;
    ydot[1] = -2.0 * bt + t * bt * bt;
    for (j, k) in ks().iter().enumerate() {
        ydot[2 + j] = bt - k * (1.0 + pert.k_rel) * y[2 + j] / t;
    }
}

/// Solver settings.
#[derive(Clone, Copy, Debug)]
pub struct Tol {
    pub rtol: f64,
    pub atol: f64,
}

/// One fresh CVODE (BDF) solve on segment `seg` from `(t0, y0)` to `t1`.
pub fn propagate(
    c: &Coeffs,
    seg: usize,
    pert: Perturb,
    tol: Tol,
    t0: f64,
    y0: &[f64],
    t1: f64,
) -> Result<Vec<f64>, String> {
    if t1 <= t0 {
        return Ok(y0.to_vec());
    }
    let cc = c.clone();
    let f = move |t: f64, y: &[f64], ydot: &mut [f64]| -> Result<(), String> {
        rhs(&cc, seg, pert, t, y, ydot);
        Ok(())
    };
    let mut solver = Cvode::builder(Method::Bdf)
        .rtol(tol.rtol)
        .atol(tol.atol)
        .max_steps(1_000_000)
        .build(f, t0, SerialVector::from_slice(y0))
        .map_err(|e| format!("CVODE setup failed: {e}"))?;
    let (t, y) = solver
        .solve(t1, Task::Normal)
        .map_err(|e| format!("CVODE failed on segment {seg} [{t0}, {t1}]: {e}"))?;
    if (t - t1).abs() > 1e-12 {
        return Err(format!("CVODE stopped at {t}, not {t1}"));
    }
    Ok(y.to_vec())
}

/// Position of an f64 value relative to an enclosure: `(y - mid) / halfwidth` (inside iff
/// `|.| <= 1`, up to the outward f64 rounding of the box).
pub fn rel_pos(y: f64, lo: f64, hi: f64) -> f64 {
    let mid = 0.5 * (lo + hi);
    let hw = 0.5 * (hi - lo);
    if hw > 0.0 {
        (y - mid) / hw
    } else if y == mid {
        0.0
    } else {
        f64::INFINITY
    }
}

/// Result of a comparison over all checked nodes.
#[derive(Clone, Debug, Default)]
pub struct Verdict {
    pub nodes: usize,
    pub values: usize,
    pub outside: usize,
    /// per component group `[s1, s2, ps]`: values outside, max |rel_pos|, max |y - nearest
    /// endpoint| over the outside values
    pub outside_by_group: [usize; 3],
    pub max_abs_rel_pos: [f64; 3],
    pub max_excess: [f64; 3],
    pub first_outside: Option<(usize, usize)>,
}

impl Verdict {
    fn record(&mut self, node: usize, comp: usize, y: f64, lo: f64, hi: f64) {
        let g = comp.min(2);
        let rp = rel_pos(y, lo, hi);
        self.values += 1;
        if rp.abs() > self.max_abs_rel_pos[g] {
            self.max_abs_rel_pos[g] = rp.abs();
        }
        if y < lo || y > hi {
            self.outside += 1;
            self.outside_by_group[g] += 1;
            let ex = if y < lo { lo - y } else { y - hi };
            if ex > self.max_excess[g] {
                self.max_excess[g] = ex;
            }
            if self.first_outside.is_none() {
                self.first_outside = Some((node, comp));
            }
        }
    }
}

fn segment_starts(nodes: &[NodeBox]) -> Vec<usize> {
    // indices into `nodes` (sorted by table index) where the step segment (orig of the left node)
    // changes: a node is a segment start if its orig differs from its predecessor's
    let mut s = vec![0];
    for w in 1..nodes.len() {
        if nodes[w].orig != nodes[w - 1].orig {
            s.push(w);
        }
    }
    s
}

/// Global comparison: chained integration from `ss0(small)` at the first node; at every node a
/// fresh solve from the start of its segment; the segment-start states are chained.
pub fn check_global(d: &TableData, pert: Perturb, tol: Tol) -> Result<Verdict, String> {
    let nodes = &d.nodes;
    let starts = segment_starts(nodes);
    let mut v = Verdict::default();
    let mut y_start = d.init.clone();
    for (si, &s) in starts.iter().enumerate() {
        let end = starts.get(si + 1).copied().unwrap_or(nodes.len() - 1);
        let seg = nodes[s].orig;
        let t0 = nodes[s].t;
        for nb in &nodes[s..=end] {
            let y = propagate(&d.coeffs, seg, pert, tol, t0, &y_start, nb.t)?;
            v.nodes += 1;
            for (comp, &yy) in y.iter().enumerate() {
                v.record(nb.index, comp, yy, nb.lo[comp], nb.hi[comp]);
            }
        }
        if si + 1 < starts.len() {
            y_start = propagate(&d.coeffs, seg, pert, tol, t0, &y_start, nodes[end].t)?;
        }
    }
    Ok(v)
}

/// Node-to-node comparison: for every node whose table predecessor is also in the data, start at
/// the midpoint of the predecessor's enclosure and integrate to the node on the predecessor's
/// segment.
pub fn check_local(d: &TableData, pert: Perturb, tol: Tol) -> Result<Verdict, String> {
    let mut v = Verdict::default();
    for w in 1..d.nodes.len() {
        let (a, b) = (&d.nodes[w - 1], &d.nodes[w]);
        if a.index + 1 != b.index {
            continue;
        }
        let y0: Vec<f64> = a.lo.iter().zip(&a.hi).map(|(l, h)| 0.5 * (l + h)).collect();
        let y = propagate(&d.coeffs, a.orig, pert, tol, a.t, &y0, b.t)?;
        v.nodes += 1;
        for (comp, &yy) in y.iter().enumerate() {
            v.record(b.index, comp, yy, b.lo[comp], b.hi[comp]);
        }
    }
    Ok(v)
}

/// Solver self-consistency: the largest `|y_a - y_b|` per group `[s1, s2, ps]` between two
/// tolerance settings over the global integration (an empirical estimate of the CVODE error).
pub fn global_drift(d: &TableData, a: Tol, b: Tol) -> Result<[f64; 3], String> {
    let nodes = &d.nodes;
    let starts = segment_starts(nodes);
    let mut out = [0.0f64; 3];
    let mut ya = d.init.clone();
    let mut yb = d.init.clone();
    for (si, &s) in starts.iter().enumerate() {
        let end = starts.get(si + 1).copied().unwrap_or(nodes.len() - 1);
        let seg = nodes[s].orig;
        let t0 = nodes[s].t;
        for nb in &nodes[s..=end] {
            let va = propagate(&d.coeffs, seg, Perturb::default(), a, t0, &ya, nb.t)?;
            let vb = propagate(&d.coeffs, seg, Perturb::default(), b, t0, &yb, nb.t)?;
            for (comp, (x, y)) in va.iter().zip(&vb).enumerate() {
                let g = comp.min(2);
                out[g] = out[g].max((x - y).abs());
            }
        }
        if si + 1 < starts.len() {
            ya = propagate(&d.coeffs, seg, Perturb::default(), a, t0, &ya, nodes[end].t)?;
            yb = propagate(&d.coeffs, seg, Perturb::default(), b, t0, &yb, nodes[end].t)?;
        }
    }
    Ok(out)
}
