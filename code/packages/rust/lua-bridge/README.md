# lua-bridge

A thin Rust wrapper over the Lua 5.4 C API with **zero dependencies**: no `mlua`,
no `rlua`, no `bindgen`, no build-time header requirement. It declares the Lua
functions it needs as `extern "C"` and leaves the linker (or, on Linux and macOS,
the running `lua`) to resolve them.

It is the base of the repository's Rust-backed Lua extensions
(`code/packages/lua/board_vm_native` today). Each is a `cdylib` that a Lua program
loads with `require`; Lua finds its `luaopen_<name>` function and calls it.

## What is in it

- The core types (`lua_State`, `lua_CFunction`, `luaL_Reg`) and the constants
  (`LUA_REGISTRYINDEX`, the `LUA_T*` type tags).
- The functions Lua really exports, as `extern "C"` declarations.
- The C *macros* of the Lua headers (`lua_pop`, `lua_insert`, `lua_remove`,
  `lua_newtable`, `lua_pushcfunction`, `luaL_checkstring`, `luaL_getmetatable`,
  ...) as inline Rust functions. A macro has no symbol, so declaring one `extern`
  compiles and then fails to load with `undefined symbol`.
- Helpers: `push_str`, `get_str` (byte-exact, embedded NULs included),
  `get_f64`, `push_f64_table`, `get_f64_table`, `raise_error`, `register_lib`.

```rust,ignore
use lua_bridge::*;

unsafe extern "C" fn add(L: *mut lua_State) -> std::ffi::c_int {
    lua_pushnumber(L, luaL_checknumber(L, 1) + luaL_checknumber(L, 2));
    1 // one result
}

#[no_mangle]
pub unsafe extern "C" fn luaopen_mymod(L: *mut lua_State) -> std::ffi::c_int {
    let funcs = [
        luaL_Reg { name: c"add".as_ptr(), func: Some(add) },
        luaL_Reg { name: std::ptr::null(), func: None },
    ];
    register_lib(L, &funcs);
    1 // the module table
}
```

The crate's `lua_module!` macro builds the `luaopen_` name with `concat_idents!`,
which only exists on nightly Rust; write the function out as above on stable.

## Linking

The library crate links nothing. A cdylib built on it needs, per OS:

| OS | What its `build.rs` must do |
|---|---|
| Linux | nothing: ELF shared objects may leave symbols undefined and `lua` exports them |
| macOS | pass `-undefined dynamic_lookup` to the linker |
| Windows | link `lua54.lib` (the import library next to `lua.exe`) |

This crate's own `build.rs` does that for its smoke module; copy it (or
`lua/board_vm_native/build.rs`) into a new extension.

## Testing

`BUILD` / `BUILD_windows` (the same on every OS) run:

1. `cargo test -p lua-bridge`: pins the constants and layouts that come from Lua's
   headers.
2. `cargo clippy --all-targets -- -D warnings`.
3. `cargo build --release --example lua_bridge_smoke`, then
   `lua tests/smoke.lua target/release/examples`: the example is a real Lua C module
   built on the crate; the script loads it into the real `lua` with
   `package.loadlib` and checks every number, string, table, stack, function,
   userdata, metatable, `__gc`, registry and error path from the Lua side.

A compile check cannot find a bridge bug: a macro declared as a function, a wrong
constant and a wrong argument order all compile. The first run of the smoke test
found five macros and one misspelt function declared as exported symbols. The
recipe needs a `lua` 5.4 on `PATH` (`# needs-toolchain: lua`).
