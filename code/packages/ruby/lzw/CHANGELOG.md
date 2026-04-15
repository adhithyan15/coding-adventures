# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `CodingAdventures::LZW.compress(data)` — encodes String to CMP03 wire format.
- `CodingAdventures::LZW.decompress(data)` — decodes CMP03 wire format to String.
- `encode_codes` / `decode_codes` for code-stream encode/decode.
- `pack_codes` / `unpack_codes` for bit-packing serialisation.
- `BitWriter` / `BitReader` for LSB-first variable-width bit I/O.
- 30 tests, 0 failures, tricky-token edge case covered.
