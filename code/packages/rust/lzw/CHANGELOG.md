# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `compress(data: &[u8]) -> Vec<u8>` — encodes to CMP03 wire format.
- `decompress(data: &[u8]) -> Vec<u8>` — decodes CMP03 wire format.
- Private `BitWriter` / `BitReader` for LSB-first variable-width bit I/O.
- Private `encode_codes` / `decode_codes` for code-stream encode/decode.
- Private `pack_codes` / `unpack_codes` for bit-packing serialisation.
- 29 tests, 0 failures, tricky-token edge case covered.
- Constants: `CLEAR_CODE`, `STOP_CODE`, `INITIAL_NEXT_CODE`, `INITIAL_CODE_SIZE`, `MAX_CODE_SIZE`.
