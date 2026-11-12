# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 — 2026-04-20

- Initial release of IC00 `PixelContainer` for Haskell.
- `PixelContainer` record with `pcWidth`, `pcHeight`, `pcPixels`.
- `createPixelContainer`, `pixelAt`, `setPixel`, `fillPixels` primitives.
- Abstract `ImageCodec` type class with `mimeType`, `encode`, `decode`.
- Hspec test suite covering creation, bounds-checking, round-tripping,
  fill, and zero-size edge cases.
