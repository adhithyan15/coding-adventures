# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-19

- Add immutable diagnostic and type-check result values.
- Add a functional checker wrapper for structurally compatible checker
  functions.
- Add pure generic hook registration and exact-before-wildcard dispatch.
- Add reusable diagnostic collection, location handling, lifecycle reset, and
  node-kind normalization.
- Cover protocol, dispatch, fall-through, wildcard, argument, ordering, and
  validation behavior with package-native tests.
