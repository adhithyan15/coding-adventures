use std::path::PathBuf;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            if let Some(ruby) = find_ruby() {
                if let (Some(bin_dir), Some(lib_dir), Some(so_name)) = (
                    get_ruby_config(&ruby, "bindir"),
                    get_ruby_config(&ruby, "libdir"),
                    get_ruby_config(&ruby, "RUBY_SO_NAME"),
                ) {
                    let out_dir = std::env::var("OUT_DIR").unwrap();
                    let dll_name = format!("{}.dll", so_name);

                    println!("cargo:rustc-link-search=native={}", lib_dir);
                    println!("cargo:rustc-link-search=native={}", bin_dir);

                    if generate_msvc_lib(&dll_name, &so_name, &out_dir) {
                        println!("cargo:rustc-link-search=native={}", out_dir);
                        println!("cargo:rustc-link-lib={}", so_name);
                    } else {
                        println!("cargo:rustc-link-lib=dylib={}", so_name);
                    }

                    println!("cargo:rustc-cdylib-link-arg=/FORCE:UNRESOLVED");
                }
            }
        }
        _ => {}
    }
}

fn generate_msvc_lib(dll_name: &str, so_name: &str, out_dir: &str) -> bool {
    let symbols: &[(&str, bool)] = &[
        ("rb_define_module", false),
        ("rb_define_module_under", false),
        ("rb_define_class_under", false),
        ("rb_define_method", false),
        ("rb_define_singleton_method", false),
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
        ("rb_data_object_wrap", false),
        ("rb_check_typeddata", false),
        ("rb_raise", false),
        ("rb_str_strlen", false),
        ("rb_block_given_p", false),
        ("rb_yield", false),
        ("rb_cObject", true),
        ("rb_eStandardError", true),
        ("rb_eArgError", true),
        ("rb_eRuntimeError", true),
    ];

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

    for command in ["lib.exe", "lib"] {
        if let Ok(output) = std::process::Command::new(command)
            .args([
                &format!("/def:{}", def_path.display()),
                &format!("/out:{}", lib_path.display()),
                "/machine:x64",
                "/nologo",
            ])
            .output()
        {
            if output.status.success() && lib_path.exists() {
                return true;
            }
        }
    }

    false
}

fn get_ruby_config(ruby: &str, key: &str) -> Option<String> {
    let script = format!("require 'rbconfig'; print RbConfig::CONFIG['{}']", key);
    let output = std::process::Command::new(ruby)
        .args(["-e", &script])
        .output()
        .ok()?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn find_ruby() -> Option<String> {
    for name in ["ruby"] {
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
