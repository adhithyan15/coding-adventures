# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `wasm-leb128` crate: LEB128 variable-length
  integer coding (WebAssembly / DWARF), unsigned and signed.
- API: `leb128_encode_unsigned` / `leb128_encode_signed` (write into a caller
  buffer of `LEB128_MAX_BYTES`, return the byte count) and
  `leb128_decode_unsigned` / `leb128_decode_signed` (`(data, len, offset)` →
  `Leb128Status` with the value and bytes consumed), plus
  `leb128_status_message`.
- Faithful decode errors: out-of-bounds offset, over-wide (70-bit) sequence, and
  unterminated sequence. The signed arithmetic shift and the u64→i64
  reinterpretation use `memcpy`/explicit bit fills so the result is well-defined
  on every target (not reliant on implementation-defined behaviour).
- Tests use the crate's own WASM/DWARF vectors — zero, multi-byte, u32/i32 min &
  max, offset decoding, the three error conditions, and encode↔decode round
  trips including `u64::MAX` / `i64::MIN` — under GCC and Clang via `iso-harness`.
