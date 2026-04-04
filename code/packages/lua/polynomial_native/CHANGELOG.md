# Changelog — polynomial_native (Lua)

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
