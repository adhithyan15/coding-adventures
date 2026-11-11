# Changelog

## [Unreleased]

### Security

- No code change: no allocation is sized from the header.
- Added hostile_headers_test.go: hostile headers checked for bounded time and cumulative allocation, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW compression (CMP03).
- `Compress(data []byte) []byte` — encodes to CMP03 wire format.
- `Decompress(data []byte) []byte` — decodes CMP03 wire format.
- Internal `bitWriter` / `bitReader` for LSB-first variable-width bit I/O.
- Internal `encodeCodes` / `decodeCodes` for code-stream encode/decode.
- Full test suite: 16 tests, 90%+ coverage, tricky-token edge case covered.
- Constants: `ClearCode`, `StopCode`, `InitialNextCode`, `InitialCodeSize`, `MaxCodeSize`.
