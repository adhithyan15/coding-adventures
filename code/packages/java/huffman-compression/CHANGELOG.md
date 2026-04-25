# Changelog — huffman-compression (Java)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-25

### Added
- `HuffmanCompression.compress(byte[])` — encodes a byte array into CMP04
  wire format using canonical Huffman coding.
  - Builds a frequency histogram (int[256]).
  - Delegates tree construction and canonical code derivation to
    `com.codingadventures.huffmantree.HuffmanTree` (DT27).
  - Sorts code-lengths by (length, symbol) for the wire-format header.
  - Packs the encoded bit string LSB-first into bytes (same convention as
    LZW/GIF).
  - Handles edge cases: empty input returns 8-byte header; single distinct
    symbol uses 1-bit codes.
- `HuffmanCompression.decompress(byte[])` — decodes CMP04 wire-format bytes.
  - Parses the 8-byte header and code-lengths table.
  - Reconstructs canonical codes via the DEFLATE assignment rule:
    `code = (code + 1) << (next_length - prev_length)`.
  - Unpacks the LSB-first bit stream.
  - Decodes exactly `original_length` symbols by prefix accumulation.
  - Throws `IllegalArgumentException` if the bit stream is exhausted before
    all symbols are decoded.
- `packBitsLsbFirst(String)` — private helper packing a '0'/'1' string into
  bytes with the LSB-first convention.
- 42 unit tests covering:
  - Round-trip fidelity (13 tests)
  - Exact wire-format bytes including the "AAABBC" worked example (7 tests)
  - Edge cases: empty, null, single byte, high bytes, null bytes, short
    header, truncated stream (11 tests)
  - Compression effectiveness: skewed, repeated, uniform distributions (3 tests)
  - Determinism (3 tests)
  - Error handling: bit-stream exhaustion (1 test)
- `build.gradle.kts` with composite build dependency on `huffman-tree`.
- `settings.gradle.kts` with `includeBuild("../huffman-tree")`.
- `BUILD` / `BUILD_windows`, `README.md`, `.gitignore`.
