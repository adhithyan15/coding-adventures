# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0

- Load every CSV file in a selected directory into an immutable SQL data source.
- Parse quoted RFC 4180-style fields and validate header and row widths.
- Coerce null, boolean, integer, real, and text values into the shared SQL types.
- Execute queries through the existing Haskell `sql-execution-engine` package.
