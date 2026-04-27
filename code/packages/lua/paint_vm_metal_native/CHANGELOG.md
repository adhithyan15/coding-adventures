# Changelog — gf256_native (Lua)

## [0.1.0] — 2026-04-03

### Added

- Initial release: Lua C extension wrapping the Rust `gf256` crate via
  `lua-bridge`.
- Module entry point: `luaopen_gf256_native`.
- Exports: `add`, `subtract`, `multiply`, `divide`, `power`, `inverse`.
- Elements passed as Lua integers (0–255).
- Division-by-zero and inverse-of-zero guarded with `catch_unwind` → Lua error.
