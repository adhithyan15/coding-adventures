# Changelog

## 0.1.0 — 2026-04-20

- Initial release of IC00 `PixelContainer` for Haskell.
- `PixelContainer` record with `pcWidth`, `pcHeight`, `pcPixels`.
- `createPixelContainer`, `pixelAt`, `setPixel`, `fillPixels` primitives.
- Abstract `ImageCodec` type class with `mimeType`, `encode`, `decode`.
- Hspec test suite covering creation, bounds-checking, round-tripping,
  fill, and zero-size edge cases.
