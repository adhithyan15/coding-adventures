# Changelog

All notable changes to the lua-bridge crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Six declarations named symbols that Lua 5.4 does not export, so a module that
  used any of them failed to load with `undefined symbol`. Nothing had ever loaded
  a module built on this crate, because the recipe was an `echo`. Found by the new
  smoke test, and confirmed against `nm -D liblua5.4.so`:
  - `lua_insert`, `lua_remove`, `lua_isboolean`, `luaL_getmetatable` and
    `luaL_checkstring` are C macros in Lua 5.4. They are now inline Rust functions
    with the macros' definitions (`lua_insert`/`lua_remove` over the new `lua_rotate`
    declaration, `luaL_checkstring` over the new `luaL_checklstring`). Same names and
    signatures, so callers do not change.
  - `lua_newmetatable` does not exist; the function is `luaL_newmetatable`. The new
    `luaL_newmetatable` is the real declaration and `lua_newmetatable` stays as an
    inline alias for it.

### Added

- `build.rs`: `-undefined dynamic_lookup` on macOS and `lua54.lib` on Windows for
  the smoke module (the same logic as `lua/board_vm_native`); nothing changes for
  Linux.
- `examples/lua_bridge_smoke.rs` and `tests/smoke.lua`: a real Lua C module built on
  the crate and loaded into the real `lua` by `package.loadlib`. It calls the number,
  string (embedded NULs included), table, stack, function, userdata, metatable,
  `__gc`, registry (`LUA_REGISTRYINDEX`) and error (`raise_error`, variadic
  `luaL_error`, `luaL_argerror`) paths and checks each result from Lua.
- Unit tests for the constants and layouts that come from Lua's headers rather than
  from a symbol (`LUA_REGISTRYINDEX`, the type tags, `luaL_Reg`).
- A README and this changelog (the crate had neither).

### Changed — `BUILD`/`BUILD_windows` build, lint and test instead of echoing

Both recipes were `echo "lua-bridge: compile-only crate (requires Lua 5.4 shared
library to link)"`, so CI never compiled the crate. They now run the unit tests,
`cargo build --release`, `cargo clippy --all-targets -D warnings` and the smoke test
on every OS, with `# needs-toolchain: lua` so CI installs the pinned Lua 5.4.7. The
crate left the W6 Windows echo-skip allowlist (BUILD-002).

## [0.1.0] - Unreleased

### Added

- Raw `extern "C"` declarations for the Lua 5.4 C API, the macros that have no
  symbol as inline functions, and the `push_str`, `get_str`, `get_f64`,
  `push_f64_table`, `get_f64_table`, `raise_error` and `register_lib` helpers.
