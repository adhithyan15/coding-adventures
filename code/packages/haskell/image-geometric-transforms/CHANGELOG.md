# Changelog

## Unreleased

- `BUILD_windows` now runs `cabal test` instead of skipping with "haskell support
  not enabled in windows CI yet", so this package is built and tested on Windows.
- The Darwin BUILD now runs Cabal with the Command Line Tools developer dir
  (`DEVELOPER_DIR=/Library/Developer/CommandLineTools`), so GHC 9.4.8's CLT SDK
  sysroot gets the matching linker on macOS CI with Command Line Tools 27.

## 0.1.0 — 2026-04-20

- Initial release of IMG04 geometric transforms for Haskell.
- Lossless: `flipHorizontal`, `flipVertical`, `rotate90CW`,
  `rotate90CCW`, `rotate180`, `crop`.
- Continuous: `scale`, `rotate` (with `RotateBounds = Fit | Crop`),
  `translate`, `affine`, `perspectiveWarp`.
- Interpolation: `Nearest`, `Bilinear`, `Bicubic` (Catmull-Rom).
- Out-of-bounds policies: `Zero`, `Replicate`, `Reflect`, `Wrap`.
- sRGB-aware sampling: bilinear and bicubic blend in linear light.
- Hspec test suite.
