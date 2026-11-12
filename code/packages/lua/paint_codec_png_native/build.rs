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
            // A PE/COFF DLL must resolve every import at link time, so link
            // against the Lua 5.4 import library `lua54.lib`.  Search order:
            // LUA_LIB_PATH, then the directory holding `lua54.lib` next to the
            // `lua.exe` on PATH (CI installs Lua with scripts/setup_lua.py, which
            // puts it in <prefix>\lib beside <prefix>\bin\lua.exe).  Only the
            // `lua54` name exists; the old `lua5.4` and `lua-5.4` names do not,
            // and rustc fails on a missing `-l` library.
            if let Ok(lib_path) = std::env::var("LUA_LIB_PATH") {
                println!("cargo:rustc-link-search=native={lib_path}");
            } else if let Some(dir) = lua_lib_dir_from_path() {
                println!("cargo:rustc-link-search=native={}", dir.display());
            }
            println!("cargo:rustc-link-lib=lua54");
        }
        _ => {
            // Linux and other Unix: ELF allows undefined symbols in shared
            // objects by default. No flags needed.
        }
    }
}

/// Find the directory holding `lua54.lib` next to the `lua.exe` on PATH:
/// `<bin>/../lib` (the layout `setup_lua.py` and LuaRocks use) or `<bin>` itself.
fn lua_lib_dir_from_path() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if !dir.join("lua.exe").is_file() {
            continue;
        }
        let mut candidates = vec![dir.clone()];
        if let Some(parent) = dir.parent() {
            candidates.insert(0, parent.join("lib"));
        }
        for candidate in candidates {
            if candidate.join("lua54.lib").is_file() {
                return Some(candidate);
            }
        }
    }
    None
}
