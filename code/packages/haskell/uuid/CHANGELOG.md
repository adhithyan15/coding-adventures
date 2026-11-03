# Changelog

## Unreleased

- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Add UUID parsing, validation, formatting, and comparison.
- Add v1, v3, v4, v5, and v7 generation.
- Add standard namespaces and nil/max constants.
