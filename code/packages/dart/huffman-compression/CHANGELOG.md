# Changelog

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `decompress` now rejects a declared `original_length` larger than
  8 x payload bytes BEFORE allocating the output buffer, so a tiny hostile
  input can no longer force an allocation of up to `maxDecompressedSize`.
  The existing validation (symbol_count <= 256, no duplicate symbols, code
  lengths 1..16, table sorted, Kraft check, fail-fast on unmatched prefixes,
  O(1) bit reader) is unchanged and now covered by explicit hostile-input
  tests. Valid encoder output still round-trips.

## [0.1.0] - 2026-04-18

### Added

- Added the Dart CMP04 Huffman compression and decompression implementation.
- Added canonical code-length serialisation, LSB-first bit packing, and strict decoder validation.
- Added regression tests for malformed headers, invalid code tables, truncated streams, and padding errors.
