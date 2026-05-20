//! # build.rs — linker flags for the Python C extension cdylib
//!
//! The crate's cdylib references Python C API symbols (Py_DecRef,
//! PyModule_Create2, PyTuple_GetItem, etc.) that aren't present at
//! link time — they're resolved by the embedding CPython
//! interpreter when the .so/.dylib/.pyd is loaded via `import`.
//!
//! Linker behavior differs per OS:
//!
//! * **Linux (GNU ld / gold / lld)**: shared libraries permit
//!   undefined symbols by default — they're left to runtime
//!   resolution by the dynamic linker.  No extra flags needed.
//!
//! * **macOS (ld64 / lld)**: rejects undefined symbols at link
//!   time by default since Big Sur unless `-undefined dynamic_lookup`
//!   is passed.  We pass it here so the cdylib links cleanly and
//!   the symbols resolve when CPython dlopen's the .dylib.
//!
//! * **Windows (MSVC / lld-link)**: requires *every* imported symbol
//!   to be declared in a `.lib` import library at link time.  There's
//!   no "undefined dynamic_lookup" equivalent.  CPython ships
//!   `python3.lib` (Limited API, ABI-stable across all Python 3.x)
//!   in `<install>/libs/`; we probe for it via `sysconfig` and emit
//!   the right `rustc-link-search` + `rustc-link-lib` directives.
//!
//!   All the symbols `python-bridge` exports are part of the Limited
//!   API (PEP 384), so linking against `python3.lib` (not the
//!   version-specific `python3X.lib`) gives us one `.pyd` that loads
//!   under every Python 3.x install on Windows.  See
//!   <https://docs.python.org/3/c-api/stable.html>.
//!
//! The `cargo:rustc-cdylib-link-arg=` directives only apply to the
//! cdylib target; cargo test binaries (and any rlib targets) are
//! unaffected.  Confirmed locally: `cargo test -p matrix-rust-python`
//! still links cleanly without these flags because the test binary
//! never references the extern symbols (its tests only exercise the
//! pure-Rust core).

use std::process::Command;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => emit_macos_link_flags(),
        "windows" => emit_windows_link_flags(),
        // Linux ELF default behavior is already permissive — no
        // action needed.  Same for any other Unix-y target.
        _ => {}
    }

    // Re-run this script if its source changes (cargo's default
    // behavior is fine — no need to declare per-file deps since the
    // script reads no files).  We DO want to re-run if the env vars
    // we read change, so declare those explicitly.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-env-changed=PYTHON_SYS_EXECUTABLE");
}

/// macOS-specific cdylib linker flags.
///
/// `-undefined dynamic_lookup` tells ld64 to defer unresolved symbol
/// checking to dlopen / dlsym at runtime, which is exactly the
/// semantics a Python C extension needs (CPython provides the
/// symbols when it loads the .dylib).
///
/// This pair of flags is the documented mechanism for building
/// Python C extensions on macOS — pyo3 sets the same flags via its
/// own build script, and setuptools' build_ext does the equivalent.
fn emit_macos_link_flags() {
    println!("cargo:rustc-cdylib-link-arg=-undefined");
    println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
}

/// Windows-specific cdylib linker setup.
///
/// Windows linkers (MSVC link.exe, lld-link) require every imported
/// symbol to be resolved against a `.lib` at link time.  CPython on
/// Windows ships `python3.lib` (the Limited API stub) in
/// `<install>/libs/` — pointing the linker at it satisfies all of
/// python-bridge's extern declarations.
///
/// We probe for Python's lib directory by running the host `python`
/// (or `python3`) and querying `sysconfig`.  The honored env vars
/// match the pyo3 / maturin convention so the same setup works in
/// both ecosystems:
///
///   * `PYO3_PYTHON`            (preferred — explicit Python path)
///   * `PYTHON_SYS_EXECUTABLE`  (legacy rust-cpython convention)
///   * default `python` on PATH (CI usually has `actions/setup-python`)
///
/// If the probe fails (no Python on PATH, sysconfig failure, etc.)
/// we emit no link directives and let the linker fail with a clear
/// "unresolved external symbol _PyModule_Create2" message — which
/// is more debuggable than silently building a broken `.pyd`.
fn emit_windows_link_flags() {
    let python = std::env::var("PYO3_PYTHON")
        .or_else(|_| std::env::var("PYTHON_SYS_EXECUTABLE"))
        .ok()
        .unwrap_or_else(|| {
            // Try `python` first (Windows default), then `python3`
            // (the POSIX-style symlink if Python was installed via
            // chocolatey / scoop / etc.).
            if Command::new("python").arg("--version").output().is_ok() {
                "python".to_string()
            } else {
                "python3".to_string()
            }
        });

    // One python invocation returns BOTH the libs dir and a sanity
    // marker so we don't have to spawn twice.  Format on success:
    //
    //   <libs-directory>\n
    //
    // sysconfig.get_paths()['data'] points at the Python install
    // root (e.g. `C:\Python311`).  Its `libs/` subdirectory contains
    // `python3.lib` (Limited API) and `python311.lib` (full ABI).
    let probe = Command::new(&python)
        .args([
            "-c",
            "import sysconfig, os; print(os.path.join(sysconfig.get_paths()['data'], 'libs'))",
        ])
        .output();

    let libs_dir = match probe {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => {
            // Couldn't run python — let the linker fail with its own
            // "unresolved external" message rather than silently
            // building a broken .pyd.
            println!(
                "cargo:warning=matrix-rust-python build.rs: could not run `{}` to probe \
                 for Python libs directory.  Set PYO3_PYTHON to an explicit Python path \
                 if the Windows linker fails to find python3.lib.",
                python
            );
            return;
        }
    };

    println!("cargo:rustc-link-search=native={}", libs_dir);

    // Link against `python3.lib` (Limited API).  Every Python C API
    // symbol python-bridge declares is part of the Limited API since
    // 3.2 (PEP 384), so the resulting `.pyd` is ABI-compatible with
    // every CPython 3.x install — no per-version rebuild needed.
    //
    // If we ever start using non-Limited-API symbols, switch this to
    // `python3X` (version-specific) and add the version probe.
    println!("cargo:rustc-link-lib=python3");
}
