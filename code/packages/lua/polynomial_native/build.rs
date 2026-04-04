// build.rs — linker flags for the Lua polynomial_native C extension
//
// Lua extension modules (.so/.dylib/.dll) are loaded by the Lua interpreter
// at runtime via require(). The Lua C API symbols are provided by the
// interpreter, not linked at build time.
//
// - macOS: `-undefined dynamic_lookup` — resolve symbols at load time
// - Linux: nothing needed — ELF allows undefined symbols by default
// - Windows: link against lua5.4.dll's import library

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            if let Some(lua_lib_dir) = find_lua_lib_dir() {
                println!("cargo:rustc-link-search=native={}", lua_lib_dir);
            }
            for name in &["lua5.4", "lua54", "lua-5.4"] {
                println!("cargo:rustc-link-lib={}", name);
            }
        }
        _ => {}
    }
}

fn find_lua_lib_dir() -> Option<String> {
    let lua_exe = find_exe("lua")?;
    let parent = std::path::Path::new(&lua_exe).parent()?;
    Some(parent.to_string_lossy().to_string())
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
