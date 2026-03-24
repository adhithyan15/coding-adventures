// build.rs -- Tell the linker to allow undefined Ruby C API symbols
// =================================================================
//
// Ruby extension modules (.so/.bundle/.dll) are loaded into a running
// Ruby interpreter at runtime. The C API symbols (rb_define_method,
// rb_data_object_wrap, etc.) are provided by the interpreter, not by
// a library we link against at compile time. We need to tell the linker:
//
// - macOS: `-undefined dynamic_lookup` -- resolve symbols at load time
// - Linux: nothing needed -- ELF shared objects allow undefined symbols by default
// - Windows: link against the Ruby DLL import library -- Windows requires
//   explicit linking. MinGW-built Ruby (RubyInstaller) provides a `.dll.a`
//   import library which MSVC's linker partially understands (functions work,
//   but static data symbols like rb_cObject don't). To fix this, we generate
//   a proper MSVC `.lib` from a handwritten .def file containing all the
//   Ruby C API symbols we use.

use std::path::PathBuf;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // macOS linker requires explicit flag to allow undefined symbols.
            // These symbols will be resolved when Ruby loads our .bundle.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            // On Windows, we need to link against the Ruby import library.
            //
            // RubyInstaller (MinGW-built) provides a `.dll.a` import library
            // that MSVC's link.exe only partially understands -- function symbols
            // work but static data symbols (rb_cObject, rb_eStandardError, etc.)
            // don't resolve. We fix this by:
            //
            // 1. Writing a .def file listing all Ruby C API symbols we need
            // 2. Using MSVC's `lib /def:` to create a proper .lib import library
            // 3. Linking against both the .dll.a (for most symbols) and our .lib
            //    (for the missing data symbols)
            if let Some(ruby) = find_ruby() {
                if let (Some(bin_dir), Some(lib_dir), Some(so_name)) = (
                    get_ruby_config(&ruby, "bindir"),
                    get_ruby_config(&ruby, "libdir"),
                    get_ruby_config(&ruby, "RUBY_SO_NAME"),
                ) {
                    let out_dir = std::env::var("OUT_DIR").unwrap();
                    let dll_name = format!("{}.dll", so_name);

                    // Add search paths
                    println!("cargo:rustc-link-search=native={}", lib_dir);
                    println!("cargo:rustc-link-search=native={}", bin_dir);

                    // Generate a proper .lib from a .def file using lib.exe
                    // for function symbols that MSVC can resolve. For remaining
                    // data symbols (rb_cObject, etc.) that the linker still can't
                    // resolve from the .def, we use /FORCE:UNRESOLVED to allow
                    // linking to succeed -- these symbols will be resolved at
                    // load time when Ruby loads our DLL.
                    if generate_msvc_lib(&dll_name, &so_name, &out_dir) {
                        println!("cargo:rustc-link-search=native={}", out_dir);
                        println!("cargo:rustc-link-lib={}", so_name);
                    } else {
                        // Fallback: use the .dll.a as-is
                        println!("cargo:rustc-link-lib=dylib={}", so_name);
                    }

                    // The ruby-bridge rlib contains pub functions that reference
                    // extern statics (rb_eArgError, rb_eRuntimeError). Even though
                    // our code never calls those functions, the linker still tries
                    // to resolve them. /FORCE:UNRESOLVED allows the link to succeed.
                    // These statics are never accessed at runtime because we use
                    // rb_path2class() instead.
                    println!("cargo:rustc-cdylib-link-arg=/FORCE:UNRESOLVED");
                }
            }
        }
        _ => {
            // Linux and other Unix: ELF allows undefined symbols in shared
            // objects by default. No special linker flags needed.
        }
    }
}

/// Generate a proper MSVC .lib import library from a handwritten .def file.
///
/// We know exactly which Ruby C API symbols we use (and which the ruby-bridge
/// crate uses). Rather than trying to parse the DLL's export table, we simply
/// list them all in a .def file and use MSVC's `lib.exe /def:` to create
/// a .lib.
///
/// `lib.exe` should be in the same directory as `link.exe` that Rust/Cargo
/// already found. We locate it via the `PATH` set up by cargo's MSVC detection.
fn generate_msvc_lib(dll_name: &str, so_name: &str, out_dir: &str) -> bool {
    // All Ruby C API symbols used by ruby-bridge and our extension.
    // This list must be kept in sync with the extern "C" declarations
    // in ruby-bridge/src/lib.rs and our own lib.rs.
    //
    // Symbols are (name, is_data). Data symbols (extern statics like
    // rb_cObject) need the DATA keyword in the .def file so that MSVC's
    // linker generates proper __imp_ prefixed imports for them.
    let symbols: &[(&str, bool)] = &[
        // Functions from ruby-bridge extern "C" block:
        ("rb_define_module", false),
        ("rb_define_module_under", false),
        ("rb_define_class_under", false),
        ("rb_define_method", false),
        ("rb_define_singleton_method", false),
        ("rb_define_alloc_func", false),
        ("rb_utf8_str_new", false),
        ("rb_string_value_cstr", false),
        ("rb_ary_new", false),
        ("rb_ary_push", false),
        ("rb_ary_entry", false),
        ("rb_array_len", false),
        ("rb_int2inum", false),
        ("rb_data_object_wrap", false),
        ("rb_check_typeddata", false),
        ("rb_raise", false),
        ("rb_str_strlen", false),
        // Static data from ruby-bridge extern "C" block:
        ("rb_cObject", true),
        ("rb_eStandardError", true),
        ("rb_eArgError", true),
        ("rb_eRuntimeError", true),
        // Functions from our lib.rs:
        ("rb_num2long", false),
        ("rb_path2class", false),
    ];

    // Write the .def file
    //
    // The DATA keyword after a symbol name tells the linker this is a
    // data import (extern static), not a function. Without it, the linker
    // generates a function-call thunk that causes crashes or wrong values
    // when used to access data.
    let def_path = PathBuf::from(out_dir).join(format!("{}.def", so_name));
    let mut def_content = format!("LIBRARY {}\nEXPORTS\n", dll_name);
    for (sym, is_data) in symbols {
        if *is_data {
            def_content.push_str(&format!("    {} DATA\n", sym));
        } else {
            def_content.push_str(&format!("    {}\n", sym));
        }
    }
    if std::fs::write(&def_path, &def_content).is_err() {
        return false;
    }

    let lib_path = PathBuf::from(out_dir).join(format!("{}.lib", so_name));

    // Try lib.exe (should be on PATH in cargo's MSVC environment)
    let result = std::process::Command::new("lib.exe")
        .args([
            &format!("/def:{}", def_path.display()),
            &format!("/out:{}", lib_path.display()),
            "/machine:x64",
            "/nologo",
        ])
        .output();

    if let Ok(output) = result {
        if output.status.success() && lib_path.exists() {
            return true;
        }
    }

    // Try bare `lib` as well
    let result = std::process::Command::new("lib")
        .args([
            &format!("/def:{}", def_path.display()),
            &format!("/out:{}", lib_path.display()),
            "/machine:x64",
            "/nologo",
        ])
        .output();

    if let Ok(output) = result {
        if output.status.success() && lib_path.exists() {
            return true;
        }
    }

    // Try to find lib.exe next to link.exe (which cargo found)
    // by searching common Visual Studio paths
    let vs_paths = [
        "C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC",
        "C:/Program Files/Microsoft Visual Studio/2022/Professional/VC/Tools/MSVC",
        "C:/Program Files/Microsoft Visual Studio/2022/Enterprise/VC/Tools/MSVC",
        "C:/Program Files (x86)/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC",
        "C:/Program Files (x86)/Microsoft Visual Studio/2019/Community/VC/Tools/MSVC",
    ];

    for vs_base in &vs_paths {
        let base = PathBuf::from(vs_base);
        if !base.exists() {
            continue;
        }
        // Look for versioned subdirectories
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let lib_exe = entry.path().join("bin/HostX64/x64/lib.exe");
                if lib_exe.exists() {
                    let result = std::process::Command::new(&lib_exe)
                        .args([
                            &format!("/def:{}", def_path.display()),
                            &format!("/out:{}", lib_path.display()),
                            "/machine:x64",
                            "/nologo",
                        ])
                        .output();

                    if let Ok(output) = result {
                        if output.status.success() && lib_path.exists() {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Get a Ruby RbConfig value by key.
fn get_ruby_config(ruby: &str, key: &str) -> Option<String> {
    let script = format!("print RbConfig::CONFIG['{}']", key);
    let output = std::process::Command::new(ruby)
        .args(["-e", &script])
        .output()
        .ok()?;
    let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if val.is_empty() { None } else { Some(val) }
}

/// Find the Ruby executable in PATH.
fn find_ruby() -> Option<String> {
    for name in &["ruby"] {
        // Try 'where' on Windows
        if let Ok(output) = std::process::Command::new("where").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
        // Try 'which' on Unix
        if let Ok(output) = std::process::Command::new("which").arg(name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
    }
    None
}
