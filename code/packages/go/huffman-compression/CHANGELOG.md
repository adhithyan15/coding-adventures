# Changelog

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
