// build.rs — linker flags for the Lua gf256_native C extension
//
// Lua extension modules (.so/.dylib/.dll) are loaded by the Lua interpreter
// at runtime via require(). The Lua C API symbols (lua_pushinteger,
// luaL_checkinteger, etc.) are provided by the interpreter, not linked at
// build time.
//
// - macOS: `-undefined dynamic_lookup` lets the linker defer symbol resolution
//          to load time, when the Lua interpreter process provides them.
// - Linux: ELF shared objects allow undefined symbols by default — no flags needed.
// - Windows: Requires linking against lua5.4.dll's import library. We locate
//            Lua via the PATH and emit the appropriate search/lib directives.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // Apple ld requires an explicit flag to allow undefined symbols.
            // The symbols (luaL_checkinteger, lua_pushinteger, etc.) are
            // resolved when Lua calls dlopen() on our .dylib.
            println!("cargo:rustc-cdylib-link-arg=-undefined");
            println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
        }
        "windows" => {
            // On Windows, all symbols must be resolved at link time.
            // We need to link against lua5.4.dll's import library.
            if let Some(lua_lib_dir) = find_lua_lib_dir() {
                println!("cargo:rustc-link-search=native={}", lua_lib_dir);
            }
            // Try common Lua import library names used by LuaBinaries and luarocks.
            for name in &["lua5.4", "lua54", "lua-5.4"] {
                println!("cargo:rustc-link-lib={}", name);
            }
        }
        _ => {
            // Linux and other Unix: ELF allows undefined symbols in shared
            // objects by default. No flags needed.
        }
    }
}

/// Try to locate the Lua library directory on Windows by asking lua.exe for
/// its LUA_LIBDIR configuration.
fn find_lua_lib_dir() -> Option<String> {
    // On GitHub Actions Windows runners, Lua is installed in the repo's
    // .lua/ directory. Try finding lua.exe in PATH.
    let lua_exe = find_exe("lua")?;
    let parent = std::path::Path::new(&lua_exe).parent()?;
    // The DLL is typically next to the executable.
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
