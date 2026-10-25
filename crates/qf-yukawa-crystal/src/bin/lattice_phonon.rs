//! Lattice-phonon driver (Tier X numerics) for the QU-1 falsification gate.
//!
//! * `lattice_phonon point POT x y N1,N2,.. RC` prints one CSV row per grid size for the shape `tau = x + i y`.
//! * `lattice_phonon scan POT N1,N2,.. RC TAUFILE OUT.csv` does the same for every `x y` line of TAUFILE with two threads.
//! * `lattice_phonon ewald_static d x y` compares the dipolar static energy with the Ewald sum.
//! * `lattice_phonon frozen POT x y n i j ex ey RC` prints the frozen-phonon second difference and `(1/2) e^T D e`.
//! * `lattice_phonon duffy a x y N` prints the spectral (Poisson + Duffy-Gauss) `E_zp` of `exp(-pi a r^2)`.
//! * `lattice_phonon curv a N h` prints `c = Lap E_zp(rho)/4` at the triangular point for `exp(-pi a r^2)`.
//!
//! Potential specs: `gauss:a`, `yukawa:kappa`, `riesz:s`, `dipolar:d`, `ring:r0,w`, `morse:alpha,r0`, `lj:sigma`,
//! `gdiff:a1,a2,c`. CSV columns: `pot,x,y,n,rc,e_static,e_zp,e_zp_clip,lam_min,n_unstable`.

use qf_yukawa_crystal::dipolar_ewald::DipolarEwald;
use qf_yukawa_crystal::phonon::{
    Analysis, Pot, analyze, dynamical_grid, frozen_phonon_dipolar_ewald, frozen_phonon_pair_sum,
    static_energy,
};
use qf_yukawa_crystal::phonon_spectral::{GaussPoisson, curvature_at_rho_gauss, ezp_gauss_duffy};
use std::fmt::Write as _;
use std::io::Write as _;

const HEADER: &str = "pot,x,y,n,rc,e_static,e_zp,e_zp_clip,lam_min,n_unstable";

fn row(spec: &str, a: &Analysis) -> String {
    format!(
        "{},{:.12},{:.12},{},{},{:.15e},{:.15e},{:.15e},{:.9e},{}",
        spec.replace(',', ";"),
        a.x,
        a.y,
        a.n,
        a.rc,
        a.e_static,
        a.e_zp,
        a.e_zp_clip,
        a.lam_min,
        a.n_unstable
    )
}

fn parse_list(s: &str) -> Result<Vec<usize>, String> {
    s.split(',')
        .map(|t| t.parse::<usize>().map_err(|e| format!("{t}: {e}")))
        .collect()
}

fn scan(spec: &str, ns: &[usize], rc: f64, taus: &[(f64, f64)]) -> Result<Vec<String>, String> {
    let pot = Pot::parse(spec)?;
    let threads = 2usize;
    let chunks: Vec<Vec<(f64, f64)>> = (0..threads)
        .map(|t| taus.iter().skip(t).step_by(threads).copied().collect())
        .collect();
    let mut rows: Vec<(usize, String)> = Vec::new();
    std::thread::scope(|sc| {
        let handles: Vec<_> = chunks
            .iter()
            .enumerate()
            .map(|(t, chunk)| {
                let pot = &pot;
                sc.spawn(move || {
                    let mut local = Vec::new();
                    for (idx, &(x, y)) in chunk.iter().enumerate() {
                        let mut block = String::new();
                        for &n in ns {
                            let a = analyze(pot, x, y, n, rc);
                            let _ = writeln!(block, "{}", row(spec, &a));
                        }
                        local.push((t + idx * threads, block));
                    }
                    local
                })
            })
            .collect();
        for h in handles {
            if let Ok(v) = h.join() {
                rows.extend(v);
            }
        }
    });
    rows.sort_by_key(|r| r.0);
    Ok(rows.into_iter().map(|r| r.1).collect())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |i: usize| -> Result<&str, String> {
        args.get(i)
            .map(String::as_str)
            .ok_or_else(|| format!("missing argument {i}"))
    };
    let f = |i: usize| -> Result<f64, String> {
        arg(i)?.parse::<f64>().map_err(|e| format!("arg {i}: {e}"))
    };
    match arg(1)? {
        "point" => {
            let spec = arg(2)?;
            let ns = parse_list(arg(5)?)?;
            let out = scan(spec, &ns, f(6)?, &[(f(3)?, f(4)?)])?;
            println!("{HEADER}");
            for b in out {
                print!("{b}");
            }
        }
        "scan" => {
            let spec = arg(2)?;
            let ns = parse_list(arg(3)?)?;
            let rc = f(4)?;
            let text = std::fs::read_to_string(arg(5)?).map_err(|e| e.to_string())?;
            let mut taus = Vec::new();
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let mut it = line.split_whitespace();
                let x: f64 = it.next().ok_or("x")?.parse().map_err(|_| "bad x")?;
                let y: f64 = it.next().ok_or("y")?.parse().map_err(|_| "bad y")?;
                taus.push((x, y));
            }
            let out = scan(spec, &ns, rc, &taus)?;
            let mut file = std::fs::File::create(arg(6)?).map_err(|e| e.to_string())?;
            writeln!(file, "{HEADER}").map_err(|e| e.to_string())?;
            for b in out {
                write!(file, "{b}").map_err(|e| e.to_string())?;
            }
        }
        "frozen" => {
            let spec = arg(2)?;
            let pot = Pot::parse(spec)?;
            let (x, y) = (f(3)?, f(4)?);
            let n = arg(5)?.parse::<usize>().map_err(|e| e.to_string())?;
            let ij = (
                arg(6)?.parse::<usize>().map_err(|e| e.to_string())?,
                arg(7)?.parse::<usize>().map_err(|e| e.to_string())?,
            );
            let e = [f(8)?, f(9)?];
            let rc = f(10)?;
            let dm = dynamical_grid(&pot, x, y, n, rc, spec.starts_with("dipolar"));
            let d = dm[ij.0 * n + ij.1];
            let exact = 0.5 * (e[0] * e[0] * d[0] + 2.0 * e[0] * e[1] * d[1] + e[1] * e[1] * d[2]);
            let (f1, f2) = if let Pot::Dipolar { d } = pot {
                (
                    frozen_phonon_dipolar_ewald(d, x, y, n, ij, e, 2e-3),
                    frozen_phonon_dipolar_ewald(d, x, y, n, ij, e, 4e-3),
                )
            } else {
                (
                    frozen_phonon_pair_sum(&pot, (x, y), n, ij, e, 2e-3, rc),
                    frozen_phonon_pair_sum(&pot, (x, y), n, ij, e, 4e-3, rc),
                )
            };
            let rich = (4.0 * f1 - f2) / 3.0;
            println!(
                "{{\"pot\":\"{spec}\",\"x\":{x},\"y\":{y},\"n\":{n},\"ij\":[{},{}],\"e\":[{},{}],\"half_eDe\":{exact:.15e},\"frozen_richardson\":{rich:.15e},\"rel_diff\":{:.3e}}}",
                ij.0,
                ij.1,
                e[0],
                e[1],
                (rich - exact).abs() / exact.abs()
            );
        }
        "ewald_static" => {
            // exact (Ewald) dipolar-exciton static energy of the shape tau, U = V/d^2 units
            let d = f(2)?;
            let (x, y) = (f(3)?, f(4)?);
            let sy = y.sqrt();
            let ew = DipolarEwald::new(d, 0.9, [1.0 / sy, 0.0], [x / sy, sy]);
            let exact = ew.energy_per_particle(&[0.0, 0.0]);
            let mine = static_energy(&Pot::Dipolar { d }, x, y, 400.0);
            println!(
                "{{\"d\":{d},\"x\":{x},\"y\":{y},\"ewald\":{exact:.15e},\"direct_tail_rc400\":{mine:.15e},\"diff\":{:.3e}}}",
                mine - exact
            );
        }
        "duffy" => {
            // spectral (reciprocal-space + Duffy-Gauss) E_zp of the Gaussian exp(-pi a r^2)
            let (a, x, y) = (f(2)?, f(3)?, f(4)?);
            let n = arg(5)?.parse::<usize>().map_err(|e| e.to_string())?;
            let r = ezp_gauss_duffy(a, x, y, n);
            let exact = GaussPoisson::new(a, x, y).mean_trace_exact();
            println!(
                "{{\"a\":{a},\"x\":{x},\"y\":{y},\"n\":{n},\"e_zp\":{:.15e},\"mean_trace\":{:.15e},\"mean_trace_exact\":{exact:.15e},\"min_ratio\":{:.9e}}}",
                r.e_zp, r.mean_trace, r.min_ratio
            );
        }
        "curv" => {
            // c = Lap E_zp(rho)/4 at the triangular point for the Gaussian exp(-pi a r^2)
            let a = f(2)?;
            let n = arg(3)?.parse::<usize>().map_err(|e| e.to_string())?;
            let h = f(4)?;
            println!(
                "{{\"a\":{a},\"n\":{n},\"h\":{h},\"c\":{:.12e}}}",
                curvature_at_rho_gauss(a, n, h)
            );
        }
        other => return Err(format!("unknown command {other}")),
    }
    Ok(())
}
