# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now builds the extension and runs the busted suite on Windows instead of skipping. `build.rs` now links only `lua54` and finds `lua54.lib` next to the `lua.exe` on PATH (`<bin>\..\lib`, as `scripts/setup_lua.py` lays it out, or `LUA_LIB_PATH`), like `lua/conduit`; before it searched only `lua.exe`'s own directory and emitted three library names, two of which do not exist. 

## 0.1.0

- Add Rust-backed Lua native Board VM frame/module builders.
- Add Lua `Session` sugar for HELLO, CAPS_QUERY, and blink upload/run frames.
- Expose Rust-owned target metadata, connection options, upload options, and
  host device classification through Lua tables.
