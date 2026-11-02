# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-12

- Add deterministic heap-based Huffman tree construction.
- Add ordinary and canonical prefix-code tables.
- Add exact-count decoding and structural inspection.
- Add tests for errors, edge cases, determinism, round trips, and invariants.
