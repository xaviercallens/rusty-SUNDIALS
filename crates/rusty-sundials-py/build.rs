fn main() {
    // pyo3's `extension-module` feature deliberately does not link libpython: the symbols it
    // references (e.g. `PyExc_RuntimeError`) are provided by the Python process that dlopen()s
    // this cdylib at runtime, not by an on-disk libpython at link time. `maturin` knows to relax
    // symbol resolution for that; a plain `cargo build`/`cargo test` does not, and macOS's linker
    // (unlike Linux's) refuses to produce a dylib with symbols left unresolved, so it fails with
    // "undefined symbols for architecture". Tell it to defer resolution to load time instead.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
}
