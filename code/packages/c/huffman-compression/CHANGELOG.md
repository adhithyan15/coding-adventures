# Changelog

All notable changes to `huffman-compression` (C) are documented in this file.

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
