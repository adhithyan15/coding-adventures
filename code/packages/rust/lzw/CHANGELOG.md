# Changelog

## [Unreleased]

### Added

- Add required_capabilities.json (pure computation).

### Security

- `decompress` stops expanding codes once the declared `original_length` is reached (each code can emit ~4 KiB, so a small stream could previously expand far beyond it before the final truncate). Valid streams are unchanged.
- Added `tests/hostile_headers.rs` (allocation guard, hostile headers, exhaustive <=2-byte and seeded random round trips).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW encoding and decoding (CMP03 spec).
- `BitWriter` and `BitReader` helpers for LSB-first variable-width bit packing.
- `compress` / `decompress` one-shot CMP03 wire-format API.
- Variable-width codes (9–16 bits) with automatic code-size growth.
- Dictionary reset (CLEAR_CODE) when `next_code` reaches 2^16.
- Tricky-token edge case (`code == next_code`) handled in decoder.
- Security: invalid code, missing CLEAR_CODE, and short-header errors returned
  as `Err(String)` rather than panics.
- 30 unit tests covering spec vectors, round-trips, bit I/O, and error paths.
