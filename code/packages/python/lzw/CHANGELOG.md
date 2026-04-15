# Changelog

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
