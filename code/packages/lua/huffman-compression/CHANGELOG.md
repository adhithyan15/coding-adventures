# Changelog

All notable changes to `coding-adventures-huffman-compression` will be
documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-12

### Added

- Initial release of the CMP04 Huffman compression package for Lua.
- `M.compress(data)` — compresses a Lua string to CMP04 wire-format bytes.
  - Counts byte frequencies via `data:byte(i)` iteration.
  - Builds a Huffman tree using `coding-adventures-huffman-tree` (DT27).
  - Generates canonical Huffman codes via `tree:canonical_code_table()`.
  - Sorts the code-length table by `(length, symbol)` for the wire header.
  - Packs the encoded bit stream LSB-first into raw bytes.
  - Assembles the CMP04 header: `original_length` + `symbol_count` + table + bits.
- `M.decompress(data)` — decompresses CMP04 wire-format bytes back to a string.
  - Parses the 8-byte header using `string.unpack(">I4", ...)`.
  - Parses the code-length table and reconstructs canonical codes.
  - Unpacks the bit stream LSB-first.
  - Decodes `original_length` symbols via greedy prefix matching.
  - Handles empty input, single-symbol inputs, and all 256 distinct byte values.
- Comprehensive Busted test suite (`tests/test_huffman_compression.lua`):
  - Round-trip spec vectors (empty, single byte, AAABBC, ABABAB, all-256).
  - Wire format verification for "AAABBC" (exact byte positions).
  - Header property checks (big-endian uint32, entry sort order, determinism).
  - Edge case robustness (truncated input, zero-byte header, null bytes).
  - Compression effectiveness (single-symbol, biased distribution).
  - Decode correctness (exact lengths, long strings, pseudo-random data).
- `coding-adventures-huffman-compression-0.1.0-1.rockspec` for LuaRocks.
- `BUILD` file for the monorepo build system (ensures huffman-tree is installed first).
- `README.md` with usage examples, wire format diagram, and series context.
