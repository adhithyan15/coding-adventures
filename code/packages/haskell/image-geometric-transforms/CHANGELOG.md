# Changelog

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
