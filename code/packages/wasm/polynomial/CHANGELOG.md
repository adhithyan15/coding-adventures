# Changelog — polynomial-wasm

All notable changes to this package are documented here.

---

## [0.1.0] — 2026-04-03

### Added

- Initial release: WebAssembly build of the `polynomial` crate with pure C ABI exports.
- Exported functions: `poly_alloc`, `poly_dealloc`, `poly_last_result_len`,
  `poly_had_error`, `poly_normalize`, `poly_degree`, `poly_add`, `poly_subtract`,
  `poly_multiply`, `poly_divmod`, `poly_divmod_quotient_ptr`,
  `poly_divmod_quotient_len`, `poly_divmod_remainder_ptr`,
  `poly_divmod_remainder_len`, `poly_divide`, `poly_modulo`, `poly_evaluate`,
  `poly_gcd`.
- Linear memory allocator protocol (`poly_alloc`/`poly_dealloc`) for passing
  variable-length arrays between host and module.
- `poly_divmod` caches quotient and remainder in module-level statics, accessible
  via four separate accessor functions.
- `catch_unwind` error handling: panics (e.g., division by zero) set the error
  flag instead of trapping the WASM module.
- Release profile: `opt-level = "z"`, `lto = true`, `strip = true`,
  `panic = "abort"` for minimal `.wasm` binary size.
- Zero external dependencies: no wasm-bindgen, no wasm-pack, no JavaScript glue.
