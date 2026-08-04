# Changelog

## 0.1.0 - 2026-07-12

- Add CMP04 compression and decompression for strict `ByteString` values.
- Reuse the deterministic Haskell `huffman-tree` implementation.
- Add canonical code reconstruction and LSB-first bit packing.
- Validate malformed headers, length tables, prefixes, and truncated streams.
- Add round-trip, wire-format, effectiveness, determinism, and error tests.
