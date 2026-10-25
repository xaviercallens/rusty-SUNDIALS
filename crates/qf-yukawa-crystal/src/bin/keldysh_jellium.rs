//! Keldysh-jellium Madelung energies of the triangular and square 2D Wigner crystals (Tier X numerics).
//!
//! `keldysh_jellium scan OUT.json`: potential checks, controls, and the r0 scan at density one.
//! `keldysh_jellium points IN.txt OUT.json`: E_tri, E_sq at density one for each r0 listed in IN.txt (one per line).

use qf_yukawa_crystal::jellium::{ETA_SQUARE, ETA_TRIANGULAR, Jellium};
use qf_yukawa_crystal::keldysh::{
    Keldysh, madelung, madelung_parts, square_basis, triangular_basis,
};
use qf_yukawa_crystal::{spacing_density_one, triangular};
use std::f64::consts::PI;
use std::fmt::Write as _;

const ALPHA: f64 = 1.6;

fn arr(v: &[f64]) -> String {
    let parts: Vec<String> = v.iter().map(|x| format!("{x:.17e}")).collect();
    format!("[{}]", parts.join(", "))
}

fn energies(r0: f64) -> (f64, f64) {
    let (t1, t2) = triangular_basis();
    let (s1, s2) = square_basis();
    let k = Keldysh { r0 };
    (madelung(&k, t1, t2, ALPHA), madelung(&k, s1, s2, ALPHA))
}

fn scan(out: &str) -> Result<(), String> {
    let mut j = String::from("{\n");
    // 1. Potential: DLMF 11.5.2 quadrature and Gaussian mixture, at the radii of mpmath_refs.py.
    let mut rows = Vec::new();
    for r0 in [0.05, 0.3, 1.0, 4.0] {
        for r in [0.1, 0.5, 1.0, 1.0745699318, 2.0, 5.0, 12.0] {
            let k = Keldysh { r0 };
            rows.push(format!(
                "[{r0:.17e}, {r:.17e}, {:.17e}, {:.17e}]",
                k.potential_dlmf(r),
                k.potential_mixture(r)
            ));
        }
    }
    writeln!(
        j,
        " \"potential_r0_r_dlmf_mixture\": [{}],",
        rows.join(", ")
    )
    .unwrap();

    // 2. Kernel identity Vhat_sr + Vhat_lr = 2 pi/(k(1 + r0 k)).
    let mut kerr: f64 = 0.0;
    for r0 in [0.0, 0.05, 1.0, 10.0] {
        let k = Keldysh { r0 };
        for q in [0.1, 1.0, 5.0, 20.0] {
            let astar = 1.0 / (4.0 * ALPHA * ALPHA);
            let s = k.vhat_short(q, astar) + k.vhat_long(q, astar);
            kerr = kerr.max(((s - k.kernel(q)) / k.kernel(q)).abs());
        }
    }
    writeln!(j, " \"kernel_identity_max_rel_err\": {kerr:.6e},").unwrap();

    // 3. Positive control: Coulomb limit, same convention as jellium.rs.
    let (ec_t, ec_s) = energies(0.0);
    let a = spacing_density_one();
    let jel = Jellium::new(16.0 * a, 9.0 * 3.0_f64.sqrt() * a, 0.66, 8.0, 7.0);
    let ebox = jel.energy(&triangular(16, 18, a)) / 288.0;
    writeln!(
        j,
        " \"coulomb\": {{\"lattice_r0_0_tri\": {ec_t:.17e}, \"lattice_r0_0_sq\": {ec_s:.17e}, \"jellium_rs_box_tri\": {ebox:.17e}, \"literature_tri\": {:.17e}, \"literature_sq\": {:.17e}}},",
        ETA_TRIANGULAR * PI.sqrt(),
        ETA_SQUARE * PI.sqrt()
    )
    .unwrap();
    let small = [1e-10, 1e-9, 1e-8, 1e-6, 1e-4, 1e-3, 1e-2];
    let mut lim = Vec::new();
    for &r0 in &small {
        let (t, s) = energies(r0);
        lim.push(format!(
            "{{\"r0\": {r0:.3e}, \"dE_tri\": {:.17e}, \"dE_sq\": {:.17e}, \"slope_tri_over_pi\": {:.12e}, \"slope_sq_over_pi\": {:.12e}}}",
            t - ec_t,
            s - ec_s,
            (t - ec_t) / r0 / PI,
            (s - ec_s) / r0 / PI
        ));
    }
    writeln!(j, " \"coulomb_limit\": [{}],", lim.join(", ")).unwrap();

    // 4. Ewald-parameter independence (positive) and negative controls.
    let (t1, t2) = triangular_basis();
    let (s1, s2) = square_basis();
    let mut ind = Vec::new();
    for r0 in [0.0, 0.01, 0.1, 1.0, 10.0, 100.0, 1e4] {
        for (name, b1, b2) in [("tri", t1, t2), ("sq", s1, s2)] {
            let k = Keldysh { r0 };
            let mut e = Vec::new();
            let (mut wrong_recip, mut no_bg) = (Vec::new(), Vec::new());
            for al in [0.8, 1.6, 3.2] {
                let p = madelung_parts(&k, b1, b2, al, 46.0);
                e.push(p.total());
                wrong_recip.push(p.real - p.recip + p.self_term + p.background);
                no_bg.push(p.real + p.recip + p.self_term);
            }
            let spread = |v: &[f64]| {
                let mx = v.iter().cloned().fold(f64::MIN, f64::max);
                let mn = v.iter().cloned().fold(f64::MAX, f64::min);
                mx - mn
            };
            ind.push(format!(
                "{{\"r0\": {r0}, \"lattice\": \"{name}\", \"E_alpha_0.8_1.6_3.2\": {}, \"spread\": {:.3e}, \"neg_wrong_sign_recip_spread\": {:.3e}, \"neg_no_background_spread\": {:.3e}}}",
                arr(&e),
                spread(&e),
                spread(&wrong_recip),
                spread(&no_bg)
            ));
        }
    }
    writeln!(
        j,
        " \"alpha_independence_and_negative_controls\": [{}],",
        ind.join(", ")
    )
    .unwrap();
    // Negative control (c): the whole kernel with the wrong sign, -2 pi/(k(1 + r0 k)), at r0 -> 0.
    let wrong_whole = -energies(1e-9).0;
    writeln!(
        j,
        " \"neg_wrong_sign_kernel_coulomb_limit\": {{\"E\": {wrong_whole:.17e}, \"deviation_from_coulomb\": {:.6e}}},",
        (wrong_whole - ETA_TRIANGULAR * PI.sqrt()).abs()
    )
    .unwrap();

    // 5. Scaling check: E(density n, r0) = sqrt(n) E(1, r0 sqrt(n)), with n = 4.
    let k = Keldysh { r0: 0.7 };
    let (b1, b2) = ([0.5 * t1[0], 0.5 * t1[1]], [0.5 * t2[0], 0.5 * t2[1]]);
    let e4 = madelung(&k, b1, b2, 3.7);
    let e1 = energies(1.4).0;
    writeln!(
        j,
        " \"scaling_n4\": {{\"E_n4_r0_0.7\": {e4:.17e}, \"2_E_n1_r0_1.4\": {:.17e}, \"rel_diff\": {:.3e}}},",
        2.0 * e1,
        ((e4 - 2.0 * e1) / e4).abs()
    )
    .unwrap();

    // 6. r0 scan at density one.
    let mut sc = Vec::new();
    for i in 0..=50 {
        let r0 = 10f64.powf(-3.0 + 5.0 * i as f64 / 50.0);
        let (t, s) = energies(r0);
        sc.push(format!(
            "{{\"r0\": {r0:.17e}, \"E_tri\": {t:.17e}, \"E_sq\": {s:.17e}, \"gap_sq_minus_tri\": {:.17e}}}",
            s - t
        ));
    }
    writeln!(j, " \"scan\": [{}]", sc.join(",\n  ")).unwrap();
    j.push('}');
    std::fs::write(out, j).map_err(|e| e.to_string())
}

fn points(inp: &str, out: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(inp).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let r0: f64 = line.trim().parse().map_err(|e| format!("{line}: {e}"))?;
        let (t, s) = energies(r0);
        rows.push(format!("[{r0:.17e}, {t:.17e}, {s:.17e}]"));
    }
    std::fs::write(
        out,
        format!(
            "{{\"alpha\": {ALPHA}, \"r0_Etri_Esq\": [{}]}}",
            rows.join(",\n ")
        ),
    )
    .map_err(|e| e.to_string())
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("scan") if args.len() == 3 => scan(&args[2]),
        Some("points") if args.len() == 4 => points(&args[2], &args[3]),
        _ => Err("usage: keldysh_jellium scan OUT.json | points IN.txt OUT.json".into()),
    }
}
