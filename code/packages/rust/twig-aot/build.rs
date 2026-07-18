//! Build script for `twig-aot`.
//!
//! Compiles `runtime/twig_runtime.c` into a per-host static archive
//! (`libtwig_aot_runtime.a` or `twig_aot_runtime.lib`) using the `cc`
//! crate, then exports its path via `TWIG_RUNTIME_ARCHIVE_<HOST>` env
//! vars so that `src/lib.rs` can embed the right archive bytes with
//! `include_bytes!(env!(...))` at compile time.
//!
//! Implements the runtime-archive layer of
//! [LANG46](../../../specs/LANG46-twig-aot-multi-target.md).
//!
//! ## Per-host build vs. cross-compilation
//!
//! V1 supports only **host-targets-host** AOT:
//!
//! | Host build | Real archive produced | Stub archives |
//! |---|---|---|
//! | macOS ARM64 | macOS ARM64 | linux_x86_64, windows_x86_64 |
//! | Linux x86_64 | linux_x86_64 | macos_arm64, windows_x86_64 |
//! | Windows x86_64 | windows_x86_64 | macos_arm64, linux_x86_64 |
//!
//! Stub archives are a single zero byte.  At AOT compile time,
//! `twig-aot` refuses to emit for a target whose archive is a stub
//! and surfaces a clear error.
//!
//! Cross-OS compilation (e.g. producing a Linux ELF on a Windows host)
//! is deferred — adding it requires bundling cross-toolchains with
//! `twig-aot` or detecting them on the host.  CI verifies each host
//! pipeline on its respective runner (`ubuntu-latest`, `macos-latest`,
//! `windows-latest`).
//!
//! ## Why embed the archive?
//!
//! `twig-aot` is a binary crate that ships as a single executable.
//! At runtime it produces an ELF/Mach-O/PE object file from the user's
//! Twig source and passes it to the system linker.  The object file
//! contains relocation records for runtime helpers like
//! `__twig_print_i64`, declared as undefined externals.
//!
//! For the linker to resolve them it needs the static archive
//! alongside the object file.  Embedding the archive bytes in the
//! `twig-aot` binary and writing them to a temp file at link time is
//! the cleanest approach: no separate installation step, no
//! path-search fragility.

use std::fs;
use std::path::PathBuf;

/// The three hosts LANG46 supports for V1 host-targets-host AOT.
const HOST_KEYS: &[&str] = &[
    "MACOS_ARM64",
    "LINUX_X86_64",
    "WINDOWS_X86_64",
];

fn main() {
    // Re-run this build script if any C runtime source changes.
    println!("cargo:rerun-if-changed=runtime/twig_runtime.c");
    println!("cargo:rerun-if-changed=runtime/dynval_runtime.c");
    // twig_gc.c has been retired (#118b-2b): the GC now comes from the
    // gc-core-capi crate. Re-run if its C-ABI source changes so the embedded
    // gc archive stays fresh.
    println!("cargo:rerun-if-changed=../gc-core-capi/src");

    let out_dir: PathBuf = std::env::var("OUT_DIR")
        .expect("OUT_DIR not set by cargo")
        .into();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // Map the current build host to one of the LANG46 host keys.
    //
    // Returns `None` if the host is something we don't support
    // (e.g. linux on aarch64, freebsd, etc.) — in that case the build
    // proceeds with all three archives as stubs and the developer
    // gets a clear AOT-time error if they try to use `twig-aot`.
    let host_key: Option<&str> = match (target_os.as_str(), target_arch.as_str()) {
        ("macos",   "aarch64") => Some("MACOS_ARM64"),
        ("linux",   "x86_64")  => Some("LINUX_X86_64"),
        ("windows", "x86_64")  => Some("WINDOWS_X86_64"),
        _ => None,
    };

    // Compile the runtime for the host (and only the host).
    //
    // `cc::Build::compile` invokes the platform C compiler and
    // produces a static archive at `$OUT_DIR/<lib_basename>`.  On
    // Unix-likes that's `libtwig_aot_runtime.a`; on MSVC it's
    // `twig_aot_runtime.lib`.
    //
    // Both translation units go into the *same* archive: `twig_runtime.c`
    // (LANG41/75/76 I/O + alloc helpers) and `dynval_runtime.c` (LANG77 the
    // shared lisp value model — cons/symbols/pair?/equal?).  Because
    // `cc::Build::compile` emits `cargo:rustc-link-lib=static=...`, the
    // archive is also linked into `twig-aot`'s own test binary, so the
    // golden test in `src/dynval_runtime_golden.rs` can call the
    // `__dyn_*` functions directly on the host.
    // The garbage collector is no longer a C translation unit here. twig_gc.c
    // has been retired (#118b-2b); the collector now lives in the `gc-core-capi`
    // crate, whose staticlib we build and embed below. dynval_runtime.c's
    // `__twig_gc_alloc` reference is left undefined in this archive and resolved
    // at link time against gc-core-capi (see Part C in src/lib.rs, and the
    // `extern crate gc_core_capi` for twig-aot's own binary/tests).
    if host_key.is_some() {
        cc::Build::new()
            .file("runtime/twig_runtime.c")
            .file("runtime/dynval_runtime.c")
            .compile("twig_aot_runtime");
    }

    // Determine the host archive filename.
    //
    // `cc` does not expose this directly, but the convention is:
    //   - GNU / Unix-like: lib<NAME>.a
    //   - MSVC:            <NAME>.lib
    let host_archive_basename = if target_os == "windows" && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        "twig_aot_runtime.lib"
    } else {
        "libtwig_aot_runtime.a"
    };
    let host_archive_path = out_dir.join(host_archive_basename);

    // For each LANG46 host key:
    //   - If it matches the build host, point its env var at the real
    //     archive.
    //   - Otherwise, write a 1-byte stub file and point the env var
    //     at it.  The driver at AOT time detects the stub and emits a
    //     clean "no runtime archive for X on this host" error.
    for key in HOST_KEYS {
        let env_var = format!("TWIG_RUNTIME_ARCHIVE_{key}");
        if host_key == Some(*key) {
            println!("cargo:rustc-env={env_var}={}", host_archive_path.display());
        } else {
            let stub_path = out_dir.join(format!("twig_runtime_stub_{}.bin",
                                                 key.to_lowercase()));
            // Stub content = single zero byte.  Driver checks length and
            // surfaces a clear error if any caller selects this target
            // on this host.
            fs::write(&stub_path, [0u8]).expect("write runtime stub");
            println!("cargo:rustc-env={env_var}={}", stub_path.display());
        }
    }

    // ── Embed the gc-core-capi staticlib (the native GC) ───────────────────
    //
    // #118b-2b: the collector is now `gc-core-capi`'s `libgc_core_capi.a`.
    // At AOT link time, `src/lib.rs` writes this archive to a temp file and
    // hands it to the linker alongside the runtime archive so the emitted
    // executable's `__twig_gc_alloc` / `__twig_gc_safepoint` references (and
    // dynval_runtime.c's `__twig_gc_alloc`, pulled from the runtime archive)
    // resolve. We build the staticlib here with a nested `cargo build` and
    // copy it into OUT_DIR, then export its path so `include_bytes!` can bake
    // the bytes into the twig-aot binary.
    //
    // Nested-cargo hygiene: use an isolated `--target-dir` under OUT_DIR so we
    // never contend for the outer build's target lock (which would deadlock).
    // `--release` matches the archive we want embedded (small, optimized).
    let gc_archive_dst = out_dir.join("libgc_core_capi.a");
    if host_key.is_some() {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let gc_target = out_dir.join("gc-core-capi-build");
        let status = std::process::Command::new(&cargo)
            .args(["build", "--release", "-p", "gc-core-capi", "--target-dir"])
            .arg(&gc_target)
            // Do not inherit the outer build's CARGO_TARGET_DIR — that would
            // point the nested build back at the locked outer target dir.
            .env_remove("CARGO_TARGET_DIR")
            .status()
            .expect("spawn nested cargo build for gc-core-capi staticlib");
        assert!(status.success(), "gc-core-capi staticlib build failed");

        // Staticlib artifact naming follows the same convention as the runtime
        // archive above: MSVC emits `<name>.lib`, every other toolchain emits
        // `lib<name>.a`. Copy whichever the nested build produced.
        let gc_staticlib_basename = if target_os == "windows"
            && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
        {
            "gc_core_capi.lib"
        } else {
            "libgc_core_capi.a"
        };
        let gc_archive_src = gc_target.join("release").join(gc_staticlib_basename);
        fs::copy(&gc_archive_src, &gc_archive_dst)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}",
                                       gc_archive_src.display(),
                                       gc_archive_dst.display()));
    } else {
        // Unsupported host: mirror the runtime-stub pattern. AOT is refused on
        // this host anyway, so a 1-byte stub keeps `include_bytes!` happy.
        fs::write(&gc_archive_dst, [0u8]).expect("write gc-core-capi stub");
    }
    println!("cargo:rustc-env=GC_CORE_CAPI_ARCHIVE={}", gc_archive_dst.display());

    // Backwards-compatible alias for the host's archive.  The existing
    // macOS ARM64 path uses `TWIG_RUNTIME_ARCHIVE`; keep that name
    // valid (points at the host archive when supported, stub
    // otherwise) so the old `compile_file_macos_arm64` entry point
    // continues to function without changes.
    let legacy_path: PathBuf = if host_key.is_some() {
        host_archive_path
    } else {
        let p = out_dir.join("twig_runtime_stub_legacy.bin");
        fs::write(&p, [0u8]).expect("write legacy stub");
        p
    };
    println!("cargo:rustc-env=TWIG_RUNTIME_ARCHIVE={}", legacy_path.display());
}
