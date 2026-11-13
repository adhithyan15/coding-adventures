// build.rs -- link flags for the lua_bridge_smoke example (a Lua C module).
//
// A Lua C module (.so / .dylib / .dll) is loaded by `require` or
// `package.loadlib` into a running Lua; the lua_* symbols it calls come from
// that process.  The library crate itself links nothing, so none of this
// affects a consumer unless its own build.rs asks for the same thing (as
// lua/board_vm_native does).
//
// - macOS: `-undefined dynamic_lookup`, because Apple ld rejects undefined
//   symbols in a dylib by default.
// - Linux: ELF shared objects may leave symbols undefined; Lua's own
//   executable exports them (it is linked with -Wl,-E).
// - Windows: a PE/COFF DLL must resolve every import at link time, so link
//   `lua54.lib`.  Search order: LUA_LIB_PATH, then `<bin>/../lib` or `<bin>`
//   beside the `lua.exe` on PATH (CI installs Lua with scripts/setup_lua.py,
//   which puts the import library in <prefix>\lib beside <prefix>\bin\lua.exe).
//   Only the `lua54` name exists; rustc fails on a missing `-l` library.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LUA_LIB_PATH");
    println!("cargo:rerun-if-env-changed=PATH");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            // `rustc-link-arg` (not `rustc-cdylib-link-arg`) so the unit-test
            // binary links too, should it ever keep a reference to lua_*.
            println!("cargo:rustc-link-arg=-undefined");
            println!("cargo:rustc-link-arg=dynamic_lookup");
        }
        "windows" => {
            if let Ok(lib_path) = std::env::var("LUA_LIB_PATH") {
                println!("cargo:rustc-link-search=native={lib_path}");
            } else if let Some(dir) = lua_lib_dir_from_path() {
                println!("cargo:rustc-link-search=native={}", dir.display());
            }
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
