# Changelog

## [Unreleased]

### Security

- No code change: no allocation is sized from the header.
- Added test_hostile_headers.py: hostile headers (0xFFFFFFFF counts/lengths, counts beyond the data) checked for bounded time and memory, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `compress(data)` — encodes bytes to CMP03 wire format (variable-width bit-packed codes).
- `decompress(data)` — decodes CMP03 wire format back to original bytes.
- `_encode_codes` / `_decode_codes` — internal code-stream encoder/decoder.
- `_pack_codes` / `_unpack_codes` — bit-packing serialisation layer.
- `_BitWriter` / `_BitReader` — LSB-first variable-width bit I/O helpers.
- Full test suite with 95%+ coverage including the tricky-token edge case.
- Constants: `CLEAR_CODE`, `STOP_CODE`, `INITIAL_NEXT_CODE`, `INITIAL_CODE_SIZE`, `MAX_CODE_SIZE`.
