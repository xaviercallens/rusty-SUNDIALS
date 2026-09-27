use qf_cmb_cascade::{r_bound_2sigma, sigma_cv, sigma_ell};
use std::time::Instant;

fn main() {
    let t0 = Instant::now();
    println!("=== Exact-spectrum r_bound_2sigma grid (Planck bars) ===");
    println!(
        "{:>8} {:>5} {:>9} {:>16}",
        "beta/H*", "zpt", "ell_peak", "r_2sigma"
    );
    for &zpt in &[0.1, 0.2] {
        for &beta_h in &[10.0, 20.0, 50.0, 100.0, 200.0, 500.0] {
            let (r, ell) = r_bound_2sigma(zpt, beta_h, 5.99, 0.1, sigma_ell);
            println!("{:8.0} {:5.2} {:9} {:16.6}", beta_h, zpt, ell, r);
        }
    }
    println!("\n=== Fisher forecast: Planck bars vs. cosmic-variance floor ===");
    println!(
        "{:>8} {:>5} {:>18} {:>20} {:>11}",
        "beta/H*", "zpt", "r(Planck)", "r(CV floor)", "tightening"
    );
    for &zpt in &[0.1, 0.2] {
        for &beta_h in &[10.0, 50.0, 100.0, 200.0, 500.0] {
            let (r_planck, _) = r_bound_2sigma(zpt, beta_h, 5.99, 0.1, sigma_ell);
            let (r_cv, _) = r_bound_2sigma(zpt, beta_h, 5.99, 0.1, sigma_cv);
            println!(
                "{:8.0} {:5.2} {:18.6} {:20.6} {:10.2}x",
                beta_h,
                zpt,
                r_planck,
                r_cv,
                r_planck / r_cv
            );
        }
    }
    println!("\ntotal wall time: {:?}", t0.elapsed());
}
