# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Validate 11- and 12-digit ASCII UPC-A input.
- Compute and verify the required modulo-10 check digit.
- Encode the complete left/right digit tables and 95-module guard structure.
- Emit attributed runs and backend-neutral paint scenes through the shared 1D geometry.
