# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test all` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0

- Add a pure Haskell coordinator for Code 39, Codabar, Code 128, EAN-13,
  ITF, and UPC-A.
- Preserve typed encoder and layout failures across the shared pipeline.
- Render backend-neutral paint scenes through the pure ASCII Paint VM.
