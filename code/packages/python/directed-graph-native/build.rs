// build.rs — Tell the linker to allow undefined symbols
//
// Python extension modules (.so/.dylib/.pyd) are loaded into a running
// Python interpreter at runtime. The C API symbols (PyModule_Create,
// PyList_New, etc.) are provided by the interpreter, not by a library
// we link against. We need to tell the linker:
//
// - macOS: `-undefined dynamic_lookup` — resolve symbols at load time
// - Linux: nothing needed — ELF shared objects allow undefined symbols by default
// - Windows: link against `pythonXY.lib` — Windows requires explicit linking
//
// This is what PyO3's build script does internally. Since we replaced PyO3
// with our own python-bridge, we handle it ourselves.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // macOS linker requires explicit flag to allow undefined symbols.
            // These symbols will be resolved when Python loads our .dylib.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            // On Windows, we need to link against the Python import library.
            // Try to find it via the PYO3_PYTHON or PYTHON_SYS_EXECUTABLE env vars,
            // or fall back to the python3.lib in the PATH.
            //
            // For maturin builds, maturin sets the correct python path.
            // We try to find the library directory from the Python executable.
            if let Ok(python) = std::env::var("PYO3_PYTHON")
                .or_else(|_| std::env::var("PYTHON_SYS_EXECUTABLE"))
                .or_else(|_| which_python())
            {
                if let Some(lib_dir) = get_python_lib_dir(&python) {
                    println!("cargo:rustc-link-search=native={}", lib_dir);
                }
                if let Some(lib_name) = get_python_lib_name(&python) {
                    println!("cargo:rustc-link-lib={}", lib_name);
                }
            }
        }
        _ => {
            // Linux and other Unix: ELF allows undefined symbols in shared
            // objects by default. No special linker flags needed.
        }
    }
}

fn which_python() -> Result<String, std::env::VarError> {
    // Try to find python3 or python in PATH
    for name in &["python3", "python"] {
        if let Ok(output) = std::process::Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }
        // Windows: try where instead of which
        if let Ok(output) = std::process::Command::new("where").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }
    }
    Err(std::env::VarError::NotPresent)
}

fn get_python_lib_dir(python: &str) -> Option<String> {
    // On Windows, LIBDIR from sysconfig may be empty. Fall back to
    // sys.base_prefix + "/libs" which is where pythonXY.lib lives.
    let output = std::process::Command::new(python)
        .args(["-c", "import sysconfig, sys, os; d = sysconfig.get_config_var('LIBDIR') or ''; print(d if d and os.path.isdir(d) else os.path.join(sys.base_prefix, 'libs'))"])
        .output()
        .ok()?;
    let dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if dir.is_empty() { None } else { Some(dir) }
}

fn get_python_lib_name(python: &str) -> Option<String> {
    let output = std::process::Command::new(python)
        .args(["-c", "import sys; print(f'python{sys.version_info.major}{sys.version_info.minor}')"])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() { None } else { Some(name) }
}
