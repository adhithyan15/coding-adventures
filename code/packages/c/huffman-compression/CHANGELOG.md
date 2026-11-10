# Changelog

All notable changes to `huffman-compression` (C) are documented in this file.

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `huffman_decompress` now treats the code-lengths table as hostile
  input. It returns 0 for `symbol_count > 256`, duplicate symbols, code lengths
  outside 1..56 (was 1..32), tables not sorted by length, over-subscribed
  (Kraft > 1) tables, and a declared `original_length` larger than 8 x payload
  bytes, which is checked BEFORE the output buffer is allocated (the buffer is
  now allocated once, not grown per symbol). Codes are 64-bit and decoded with
  per-length canonical tables, so each bit costs O(1) rather than a scan of the
  whole table, and decoding fails as soon as the accumulated bits reach the
  longest code length. Valid encoder output still round-trips.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `huffman-compression` crate:
  `huffman_compress` / `huffman_decompress` with canonical Huffman codes and the
  CMP04 wire format (length header + code-lengths table + LSB-first bit stream).
  Array-based Huffman tree (memory-safe); overflow-guarded malloc outputs.
- Tests (via the shared `iso-harness`) covering compress/decompress round-trips
  over single-symbol, skewed, natural-language, two-symbol, and full-alphabet
  inputs, plus header and edge-case checks — under GCC, Clang, and MSVC with
  strict ISO-conformance flags.
