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
            // Try LUA_LIB_PATH first, then fall back to common mise paths.
            if let Ok(lib_path) = std::env::var("LUA_LIB_PATH") {
                println!("cargo:rustc-link-search={lib_path}");
            } else {
                // leafo/gh-actions-lua on Windows puts Lua in %USERPROFILE%\AppData\Roaming
                // or the GitHub Actions tool cache. Try a few common locations.
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
