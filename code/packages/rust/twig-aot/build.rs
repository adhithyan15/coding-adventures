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
    // Re-run this build script if the C source changes.
    println!("cargo:rerun-if-changed=runtime/twig_runtime.c");

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
    if host_key.is_some() {
        cc::Build::new()
            .file("runtime/twig_runtime.c")
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
