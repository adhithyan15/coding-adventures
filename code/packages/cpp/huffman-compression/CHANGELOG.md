# Changelog

All notable changes to `huffman-compression` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `huffman-compression`
  crate: `ca::huffman::compress` / `ca::huffman::decompress` with canonical
  Huffman codes and the CMP04 wire format. std::vector-based Huffman tree.
- Tests (via the shared `iso-harness`) covering round-trips over varied
  distributions plus edge cases — under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
