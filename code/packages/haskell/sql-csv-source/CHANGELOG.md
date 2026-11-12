# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Load every CSV file in a selected directory into an immutable SQL data source.
- Parse quoted RFC 4180-style fields and validate header and row widths.
- Coerce null, boolean, integer, real, and text values into the shared SQL types.
- Execute queries through the existing Haskell `sql-execution-engine` package.
