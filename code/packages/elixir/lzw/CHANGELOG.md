# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `compress/1` — encodes binary to CMP03 wire format.
- `decompress/1` — decodes CMP03 wire format to binary.
- `encode_codes/1` / `decode_codes/1` for code-stream encode/decode.
- `pack_codes/2` / `unpack_codes/1` for bit-packing serialisation.
- Private `bw_*` / `br_*` helpers for LSB-first variable-width bit I/O.
- 29 tests, 0 failures, 93%+ coverage, tricky-token edge case covered.
- Accessor functions for all constants.
