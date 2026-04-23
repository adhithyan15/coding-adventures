# Changelog

All notable changes to the Swift LZW package will be documented here.

## [0.1.0] — 2026-04-23

### Added

- Initial release of `LZW` Swift package (CMP03).
- `compress(_ data: [UInt8]) -> [UInt8]` — full LZW pipeline: encode → bit-pack with
  4-byte big-endian `original_length` header.
- `decompress(_ data: [UInt8]) -> [UInt8]` — full LZW pipeline: unpack → decode, with
  output trimmed to `original_length` to discard padding bits.
- Private `encodeCodes(_ data: [UInt8]) -> ([UInt32], Int)` — flat-HashMap LZW encoder.
  Emits `clearCode`, codes, optional mid-stream `clearCode` when dict is full, `stopCode`.
- Private `decodeCodes(_ codes: [UInt32]) -> [UInt8]` — LZW decoder with tricky-token
  edge case: when `code == nextCode`, constructs entry as `dict[prevCode] + [dict[prevCode][0]]`.
- Private `packCodes(_ codes: [UInt32], originalLength: Int) -> [UInt8]` — variable-width
  LSB-first bit-packing using `BitWriter`. Tracks `codeSize` and `nextCode` to grow
  `codeSize` at the right boundaries; resets on `clearCode`.
- Private `unpackCodes(_ data: [UInt8]) -> ([UInt32], Int)` — unpacks LSB-first packed
  bytes using `BitReader`. Stops on `stopCode` or data exhaustion.
- `BitWriter` struct: `UInt64` accumulator, `write(_:size:)`, `flush()`.
- `BitReader` struct: `UInt64` accumulator, `read(size:) -> UInt32?`, `exhausted` guard.
- Exported constants: `clearCode`, `stopCode`, `initialNextCode`, `initialCodeSize`,
  `maxCodeSize`.
- 29 unit tests covering constants, encode/decode, pack/unpack, and round-trip for empty,
  single byte, two distinct bytes, repeated pairs, tricky token, clear mid-stream,
  invalid code skip, long string, binary data, all-zeros, all-0xFF, and repetitive data.
- `BUILD` (macOS/Linux) and `BUILD_windows` (Windows CI skip) files.
- `.gitignore` — excludes `.build/` to avoid committing Swift compiler artefacts.
