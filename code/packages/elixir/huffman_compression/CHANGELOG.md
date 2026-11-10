# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `decompress/1` now treats the code-lengths table as hostile input
  and returns `{:error, reason}` for `symbol_count > 256`, duplicate symbols,
  code lengths outside 1..56, tables not sorted by length, over-subscribed
  (Kraft > 1) tables, a declared `original_length` larger than 8 x payload
  bytes, a payload that ends early, and bits that match no code. Previously a
  truncated or undecodable stream silently returned a partial binary. Bits are
  now read in O(1) with `:binary.at/2` instead of `String.length/1` per symbol
  (which made decoding quadratic), and decoding fails as soon as the
  accumulated bits reach the longest code length. Valid encoder output still
  round-trips.

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
