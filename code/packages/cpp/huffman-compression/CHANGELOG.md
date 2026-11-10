# Changelog

All notable changes to `huffman-compression` (C++) are documented in this file.

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `decompress` now treats the code-lengths table as hostile input and
  throws `std::invalid_argument` for `symbol_count > 256`, duplicate symbols,
  code lengths outside 1..56 (was 1..32), tables not sorted by length,
  over-subscribed (Kraft > 1) tables, and a declared `original_length` larger
  than 8 x payload bytes, checked BEFORE the output is reserved. Codes are
  64-bit and decoded with per-length canonical tables, so each bit costs O(1)
  rather than a scan of the whole table, and decoding fails as soon as the
  accumulated bits reach the longest code length. Valid encoder output still
  round-trips.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `huffman-compression`
  crate: `ca::huffman::compress` / `ca::huffman::decompress` with canonical
  Huffman codes and the CMP04 wire format. std::vector-based Huffman tree.
- Tests (via the shared `iso-harness`) covering round-trips over varied
  distributions plus edge cases — under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
