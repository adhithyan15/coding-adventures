# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Add immutable rectangular matrix construction and inspection.
- Add arithmetic, matrix multiplication, reductions, and element-wise math.
- Add immutable indexed updates, reshaping, extraction, and comparison helpers.
