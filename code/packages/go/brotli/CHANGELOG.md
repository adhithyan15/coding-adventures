# Changelog

## 0.1.0 — 2026-04-13

### Added

- Initial implementation of CMP06 Brotli compression and decompression.
- `Compress(data []byte) ([]byte, error)` — three-pass Brotli encoder.
- `Decompress(data []byte) ([]byte, error)` — CMP06 wire-format decoder.
- Insert-copy code (ICC) table: 64 codes (0–62 for data, 63 for end-of-data sentinel).
- Distance code table: 32 codes covering offsets 1–65535 (codes 24–31 extend CMP05's range).
- Four literal context buckets based on the preceding byte's character class
  (space/punct=0, digit=1, uppercase=2, lowercase=3).
- Separate canonical Huffman tree per context bucket, plus ICC and distance trees.
- 65535-byte sliding window LZ matching (inline, no LZSS dependency).
- LSB-first bit stream packing via `bitBuilder`.
- Canonical Huffman code reconstruction for the decoder.
- Special-case encoding for empty input per CMP06 spec.
- Dependency on `huffman-tree` (DT27) for Huffman tree construction.
- Flush-literal encoding: trailing literals that cannot be bundled into a
  regular ICC command are emitted AFTER the sentinel (ICC=63) in the bit
  stream. The decoder reads them until `original_length` bytes are produced.
  This cleanly handles pure-literal inputs of any size without dummy copies.
- `findBestICCCopy` helper: finds the largest encodable copy length ≤ requested
  for a given insert length, handling ICC table gaps (e.g., copy=7 is not
  representable in any ICC code).
