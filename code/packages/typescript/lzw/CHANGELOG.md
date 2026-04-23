# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `compress(data: Uint8Array): Uint8Array` — encodes to CMP03 wire format.
- `decompress(data: Uint8Array): Uint8Array` — decodes CMP03 wire format.
- `BitWriter` / `BitReader` classes for LSB-first variable-width bit I/O.
- `encodeCodes` / `decodeCodes` for code-stream encode/decode.
- `packCodes` / `unpackCodes` for bit-packing serialisation.
- 38 tests, 0 failures, tricky-token edge case and all spec vectors covered.
- Constants: `CLEAR_CODE`, `STOP_CODE`, `INITIAL_NEXT_CODE`, `INITIAL_CODE_SIZE`, `MAX_CODE_SIZE`.
