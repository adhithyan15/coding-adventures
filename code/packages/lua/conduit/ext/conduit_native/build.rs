// build.rs — linker configuration for the Lua C extension cdylib.
//
// Lua C extensions (.so / .dll) are loaded via dlopen() by the Lua interpreter
// at `require()` time. The Lua C API symbols (lua_gettop, luaL_checkudata, etc.)
// are provided by the running Lua process rather than being linked into our .so
// at build time.
//
// On macOS (Mach-O), we must pass `-undefined dynamic_lookup` to the linker
// so it does not fail on unresolved Lua symbols during the `cargo build` phase.
// The dynamic linker resolves them at runtime when Lua loads the extension.
//
// On Linux (ELF), shared libraries may have undefined symbols by default, so no
// extra flag is needed. We pass --allow-shlib-undefined to be explicit.
//
// On Windows (.dll), the extension must link against lua54.lib (the Lua import
// library). We search for it via LUA_LIB_PATH or common mise/leafo paths.

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // Tell the macOS linker to defer resolution of all undefined symbols
            // to load time. Standard approach for Lua .so extensions.
            println!("cargo:rustc-link-arg=-undefined");
            println!("cargo:rustc-link-arg=dynamic_lookup");
        }
        "linux" => {
            // ELF .so files allow undefined symbols by default.
            println!("cargo:rustc-link-arg=-Wl,--allow-shlib-undefined");
        }
        "windows" => {
            // On Windows we need to link against the Lua 5.4 import library.
            // Order: LUA_LIB_PATH, then the lib directory of the `lua.exe` on
            // PATH (CI installs Lua with scripts/setup_lua.py, which puts
            // lua54.lib in <prefix>\lib next to <prefix>\bin\lua.exe), then a
            // few fixed locations. Without a search path rust-lld fails with
            // "could not open 'lua54.lib'".
            if let Ok(lib_path) = std::env::var("LUA_LIB_PATH") {
                println!("cargo:rustc-link-search={lib_path}");
            } else if let Some(dir) = lua_lib_dir_from_path() {
                println!("cargo:rustc-link-search={}", dir.display());
            } else {
                for candidate in &[
                    "C:\\tools\\lua54",
                    "C:\\Lua\\5.4",
                ] {
                    if std::path::Path::new(candidate).exists() {
                        println!("cargo:rustc-link-search={candidate}");
                        break;
                    }
                }
            }
            // Link against lua54 (Lua 5.4 import library).
            println!("cargo:rustc-link-lib=lua54");
        }
        _ => {}
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
