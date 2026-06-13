//! # build.rs — linker flags for the silicon-rust-python cdylib
//!
//! The crate's cdylib references Python C API symbols (Py_DecRef,
//! PyModule_Create2, PyTuple_New, etc.) that aren't present at link
//! time — they're resolved by the embedding CPython interpreter when
//! the .so/.dylib/.pyd is loaded via `import`.
//!
//! Linker behavior differs per OS:
//!
//! * **Linux (GNU ld / gold / lld)**: shared libraries permit undefined
//!   symbols by default — they're left to runtime resolution by the
//!   dynamic linker.  No extra flags needed.
//!
//! * **macOS (ld64 / lld)**: rejects undefined symbols at link time by
//!   default since Big Sur unless `-undefined dynamic_lookup` is passed.
//!   We pass it here so the cdylib links cleanly and the symbols resolve
//!   when CPython dlopen's the .dylib.
//!
//! * **Windows (MSVC / lld-link)**: requires *every* imported symbol to
//!   be declared in a `.lib` import library at link time.  There's no
//!   "undefined dynamic_lookup" equivalent.  CPython ships `python3.lib`
//!   (Limited API, ABI-stable across all Python 3.x) in `<install>/libs/`;
//!   we probe for it via `sysconfig` and emit the right
//!   `rustc-link-search` + `rustc-link-lib` directives.
//!
//!   All the symbols `python-bridge` exports are part of the Limited API
//!   (PEP 384), so linking against `python3.lib` (not the
//!   version-specific `python3X.lib`) gives a single `.pyd` that loads
//!   under every Python 3.x install on Windows.

use std::process::Command;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos"   => emit_macos_link_flags(),
        "windows" => emit_windows_link_flags(),
        _ => {}
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-env-changed=PYTHON_SYS_EXECUTABLE");
}

fn emit_macos_link_flags() {
    println!("cargo:rustc-cdylib-link-arg=-undefined");
    println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
}

fn emit_windows_link_flags() {
    let python = std::env::var("PYO3_PYTHON")
        .or_else(|_| std::env::var("PYTHON_SYS_EXECUTABLE"))
        .ok()
        .unwrap_or_else(|| {
            if Command::new("python").arg("--version").output().is_ok() {
                "python".to_string()
            } else {
                "python3".to_string()
            }
        });

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
            println!(
                "cargo:warning=silicon-rust-python build.rs: could not run `{}` to probe \
                 for Python libs directory.  Set PYO3_PYTHON to an explicit Python path \
                 if the Windows linker fails to find python3.lib.",
                python
            );
            return;
        }
    };

    // Reject paths that contain newlines or null bytes — a compromised Python
    // binary could print a crafted value that injects extra cargo: directives
    // or redirects the linker to a malicious directory.
    if libs_dir.contains('\n') || libs_dir.contains('\r') || libs_dir.contains('\0') {
        println!(
            "cargo:warning=silicon-rust-python build.rs: Python libs directory contains \
             suspicious characters — skipping linker setup.  Set PYO3_PYTHON explicitly."
        );
        return;
    }

    println!("cargo:rustc-link-search=native={}", libs_dir);
    println!("cargo:rustc-link-lib=python3");
}
