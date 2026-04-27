# Changelog — huffman-compression (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of CMP04 Huffman compression in Swift.
- `compress(_ data: [UInt8]) throws -> [UInt8]` — encodes input using canonical
  Huffman codes and returns CMP04 wire-format bytes.
- `decompress(_ data: [UInt8]) throws -> [UInt8]` — decodes CMP04 wire-format
  bytes back to the original data.
- CMP04 wire format: `original_length` (BE uint32) + `symbol_count` (BE uint32)
  + code-lengths table (sorted by length then symbol) + LSB-first packed bit stream.
- Canonical Huffman codes: only code lengths are stored; the decoder reconstructs
  the exact canonical code table from lengths alone (DEFLATE-style).
- LSB-first bit packing and unpacking helpers.
- Big-endian uint32 read/write helpers.
- `String.leftPadded(toLength:with:)` extension for zero-padding binary strings.
- `HuffmanCompressionError` enum with cases: `dataTooShort`, `truncatedCodeTable`,
  `invalidCodeLength`, `bitStreamExhausted`, `huffmanTreeError`.
- Comprehensive XCTest suite covering:
  - Exact wire-format vector for "AAABBC" per the CMP04 spec.
  - Round-trip tests for strings, binary data, and all 256 byte values.
  - Edge cases: empty input, single byte, single repeated symbol.
  - Header parsing correctness (big-endian byte order, symbol count).
  - Code-lengths table structure and sort order.
  - Compression determinism and effectiveness.
  - Security: malformed input, truncated data, and zero-length code entries.
- `Package.swift` with local dependency on `../huffman-tree` (DT27).
- `BUILD` and `BUILD_windows` scripts for the monorepo build tool.
- `README.md` with wire-format documentation, usage examples, and an end-to-end
  worked example for "AAABBC".
