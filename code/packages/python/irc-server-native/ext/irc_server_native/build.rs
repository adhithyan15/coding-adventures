// build.rs — linker configuration for the Python C extension cdylib.
//
// Python C extensions (.so / .pyd) are loaded via dlopen() by the Python
// interpreter at import time. The Python C API symbols (Py_IncRef, PyDict_New,
// etc.) are provided by the running Python process rather than being linked
// into our .so at build time.
//
// On macOS (Mach-O), we must pass `-undefined dynamic_lookup` to the linker
// so it does not fail on unresolved Python symbols during the `cargo build`
// phase. The dynamic linker resolves them at runtime when Python loads the
// extension with `dlopen()`.
//
// On Linux (ELF), shared libraries may have undefined symbols by default
// (the dynamic linker resolves them at load time), so no extra flag is needed.
// We still pass --allow-shlib-undefined to be explicit.
//
// On Windows (.pyd), the extension must link against python3X.lib (the import
// library), which is found via the PYTHON_LIB_PATH environment variable.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // Tell the macOS linker to defer resolution of all undefined symbols
            // to load time. This is the standard approach for Python .so extensions.
            println!("cargo:rustc-link-arg=-undefined");
            println!("cargo:rustc-link-arg=dynamic_lookup");
        }
        "linux" => {
            // ELF .so files allow undefined symbols by default, but we can be
            // explicit about it to suppress any linker warnings.
            println!("cargo:rustc-link-arg=-Wl,--allow-shlib-undefined");
        }
        "windows" => {
            // On Windows we need to link against the Python import library
            // (`python3.lib`, the stable-ABI one, lives in <base_prefix>\libs).
            // PYTHON_LIB_PATH wins if set; otherwise ask the interpreter that
            // will load the extension, exactly as the sibling *-native crates
            // do. Without a search path rust-lld fails with
            // "could not open 'python3.lib'".
            let lib_dir = std::env::var("PYTHON_LIB_PATH").ok().or_else(|| {
                let python = std::env::var("PYO3_PYTHON")
                    .or_else(|_| std::env::var("PYTHON_SYS_EXECUTABLE"))
                    .unwrap_or_else(|_| "python".to_string());
                let output = std::process::Command::new(python)
                    .args(["-c", "import sys, os; print(os.path.join(sys.base_prefix, 'libs'))"])
                    .output()
                    .ok()?;
                let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if output.status.success() && !dir.is_empty() { Some(dir) } else { None }
            });
            if let Some(lib_path) = lib_dir {
                println!("cargo:rustc-link-search={lib_path}");
            }
            // Link against python3 (the version-agnostic import lib for stable ABI).
            println!("cargo:rustc-link-lib=python3");
        }
        _ => {}
    }
}
