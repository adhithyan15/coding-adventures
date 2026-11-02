# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

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
