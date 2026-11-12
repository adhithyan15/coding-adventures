# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Validate printable ASCII input for Code Set B.
- Encode the complete 107-symbol table with Start B, modulo-103 checksum, and stop.
- Expand attributed start, data, check, and stop runs.
- Emit backend-neutral paint scenes through the shared 1D barcode geometry.
