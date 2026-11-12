# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-19

- Add a pure Haskell CMP05 encoder built on the existing LZSS and Huffman-tree
  packages.
- Add strict self-describing wire decoding with canonical-table, prefix,
  backreference, and output-length validation.
- Add package-native coverage for empty, literal-only, repetitive, binary,
  parameter-error, and malformed-stream behavior.
