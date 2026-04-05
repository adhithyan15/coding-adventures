# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-05

### Added

- `encodeQoi(_:) → [UInt8]` — encodes a `PixelContainer` as a QOI byte array.
  Implements all 6 QOI ops in priority order: RUN, INDEX, DIFF, LUMA, RGB, RGBA.
  Splits runs > 62 pixels into multiple RUN chunks. Outputs RGBA (4 channels),
  sRGB colorspace (0). Header uses big-endian uint32 for width and height.
- `decodeQoi(_:) throws → PixelContainer` — decodes a QOI byte array. Dispatches
  on top 2 bits (or 0xFE/0xFF for RGB/RGBA). Verifies magic bytes and end marker.
  Updates seen table on every pixel. Handles runs spanning multiple RUN chunks.
- `QoiCodec` struct — `ImageCodec` conformance. `mimeType = "image/qoi"`.
- `ImageCodecQOIError` enum — `invalidMagic`, `truncatedHeader`,
  `invalidDimensions`, `unsupportedChannels`, `truncatedData`, `missingEndMarker`.
  Conforms to `Error` and `Equatable`.
- `qoiHash(_:) → Int` — hash function `(R×3 + G×5 + B×7 + A×11) % 64`.
- `Pixel` struct — internal RGBA value type with `Equatable` conformance.
- `writeBE32`, `readBE32` — big-endian I/O helpers.
- `qoiEndMarker` constant — `[0,0,0,0,0,0,0,1]`.
- Extensive literate comments covering all 6 chunk types, the bias encoding
  scheme (DIFF +2, LUMA +32/+8), the seen-pixels hash table, endianness, and
  delta wrapping arithmetic.
- 23 XCTest test cases covering header fields, hash function, compression
  ratio, long-run splitting, round-trips (solid, checkerboard, gradient,
  random, RGBA alpha), and all decode error paths.

### Design Notes

- Priority order for encoding: RUN → INDEX → DIFF → LUMA → RGB → RGBA.
  INDEX is checked before DIFF because hitting the hash table costs only 1
  byte regardless of channel delta sizes.
- We always write `channels = 4` (RGBA) even for images that have alpha = 255
  everywhere, to keep the encoder simple and the output self-describing.
- The `Pixel` struct is internal — callers use `PixelContainer` and the free
  functions `encodeQoi`/`decodeQoi`.
- `swift-tools-version: 5.9` for maximum compatibility.
