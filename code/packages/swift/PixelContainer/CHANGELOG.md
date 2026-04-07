# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-05

### Added

- `PixelContainer` struct — fixed RGBA8 pixel buffer. Stores `width × height`
  pixels as a flat `[UInt8]` in row-major order, 4 bytes per pixel (R, G, B, A).
  Byte offset formula: `(y × width + x) × 4`.
- `ImageCodec` protocol — interface every image format encoder/decoder must
  implement. Provides `mimeType`, `encode(_:) → [UInt8]`, and
  `decode(_:) throws → PixelContainer`.
- `PixelContainerError` enum — `invalidDimensions`, `invalidData`. Conforms to
  `Error` and `Equatable`.
- `pixelAt(_:x:y:) → (UInt8, UInt8, UInt8, UInt8)` — bounds-safe pixel read;
  returns `(0, 0, 0, 0)` for out-of-bounds coordinates.
- `setPixel(_:x:y:r:g:b:a:)` — bounds-safe pixel write; no-op if out of bounds.
- `fillPixels(_:r:g:b:a:)` — fills the entire buffer with a single RGBA value
  using a single linear pass (cache-friendly).
- Literate programming style with extensive inline documentation covering the
  memory layout formula, row-major order rationale, and RGBA8 design choice.
- Comprehensive test suite with 24 test cases covering initialization, buffer
  length, pixel reads/writes, out-of-bounds safety, fill operations, row-major
  ordering, error enum equality, and edge dimensions.

### Design Notes

- `PixelContainer` is a pure value type (`struct`) with no methods — mutation
  happens through free functions (`setPixel`, `fillPixels`). This is consistent
  with the educational C-style theme of the coding-adventures stack.
- `ImageCodec` uses `throws` on `decode` (decoding can fail on malformed input)
  but not on `encode` (every valid pixel buffer can always be encoded).
- `swift-tools-version: 5.9` for maximum compatibility.
