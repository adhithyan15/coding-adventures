# Changelog — polynomial_native (Lua)

## [Unreleased]

### Changed

- `BUILD_windows` now builds the extension on Windows (`cargo build --release`, like the Unix `BUILD`; the package has no automated Lua tests) instead of skipping. `build.rs` now links only `lua54` and finds `lua54.lib` next to the `lua.exe` on PATH (`<bin>\..\lib`, as `scripts/setup_lua.py` lays it out, or `LUA_LIB_PATH`), like `lua/conduit`; before it searched only `lua.exe`'s own directory and emitted three library names, two of which do not exist. 

## [0.1.0] — 2026-04-03

### Added

- Initial release: Lua C extension wrapping the Rust `polynomial` crate via
  `lua-bridge`.
- Module entry point: `luaopen_polynomial_native` (loaded via `require`).
- Exports: `normalize`, `degree`, `zero`, `one`, `add`, `subtract`,
  `multiply`, `divmod`, `divide`, `modulo`, `evaluate`, `gcd`.
- Polynomials represented as Lua tables of numbers (1-indexed).
- `divmod` returns two Lua values (quotient, remainder).
- Division-by-zero caught with `catch_unwind` → Lua `error()`.
