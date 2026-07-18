//! Shared test helpers for the lang-aot integration tests.
//!
//! Files under `tests/common/` are **not** compiled as their own test target
//! (only top-level `tests/*.rs` are), so this module can be `mod common;`-included
//! by any integration test without emitting a spurious empty test binary.

use std::path::PathBuf;
use std::process::Command;

/// Build gc-core-capi's release staticlib and return the path to
/// `libgc_core_capi.a` in the workspace target directory.
///
/// #118b-2b retired `twig-aot/runtime/twig_gc.c`; the garbage collector now lives
/// in the `gc-core-capi` crate. The LLVM/WASM emit tests used to hand `twig_gc.c`
/// to `clang` alongside `dynval_runtime.c` + `twig_runtime.c`; they now link this
/// Rust `staticlib` instead, which exports the same `__twig_gc_*` ABI (plus the
/// generic `__gc_*` names) that the emitted code and `dynval_runtime.c` reference.
///
/// The nested `cargo build` is a cache hit after the first call; concurrent calls
/// serialise on cargo's own target-dir lock, so it is safe to call from parallel
/// test threads.
pub fn gc_core_capi_archive() -> PathBuf {
    // `CARGO_MANIFEST_DIR` for a lang-aot test is `.../code/packages/rust/lang-aot`;
    // its parent is the workspace root that owns the shared `target/` dir.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest.parent().expect("lang-aot has a parent dir");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(&cargo)
        .args(["build", "--release", "-p", "gc-core-capi"])
        .current_dir(workspace_root)
        .status()
        .expect("spawn cargo build for gc-core-capi staticlib");
    assert!(status.success(), "gc-core-capi staticlib build failed");

    // Staticlib naming is toolchain-specific: MSVC emits `gc_core_capi.lib`,
    // every other toolchain emits `libgc_core_capi.a`. Return whichever exists.
    let release = workspace_root.join("target").join("release");
    let unix = release.join("libgc_core_capi.a");
    let msvc = release.join("gc_core_capi.lib");
    if msvc.exists() && !unix.exists() {
        msvc
    } else {
        unix
    }
}

/// The `clang`/`cc` arguments that replace the retired `twig_gc.c` object: the
/// gc-core-capi staticlib, followed by the system libraries its bundled Rust std
/// needs. On Linux that is `-lpthread -ldl`; macOS provides both through
/// `libSystem`, so no extra flags (and `-ldl` would error — there is no libdl).
///
/// Ordering: the archive comes first so the linker can satisfy
/// `dynval_runtime.o`'s undefined `__twig_gc_alloc` from it, then the system libs
/// resolve the archive's own std references.
pub fn gc_link_args() -> Vec<String> {
    let mut args = vec![gc_core_capi_archive().to_string_lossy().into_owned()];
    if cfg!(target_os = "linux") {
        args.push("-lpthread".into());
        args.push("-ldl".into());
    }
    args
}
