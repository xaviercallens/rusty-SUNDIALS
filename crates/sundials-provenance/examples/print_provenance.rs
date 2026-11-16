//! Prints the build provenance as JSON. With `--require-pinned`, exits 3 unless pinned.
fn main() {
    let p = sundials_provenance::build_info();
    println!("{}", p.to_json());
    if std::env::args().any(|a| a == "--require-pinned") && !p.is_pinned() {
        eprintln!("build is not pinned (dirty tree, unknown SHA, or git unavailable)");
        std::process::exit(3);
    }
}
