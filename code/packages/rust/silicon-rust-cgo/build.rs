//! build.rs — silicon-rust-cgo
//!
//! This cdylib exports only plain C functions and one struct.  All its
//! dependencies (device-physics, mosfet-models, fab-process-simulation, the
//! Rust standard library) are statically linked into the shared object, so
//! there are no undefined symbols at link time and no platform-specific linker
//! flags are needed.
//!
//! On Linux the resulting .so has runtime deps only on glibc and libgcc_s,
//! both of which are always available.  macOS and Windows similarly depend
//! only on system-provided libraries.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=include/silicon_cgo.h");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
