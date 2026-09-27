use std::time::Instant;
fn main() {
    let t0 = Instant::now();
    let (r, ell) = qf_cmb_cascade::r_bound_2sigma(0.1, 100.0, 5.99, 0.1, qf_cmb_cascade::sigma_ell);
    println!("r_bound(beta/H=100) = {r}, ell_peak={ell}, took {:?}", t0.elapsed());
}
