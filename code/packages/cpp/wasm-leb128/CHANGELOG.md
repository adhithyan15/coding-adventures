# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `wasm-leb128` crate, in namespace
  `ca::leb128`: LEB128 variable-length integer coding (WebAssembly / DWARF).
- API: `encode_unsigned` / `encode_signed` returning `std::vector<std::uint8_t>`
  (never fail), and `decode_unsigned` / `decode_signed` (over a `std::vector` or
  `data, len` plus an offset) returning `std::pair<value, bytes_consumed>` and
  throwing `ca::leb128::Error` (carrying `offset`) on failure.
- Faithful decode errors: out-of-bounds offset, over-wide (70-bit) sequence, and
  unterminated sequence. The signed arithmetic shift and the u64→i64
  reinterpretation use `memcpy`/explicit bit fills for well-defined behaviour on
  every target.
- Tests use the crate's own WASM/DWARF vectors — zero, multi-byte, u32/i32 min &
  max, offset decoding, the error conditions, and encode↔decode round trips
  including `u64::MAX` / `i64::MIN` — under GCC and Clang via `iso-harness`.
