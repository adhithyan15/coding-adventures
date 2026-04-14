# Changelog

## [0.1.0] — 2026-04-13

### Added

- Initial implementation of CMP06 Brotli compression/decompression for TypeScript.
- `compress(data: Uint8Array): Uint8Array` — two-pass Brotli compressor:
  - Pass 1: O(n²) LZ matching with 65,535-byte sliding window, minimum match length 4.
  - Pass 2a: frequency tallying across ICC codes, distance codes, and 4 literal context buckets.
  - Pass 2b: canonical Huffman tree building via `@coding-adventures/huffman-tree` (DT27).
  - Pass 2c: LSB-first bit-stream encoding using ICC Huffman codes + insert/copy/distance extra bits.
- `decompress(data: Uint8Array): Uint8Array` — wire-format parser and decoder:
  - Parses 10-byte header + 6 code-length tables.
  - Reconstructs canonical Huffman reverse maps.
  - Decodes ICC-driven bit stream into the original byte sequence.
- `ICC_TABLE` — inline 64-entry insert-copy code table as specified in CMP06.
- `DIST_TABLE` — 32-entry distance code table extending CMP05's 24 codes to cover offsets up to 65,535.
- `literalContext()` — 4-bucket context function (space/punct=0, digit=1, upper=2, lower=3).
- `findLongestMatch()` — inline LZ matcher (no LZSS dependency per spec).
- CMP06 wire format implementation: 10-byte header + per-symbol code-length tables + LSB-first bit stream.
- Empty input special case: canonical 13-byte encoding.
- Single-symbol Huffman tree special case: code "0" (length 1).
- `tests/brotli.test.ts` — 42 test cases covering all 10 spec tests plus:
  - Wire format header field verification
  - Context bucket population verification
  - Long-distance match (offset > 4096, exercising dist codes 24–31)
  - Manual wire format construction and parsing (spec test 10)
  - Determinism check (same input → identical bytes every time)
  - Various data types: English text, binary blobs, all-zero, all-0xFF, alphabet runs
- `BUILD` and `BUILD_windows` — build scripts with transitive `npm install` chaining.
- `README.md` — algorithm overview, wire format reference, API documentation.
- `CHANGELOG.md` — this file.
