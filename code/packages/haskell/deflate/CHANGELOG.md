# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 - 2026-07-19

- Add a pure Haskell CMP05 encoder built on the existing LZSS and Huffman-tree
  packages.
- Add strict self-describing wire decoding with canonical-table, prefix,
  backreference, and output-length validation.
- Add package-native coverage for empty, literal-only, repetitive, binary,
  parameter-error, and malformed-stream behavior.
