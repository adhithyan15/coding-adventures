# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Add UUID parsing, validation, formatting, and comparison.
- Add v1, v3, v4, v5, and v7 generation.
- Add standard namespaces and nil/max constants.
