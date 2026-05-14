//! Build script for `twig-aot`.
//!
//! Compiles `runtime/twig_runtime.c` into a static archive
//! (`libtwig_aot_runtime.a`) using the `cc` crate, then exports its
//! path via the `TWIG_RUNTIME_ARCHIVE` env var so that `src/lib.rs`
//! can embed the archive bytes with `include_bytes!(env!(...))`.
//!
//! ## Why embed the archive?
//!
//! `twig-aot` is a binary crate that ships as a single executable.
//! At runtime it produces an ARM64 Mach-O object file from the user's
//! Twig source and passes it to the system linker (`ld`).  The object
//! file contains `ARM64_RELOC_BRANCH26` records for runtime helpers like
//! `__twig_print_i64` — symbols declared as undefined externals.
//!
//! For `ld` to resolve them it needs the static archive alongside the
//! object file.  Embedding the archive bytes in the `twig-aot` binary
//! and writing them to a temp file at link time is the cleanest approach:
//! no separate installation step, no path-search fragility.
//!
//! ## Portability
//!
//! `cc::Build` selects the platform C compiler automatically:
//! - macOS: `clang` targeting `arm64-apple-macos`
//! - Linux: `gcc` or `clang` targeting the host
//!
//! The C source (`twig_runtime.c`) uses only `<stdio.h>` / `<stdint.h>`,
//! so it compiles cleanly on any POSIX host without modification.

fn main() {
    // Re-run this build script if the C source changes.
    println!("cargo:rerun-if-changed=runtime/twig_runtime.c");

    // Compile `twig_runtime.c` to `$OUT_DIR/libtwig_aot_runtime.a`.
    //
    // `cc::Build::compile` handles invoking the C compiler, archiving
    // the object file, and printing the necessary `cargo:rustc-link-*`
    // directives.  We don't actually want to link the archive INTO the
    // `twig-aot` binary (it's for the linker to use at AOT link time),
    // so we suppress the link directive using `cargo:warning` suppression
    // — the archive is only accessed via `include_bytes!` below.
    cc::Build::new()
        .file("runtime/twig_runtime.c")
        .compile("twig_aot_runtime");

    // Export the archive path so `src/lib.rs` can `include_bytes!` it.
    //
    // `cargo:rustc-env` sets an env var that is visible to `env!(...)` macros
    // evaluated at *compile* time (not runtime).  `include_bytes!` uses this
    // to bake the archive bytes into the binary.
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    println!("cargo:rustc-env=TWIG_RUNTIME_ARCHIVE={out_dir}/libtwig_aot_runtime.a");
}
