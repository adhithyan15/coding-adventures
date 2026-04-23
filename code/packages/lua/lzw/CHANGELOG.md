# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `compress(data)` — encodes string to CMP03 wire format.
- `decompress(data)` — decodes CMP03 wire format to string.
- `encode_codes` / `decode_codes` for code-stream encode/decode.
- `pack_codes` / `unpack_codes` for bit-packing serialisation.
- `bw_*` / `br_*` private helpers for LSB-first variable-width bit I/O.
- 42 tests, 0 failures, tricky-token edge case and all spec vectors covered.
- Constants: `CLEAR_CODE`, `STOP_CODE`, `INITIAL_NEXT_CODE`, `INITIAL_CODE_SIZE`, `MAX_CODE_SIZE`.
