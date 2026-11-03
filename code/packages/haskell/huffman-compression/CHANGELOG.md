# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-12

- Add CMP04 compression and decompression for strict `ByteString` values.
- Reuse the deterministic Haskell `huffman-tree` implementation.
- Add canonical code reconstruction and LSB-first bit packing.
- Validate malformed headers, length tables, prefixes, and truncated streams.
- Add round-trip, wire-format, effectiveness, determinism, and error tests.
