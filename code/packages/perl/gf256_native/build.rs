// build.rs — linker flags for the Perl GF256Native XS extension
//
// Perl extension modules (.so/.bundle/.dll) are loaded by the Perl DynaLoader
// at runtime via `use Module`. The Perl C API symbols (Perl_sv_setiv,
// PL_stack_sp, etc.) are provided by the interpreter process.
//
// - macOS: `-undefined dynamic_lookup` — resolve symbols at load time
// - Linux: ELF shared objects allow undefined symbols by default
// - Windows: link against the Perl DLL import library
//
// Threaded-Perl-sensitive stack access now lives in `perl-bridge`, which
// compiles a small shim against the host Perl headers.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // Apple ld requires explicit flag to allow undefined symbols.
            // The Perl C API symbols are resolved when Perl's DynaLoader
            // calls dlopen() on our .bundle/.dylib.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            // On Windows, find the Perl DLL and link against it.
            if let Some((lib_dir, lib_name)) = find_perl_lib() {
                println!("cargo:rustc-link-search=native={}", lib_dir);
                println!("cargo:rustc-link-lib={}", lib_name);
            }
        }
        _ => {
            // Linux/Unix: ELF allows undefined symbols by default.
        }
    }
}

/// Locate the Perl DLL import library on Windows (Strawberry Perl).
fn find_perl_lib() -> Option<(String, String)> {
    let perl = find_exe("perl")?;
    let parent = std::path::Path::new(&perl).parent()?;
    let lib_dir = parent.parent()?.join("lib");

    for entry in std::fs::read_dir(&lib_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if (name_str.starts_with("perl") && name_str.ends_with(".lib"))
            || (name_str.starts_with("perl") && name_str.ends_with(".dll.a"))
        {
            let lib_name = name_str
                .trim_end_matches(".lib")
                .trim_end_matches(".dll.a")
                .to_string();
            return Some((lib_dir.to_string_lossy().to_string(), lib_name));
        }
    }
    None
}

fn find_exe(name: &str) -> Option<String> {
    let output = std::process::Command::new("where")
        .arg(name)
        .output()
        .ok()?;
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
    None
}
