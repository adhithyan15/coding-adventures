# Changelog — coding-adventures-brotli

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-13

### Added

- Initial implementation of CMP06 Brotli compression algorithm.
- `compress(data: bytes) -> bytes` — compresses input using the CMP06 wire
  format with context-dependent literal trees, insert-and-copy commands, and
  a 65535-byte sliding window.
- `decompress(data: bytes) -> bytes` — decompresses CMP06 wire-format bytes
  back to the original input.
- **4 literal context buckets** based on the preceding byte:
  - bucket 0: space/punctuation
  - bucket 1: digit ('0'–'9')
  - bucket 2: uppercase letter ('A'–'Z')
  - bucket 3: lowercase letter ('a'–'z')
- **64 ICC (insert-copy code) codes** (codes 0–62 for insert+copy operations,
  code 63 as the end-of-data sentinel).
- **32 distance codes** (codes 0–31, extending the window to 65535 bytes via
  codes 24–31 beyond CMP05's 4096-byte limit).
- **10-byte wire format header** with counts for ICC, distance, and 4 literal
  code-length tables.
- LSB-first bit packing (same convention as CMP05/DEFLATE).
- Single-symbol Huffman tree encoding (code "0", length 1).
- Empty input special case encoding per spec.
- Comprehensive test suite (>90% coverage):
  - All 10 spec-mandated test cases.
  - Unit tests for ICC table, distance codes, context function, bit I/O,
    canonical code reconstruction, and wire format header.
  - Parametric stress tests with random seeds and various input lengths.

### Implementation notes

- LZ matching is performed inline (O(n²) sliding window scan). The insert-copy
  command structure does not map onto the CMP02 LZSS flat token stream.
- The `coding-adventures-huffman-tree` (DT27) dependency is used for all
  Huffman tree construction and canonical code table generation.
- No LZSS dependency (CMP02) is required.
- Literate programming style: every non-trivial step is explained inline.
