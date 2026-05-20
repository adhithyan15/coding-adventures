//! # build.rs — linker flags for the Python C extension cdylib
//!
//! The crate's cdylib references Python C API symbols (Py_DecRef,
//! PyModule_Create2, PyTuple_GetItem, etc.) that aren't present at
//! link time — they're resolved by the embedding CPython
//! interpreter when the .so/.dylib is dlopen'd via `import`.
//!
//! Most linkers handle this fine by default:
//!
//! * **Linux (GNU ld / gold / lld)**: shared libraries permit
//!   undefined symbols by default — they're left to runtime
//!   resolution by the dynamic linker.  No extra flags needed.
//! * **macOS (ld64 / lld)**: rejects undefined symbols at link
//!   time by default since Big Sur unless `-undefined dynamic_lookup`
//!   is passed.  We pass it here so the cdylib links cleanly and
//!   the symbols resolve when CPython dlopen's the .dylib.
//! * **Windows (MSVC)**: requires symbol imports be declared in a
//!   `.lib` import library at link time.  Windows wheels aren't
//!   part of MX09 v0 (deferred — see CHANGELOG).
//!
//! The `cargo:rustc-cdylib-link-arg=` directives only apply to the
//! cdylib target; cargo test binaries (and any rlib targets) are
//! unaffected.  Confirmed locally: `cargo test -p matrix-rust-python`
//! still links cleanly without these flags because the test binary
//! never references the extern symbols (its tests only exercise the
//! pure-Rust core).

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos" {
        // -undefined dynamic_lookup tells ld64 to defer unresolved
        // symbol checking to dlopen / dlsym at runtime, which is
        // exactly the semantics a Python C extension needs (CPython
        // provides the symbols when it loads the .dylib).
        //
        // This pair of flags is the documented mechanism for
        // building Python C extensions on macOS — pyo3 sets the
        // same flags via its own build script, and setuptools'
        // build_ext does the equivalent.
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }

    // Re-run this script if its source changes (cargo's default
    // behavior is fine — no need to declare per-file deps since
    // the script reads no files).
    println!("cargo:rerun-if-changed=build.rs");
}
