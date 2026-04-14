# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of CMP04 Huffman compression.
- `compress/1` — compresses a binary using canonical Huffman coding and emits
  the CMP04 wire format: 4-byte big-endian `original_length`, 4-byte big-endian
  `symbol_count`, N * 2 bytes of code-lengths table sorted by `{code_length,
  symbol_value}`, then an LSB-first bit-packed code stream. Handles empty input
  as a special case (8-byte header only).
- `decompress/1` — parses the CMP04 wire format, reconstructs canonical codes
  from the lengths table, unpacks the LSB-first bit stream, and decodes exactly
  `original_length` symbols. Returns `{:error, :too_short}` for inputs under 8
  bytes; handles truncated streams gracefully.
- `pack_bits_lsb_first/1` — packs a binary string of "0"/"1" characters into
  bytes using LSB-first ordering with zero-padding.
- `unpack_bits_lsb_first/1` — unpacks bytes into a string of "0"/"1"
  characters using LSB-first ordering.
- `canonical_codes_from_lengths/1` (private) — reconstructs canonical bit
  strings from a `{symbol, code_length}` list using the DEFLATE formula:
  `code = (prev_code + 1) << (len - prev_len)`.
- Depends on `coding_adventures_huffman_tree` (DT27) for tree construction and
  canonical code generation via `HuffmanTree.build/1` and
  `HuffmanTree.canonical_code_table/1`.
- 39 ExUnit tests covering: spec vectors (empty, single byte, AAABBC, all 256
  bytes), wire-format byte-level verification, round-trip breadth, compression
  effectiveness, bit-packing helpers, and security/robustness (truncated input,
  random bytes, crafted payloads).
- 94.74% test coverage.
