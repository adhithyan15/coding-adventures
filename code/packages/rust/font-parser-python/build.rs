// build.rs -- link the Python import library on Windows
//
// font-parser-python is a Python extension module (a cdylib that Python loads
// at import time).  The C API symbols it calls (PyModule_Create2, PyErr_*,
// Py_BuildValue, ...) are provided by the interpreter, not by a library we
// link against, so each platform needs a different answer at link time:
//
// - macOS: `-undefined dynamic_lookup` -- resolve symbols at load time.
// - Linux: nothing -- ELF shared objects may leave symbols undefined.
// - Windows: link against `pythonXY.lib`, the import library that ships with
//   every CPython install (`<base_prefix>\libs`).  A PE/COFF DLL cannot leave
//   an import unresolved.
//
// Same logic as the python/*-native packages, which replaced PyO3 with our own
// python-bridge and therefore do this by hand (it is what PyO3's build script
// does internally).

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-env-changed=PYTHON_SYS_EXECUTABLE");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // The macOS linker refuses undefined symbols unless told otherwise;
            // they are resolved when Python loads our .so/.dylib.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => link_python_import_library(),
        _ => {
            // Linux and other ELF targets: nothing to do.
        }
    }
}

/// Ask a real interpreter where its import library lives and link it.
///
/// An explicit `PYO3_PYTHON` / `PYTHON_SYS_EXECUTABLE` wins; otherwise `python`
/// (what actions/setup-python puts on PATH) and then `python3`/`py`.  If none
/// answers we say so, rather than leaving the linker to report a wall of
/// `unresolved external symbol Py*` errors with no hint why.
fn link_python_import_library() {
    // sys.base_prefix, not sys.prefix: inside a venv `libs` lives in the base.
    let script = "import sys, os; v = sys.version_info; \
                  print(os.path.join(sys.base_prefix, 'libs') + '|python%d%d' % (v.major, v.minor))";

    let mut candidates: Vec<String> = Vec::new();
    for var in ["PYO3_PYTHON", "PYTHON_SYS_EXECUTABLE"] {
        if let Ok(path) = std::env::var(var) {
            candidates.push(path);
        }
    }
    candidates.extend(["python", "python3", "py"].iter().map(|s| s.to_string()));

    for python in &candidates {
        let Ok(output) = Command::new(python).args(["-c", script]).output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Some((lib_dir, lib_name)) = stdout.split_once('|') {
            println!("cargo:warning=font-parser-python: linking {lib_name} from {lib_dir} (via {python})");
            println!("cargo:rustc-link-search=native={lib_dir}");
            println!("cargo:rustc-link-lib={lib_name}");
            return;
        }
    }

    // Not a panic: `cargo check` and `cargo clippy` never link, and must keep
    // working on a machine with no Python.  Only an actual link needs the
    // library, and it will fail right after this warning.
    println!(
        "cargo:warning=font-parser-python: could not find a Python interpreter to locate pythonXY.lib; \
         tried {candidates:?}. Put python on PATH or set PYO3_PYTHON; linking will fail without it."
    );
}
