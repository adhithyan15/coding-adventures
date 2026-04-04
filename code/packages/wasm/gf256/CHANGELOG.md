# Changelog — gf256-wasm

All notable changes to this package are documented here.

---

## [0.1.0] — 2026-04-03

### Added

- Initial release: WebAssembly build of the `gf256` crate with pure C ABI exports.
- Exported functions: `gf256_add`, `gf256_subtract`, `gf256_multiply`,
  `gf256_divide`, `gf256_power`, `gf256_inverse`, `gf256_had_error`,
  `gf256_zero`, `gf256_one`, `gf256_primitive_polynomial`.
- All values passed as `u32` (WASM has no native u8 type; the module uses only
  the low 8 bits of each argument).
- `catch_unwind` error handling: `gf256_divide(a, 0)` and `gf256_inverse(0)`
  return `0xFF` and set the error flag instead of trapping the WASM module.
- Release profile: `opt-level = "z"`, `lto = true`, `strip = true`,
  `panic = "abort"` for minimal `.wasm` binary size.
- Zero external dependencies: no wasm-bindgen, no wasm-pack, no JavaScript glue.
- No memory protocol needed — all GF(256) operations are purely scalar.
