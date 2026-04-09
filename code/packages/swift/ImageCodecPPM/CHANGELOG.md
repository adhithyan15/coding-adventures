# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-05

### Added

- `encodePpm(_:) → [UInt8]` — encodes a `PixelContainer` as a P6 binary PPM
  byte array. Header format: `"P6\n<width> <height>\n255\n"`. Pixel data is
  raw RGB with no row padding and no alpha channel.
- `decodePpm(_:) throws → PixelContainer` — cursor-based P6 decoder. Skips
  whitespace and `#` comment lines between header tokens. Supports both
  space/newline-separated headers. Synthesises alpha = 255.
- `PpmCodec` struct — `ImageCodec` conformance wrapping the two free functions.
  `mimeType = "image/x-portable-pixmap"`.
- `ImageCodecPPMError` enum — `invalidMagic`, `malformedHeader`,
  `invalidDimensions`, `unsupportedMaxval`, `truncatedPixelData`. Conforms to
  `Error` and `Equatable`.
- Literate programming style with extensive inline comments covering the PPM
  format structure, cursor-based parsing strategy, and comparison with BMP/QOI.
- 22 XCTest test cases covering header structure, RGB order, round-trip
  correctness, comment line handling, alpha synthesis, error paths, and
  edge dimensions (single row, single column).

### Design Notes

- The decoder uses a cursor (`pos: Int`) advanced through the byte array
  rather than splitting into lines or allocating substrings. This is more
  efficient and avoids dependencies on `Foundation`.
- We only support maxval = 255 (8-bit channels). A future extension could
  support maxval = 65535 (16-bit channels, 2 bytes per component).
- `swift-tools-version: 5.9` for maximum compatibility.
