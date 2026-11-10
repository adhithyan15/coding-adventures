# Changelog

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `Decompress` now treats the code-lengths table as hostile input.
  It returns an error for `symbol_count > 256`, duplicate symbols, code
  lengths outside 1..56, tables not sorted by length, over-subscribed
  (Kraft > 1) tables, and a declared `original_length` larger than 8 x payload
  bytes (checked before allocating). Bits are read in O(1) from the payload and
  decoding fails once the accumulated bits reach the longest code length.
  Valid encoder output still round-trips.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of Huffman compression (CMP04).
- `Compress(data []byte) ([]byte, error)` — encodes to CMP04 wire format using canonical Huffman codes.
- `Decompress(data []byte) ([]byte, error)` — decodes CMP04 wire format back to original bytes.
- Internal `bitBuilder` for LSB-first bit packing.
- Internal `unpackBits` for LSB-first bit stream expansion.
- Internal `buildCanonicalCodes` for reconstructing canonical codes from a sorted (symbol, length) table.
- Full test suite: round-trip tests for empty, single-byte, single-repeated-byte, all-256-bytes, skewed distributions, and natural text; wire-format exact-byte verification for "AAABBC"; error-path tests for truncated input.
- Depends on DT27 `huffman-tree` package via `replace` directive.
