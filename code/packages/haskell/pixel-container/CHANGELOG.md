# Changelog

## Unreleased

- `BUILD` runs Cabal with `DEVELOPER_DIR=/Library/Developer/CommandLineTools` on Darwin so GHC 9.4.8 links with the Command Line Tools linker that accepts the CLT SDK's `arm64e.x1` stubs; other platforms are unchanged.

## 0.1.0 — 2026-04-20

- Initial release of IC00 `PixelContainer` for Haskell.
- `PixelContainer` record with `pcWidth`, `pcHeight`, `pcPixels`.
- `createPixelContainer`, `pixelAt`, `setPixel`, `fillPixels` primitives.
- Abstract `ImageCodec` type class with `mimeType`, `encode`, `decode`.
- Hspec test suite covering creation, bounds-checking, round-tripping,
  fill, and zero-size edge cases.
