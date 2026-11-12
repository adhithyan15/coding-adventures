# grammar-tools changelog

- Unreleased: `BUILD_windows` now runs `cabal test` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- Unreleased: the Darwin BUILD now runs Cabal with the Command Line Tools
  developer dir (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC
  9.4.8's CLT SDK sysroot gets the matching linker on macOS CI with CLT 27.
- `0.1.0`: initial Haskell grammar-tools package with token grammar parsing, parser grammar parsing, cross-validation, and source compilation helpers.
