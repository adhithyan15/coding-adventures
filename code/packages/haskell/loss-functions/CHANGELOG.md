# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 - 2026-07-18

- Add mean squared, mean absolute, binary cross-entropy, and categorical
  cross-entropy losses.
- Add derivatives for every loss and clamp probability inputs away from zero
  and one.
- Reject empty and length-mismatched input vectors explicitly.
