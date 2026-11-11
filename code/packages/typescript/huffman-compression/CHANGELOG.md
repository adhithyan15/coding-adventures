# Changelog

## [Unreleased]

### Fixed

- The "decodes a large valid input in linear time" test no longer relies on
  vitest's default 5 s timeout as its performance bound. Under `--coverage` on
  a loaded Windows runner the honest linear decode took 5.4 s and timed out. It
  now sets an explicit 60 s timeout, which still catches the O(n^2) decoder it
  guards against (hours for ~1.2 M bits), and compares the 400 KB output in a
  plain loop rather than with `toEqual`.

## [0.1.1] — 2026-11-10

### Security

- SEC-003: `decompress` now treats the code-lengths table as hostile input and
  throws for `symbol_count > 256`, duplicate symbols, code lengths outside
  1..56, tables not sorted by length, over-subscribed (Kraft > 1) tables, and a
  declared `original_length` larger than 8 x payload bytes (checked before
  allocating the output). Canonical codes are built with BigInt because
  JavaScript's `<<` masks the shift count to 5 bits. Decoding walks a flat trie
  reading bits in O(1) and fails as soon as the bits match no code. Valid
  encoder output still round-trips.

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of CMP04 Huffman compression and decompression.
- `compress(data: Uint8Array): Uint8Array` — compresses input using Huffman entropy coding and returns CMP04 wire-format bytes.
- `decompress(data: Uint8Array): Uint8Array` — decompresses CMP04 wire-format bytes back to the original input.
- CMP04 wire format: 8-byte header (original_length + symbol_count, big-endian uint32), followed by code-lengths table (N × 2 bytes sorted by length then symbol), followed by LSB-first packed bit stream.
- Delegates all Huffman tree construction and canonical code derivation to `@coding-adventures/huffman-tree` (DT27).
- Comprehensive vitest test suite covering round-trips, wire format verification for "AAABBC", edge cases (empty, single byte, all 256 values), and compression properties.
- Literate-programming style source with Knuth-style inline comments, analogies, and worked examples.
