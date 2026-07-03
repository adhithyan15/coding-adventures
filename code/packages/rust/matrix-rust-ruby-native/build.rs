// =============================================================================
// build.rs — platform-specific linking for a Ruby native extension
// =============================================================================
//
// Building a Ruby native extension on Unix vs. Windows is a story of two very
// different linkers.
//
//   * On macOS:    we want Ruby's symbols (rb_define_module, ...) to remain
//                  *unresolved* in the .dylib.  The Ruby interpreter loads
//                  the .dylib at runtime via dlopen() and resolves those
//                  symbols against its own process image — i.e. against
//                  whichever libruby (or statically-linked ruby executable)
//                  is in charge.  The magic incantation is the linker pair
//                  `-undefined dynamic_lookup`.
//
//   * On Linux:    the dynamic loader auto-resolves undefined symbols at
//                  dlopen() time, so we need no extra link args.
//
//   * On Windows:  the .dll *must* resolve every symbol at link time —
//                  there is no equivalent of dynamic_lookup.  So we ask
//                  rbconfig where Ruby's import library lives and link
//                  against it (MSVC) or against `libruby.dll.a` (MinGW).
//                  This is the same dance conduit_native does; we copy it
//                  verbatim because Windows Ruby linking is fiddly and
//                  one working incantation per workspace is plenty.
//
// We don't need to do anything for cargo dependency tracking — c-bridge
// and ruby-bridge are normal Rust deps, picked up via Cargo.toml.

use std::path::PathBuf;

fn main() {
    match std::env::var("CARGO_CFG_TARGET_OS")
        .unwrap_or_default()
        .as_str()
    {
        "macos" => {
            // -undefined dynamic_lookup: tolerate Ruby symbols being unresolved
            // at .dylib link time; the host ruby process will provide them.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => link_ruby_on_windows(),
        _ => {
            // Linux + BSDs: nothing to do.  Dynamic loader will handle it.
        }
    }
}

// -----------------------------------------------------------------------------
// Windows: discover the active Ruby's bin/lib/SO_NAME via rbconfig and link.
// -----------------------------------------------------------------------------

fn link_ruby_on_windows() {
    let Some(ruby) = find_ruby() else {
        return;
    };
    let (Some(bin_dir), Some(lib_dir), Some(so_name)) = (
        get_ruby_config(&ruby, "bindir"),
        get_ruby_config(&ruby, "libdir"),
        get_ruby_config(&ruby, "RUBY_SO_NAME"),
    ) else {
        return;
    };

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    println!("cargo:rustc-link-search=native={lib_dir}");
    println!("cargo:rustc-link-search=native={bin_dir}");

    if target_env == "msvc" {
        // MSVC needs a .lib import library.  Try to generate one from a .def;
        // if that fails (no `lib.exe` on PATH), fall back to /FORCE:UNRESOLVED.
        if generate_msvc_lib(&format!("{so_name}.dll"), &so_name, &out_dir) {
            println!("cargo:rustc-link-search=native={out_dir}");
            println!("cargo:rustc-link-lib={so_name}");
        } else {
            println!("cargo:rustc-link-lib=dylib={so_name}");
        }
        println!("cargo:rustc-cdylib-link-arg=/FORCE:UNRESOLVED");
    } else if let Some(import_library) = get_ruby_config(&ruby, "LIBRUBY") {
        // MinGW: LIBRUBY is e.g. "libruby.dll.a" — link that file directly.
        let import_library = PathBuf::from(lib_dir).join(import_library);
        println!("cargo:rustc-cdylib-link-arg={}", import_library.display());
    } else {
        println!("cargo:rustc-link-lib=dylib={so_name}");
    }
}

/// Generate a minimal MSVC `.lib` from a `.def` for the Ruby symbols we
/// actually call.  Matches the conduit_native symbol list exactly — keeping
/// these in sync means a developer adding a new Ruby C API call has one
/// place to think about, not two.
fn generate_msvc_lib(dll_name: &str, so_name: &str, out_dir: &str) -> bool {
    // (symbol, is_data_export).  Functions are exports of code; constants like
    // rb_cObject are data exports and need the DATA keyword in the .def.
    let symbols: &[(&str, bool)] = &[
        ("rb_define_module", false),
        ("rb_define_module_under", false),
        ("rb_define_class_under", false),
        ("rb_define_method", false),
        ("rb_define_singleton_method", false),
        ("rb_define_module_function", false),
        ("rb_define_alloc_func", false),
        ("rb_path2class", false),
        ("rb_utf8_str_new", false),
        ("rb_string_value_cstr", false),
        ("rb_ary_new", false),
        ("rb_ary_push", false),
        ("rb_ary_entry", false),
        ("rb_intern", false),
        ("rb_funcallv", false),
        ("rb_num2long", false),
        ("rb_int2inum", false),
        ("rb_float_new", false),
        ("rb_num2dbl", false),
        ("rb_hash_new", false),
        ("rb_hash_aset", false),
        ("rb_data_object_wrap", false),
        ("rb_check_typeddata", false),
        ("rb_raise", false),
        ("rb_str_strlen", false),
        ("rb_thread_call_without_gvl", false),
        ("rb_thread_call_with_gvl", false),
        ("rb_protect", false),
        ("rb_errinfo", false),
        ("rb_set_errinfo", false),
        ("rb_cObject", true),
        ("rb_eStandardError", true),
        ("rb_eArgError", true),
        ("rb_eRuntimeError", true),
    ];

    let def_path = PathBuf::from(out_dir).join(format!("{so_name}.def"));
    let mut def_content = format!("LIBRARY {dll_name}\nEXPORTS\n");
    for (sym, is_data) in symbols {
        if *is_data {
            def_content.push_str(&format!("    {sym} DATA\n"));
        } else {
            def_content.push_str(&format!("    {sym}\n"));
        }
    }
    if std::fs::write(&def_path, def_content).is_err() {
        return false;
    }

    let lib_path = PathBuf::from(out_dir).join(format!("{so_name}.lib"));
    for program in ["lib.exe", "lib"] {
        let result = std::process::Command::new(program)
            .args([
                &format!("/def:{}", def_path.display()),
                &format!("/out:{}", lib_path.display()),
                "/machine:x64",
                "/nologo",
            ])
            .output();
        if matches!(result, Ok(output) if output.status.success() && lib_path.exists()) {
            return true;
        }
    }

    false
}

fn get_ruby_config(ruby: &str, key: &str) -> Option<String> {
    let script = format!("require 'rbconfig'; print RbConfig::CONFIG['{key}']");
    let output = std::process::Command::new(ruby)
        .args(["-e", &script])
        .output()
        .ok()?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn find_ruby() -> Option<String> {
    for finder in ["where", "which"] {
        if let Ok(output) = std::process::Command::new(finder).arg("ruby").output() {
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
    }
    None
}
