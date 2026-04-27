# Changelog — coding-adventures-huffman-compression

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of CMP04 Huffman compression.
- `compress(data)` — encodes bytes using Huffman entropy coding.
- `decompress(data)` — decodes CMP04 wire-format bytes.
- CMP04 wire format: 8-byte header (original_length + symbol_count) + code-lengths
  table + LSB-first packed bit stream.
- Depends on `coding-adventures-huffman-tree` (DT27) for tree construction and
  canonical code derivation. No embedded tree logic.
- Edge cases: empty input, single distinct byte, bytearray input.
- 37 unit tests with ≥90% coverage.
