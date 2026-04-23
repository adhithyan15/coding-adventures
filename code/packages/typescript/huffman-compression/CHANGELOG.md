# Changelog

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of CMP04 Huffman compression and decompression.
- `compress(data: Uint8Array): Uint8Array` — compresses input using Huffman entropy coding and returns CMP04 wire-format bytes.
- `decompress(data: Uint8Array): Uint8Array` — decompresses CMP04 wire-format bytes back to the original input.
- CMP04 wire format: 8-byte header (original_length + symbol_count, big-endian uint32), followed by code-lengths table (N × 2 bytes sorted by length then symbol), followed by LSB-first packed bit stream.
- Delegates all Huffman tree construction and canonical code derivation to `@coding-adventures/huffman-tree` (DT27).
- Comprehensive vitest test suite covering round-trips, wire format verification for "AAABBC", edge cases (empty, single byte, all 256 values), and compression properties.
- Literate-programming style source with Knuth-style inline comments, analogies, and worked examples.
