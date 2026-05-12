//! Build script for generating `no_std` compatible SUNDIALS bindings.
//!
//! Note: The bindings are pre-generated in `src/generated/` to reduce build times
//! and eliminate the `bindgen` and `libclang` dependency for downstream users.
//! If the C headers change, run this script to regenerate them using:
//! `cargo build --features "regenerate-bindings"`

#[cfg(feature = "regenerate-bindings")]
extern crate bindgen;

#[cfg(feature = "regenerate-bindings")]
fn main() {
    use std::env;
    use std::path::PathBuf;

    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point to bindgen, and lets you
    // build up options for the resulting bindings.
    let bindings = bindgen::Builder::new()
        // The input header we would like to generate bindings for.
        .header("wrapper.h")
        
        // --- no_std Configuration ---
        // This is critical for embedded environments.
        // It forces bindgen to use `core::` and `alloc::` instead of `std::`.
        .use_core()
        
        // Define C-types that should use `ctypes` prefix from core::ffi
        .ctypes_prefix("core::ffi")

        // In a strictly no_std environment without an allocator, 
        // we might also disable certain features, but CVODE requires allocation.
        // So we keep `alloc` available.

        // Generate the bindings
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("sundials_bindings_no_std.rs"))
        .expect("Couldn't write bindings!");
}

#[cfg(not(feature = "regenerate-bindings"))]
fn main() {
    // Standard build without regenerating bindings.
    // In a real build script, we would link the SUNDIALS C libraries here.
    // println!("cargo:rustc-link-search=native=/path/to/sundials/lib");
    // println!("cargo:rustc-link-lib=static=sundials_cvode");
}
