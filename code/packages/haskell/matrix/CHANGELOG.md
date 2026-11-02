# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Add immutable rectangular matrix construction and inspection.
- Add arithmetic, matrix multiplication, reductions, and element-wise math.
- Add immutable indexed updates, reshaping, extraction, and comparison helpers.
