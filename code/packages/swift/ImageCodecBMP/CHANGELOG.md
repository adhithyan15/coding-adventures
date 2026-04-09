# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-05

### Added

- `encodeBmp(_:) → [UInt8]` — encodes a `PixelContainer` as a 24-bit BGR BMP
  byte array. Uses negative height in the DIB header for top-down row order.
  Pads each row to a 4-byte boundary with zero bytes.
- `decodeBmp(_:) throws → PixelContainer` — decodes a 24-bit BGR BMP into a
  `PixelContainer`. Handles both positive-height (bottom-up) and
  negative-height (top-down) BMPs. Synthesises alpha = 255.
- `BmpCodec` struct — `ImageCodec` conformance wrapping the two free functions.
  `mimeType = "image/bmp"`.
- `ImageCodecBMPError` enum — `truncatedHeader`, `invalidSignature`,
  `unsupportedDibHeader`, `unsupportedBitDepth`, `unsupportedCompression`,
  `invalidDimensions`, `truncatedPixelData`. Conforms to `Error` and `Equatable`.
- `writeLE16`, `writeLE32`, `writeLE32Signed` — little-endian write helpers.
- `readLE16`, `readLE32`, `readLE32Signed` — little-endian read helpers.
- Literate programming style with extensive inline comments explaining BMP
  file structure, BGR byte order, row stride padding, and endianness.
- 22 XCTest test cases covering header field values, BGR order, stride
  padding, round-trip correctness, alpha synthesis, and all error paths.

### Design Notes

- We use a negative height value to store rows top-to-bottom. Standard BMP
  stores rows bottom-to-top (positive height); both are valid per the spec.
  Negative height avoids row-reversal logic and simplifies the encoder.
- Alpha is intentionally dropped on encode (24-bit BMP has no alpha channel).
  If the destination requires alpha-aware BMP, a future `encodeBmp32` function
  could output 32-bit BGRA BMP.
- `swift-tools-version: 5.9` for maximum compatibility.
