# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Validate printable ASCII input for Code Set B.
- Encode the complete 107-symbol table with Start B, modulo-103 checksum, and stop.
- Expand attributed start, data, check, and stop runs.
- Emit backend-neutral paint scenes through the shared 1D barcode geometry.
