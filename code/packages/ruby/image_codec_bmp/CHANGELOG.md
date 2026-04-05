# Changelog — ImageCodecBmp (Ruby)

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-05

### Added

- **`ImageCodecBmp.encode_bmp(container)`** — Encodes an RGBA8 PixelContainer to a
  32-bit top-down BMP binary string. Produces correct BITMAPFILEHEADER and
  BITMAPINFOHEADER; stores pixels in BGRA order per the BMP specification.
- **`ImageCodecBmp.decode_bmp(data)`** — Decodes a 32-bit BMP binary string to an
  RGBA8 PixelContainer. Validates magic bytes, bit count (32), and compression (0).
  Handles both top-down (negative biHeight) and bottom-up (positive biHeight) files.
- **`ImageCodecBmp::BmpCodec`** — Class implementing the `ImageCodec` mixin interface
  (`mime_type`, `encode`, `decode`). MIME type: `"image/bmp"`.
- 22 minitest tests covering: magic/header fields, encode/decode roundtrips, BGRA
  channel order verification, error cases, and bottom-up row-flip handling.

### Dependencies

- `coding-adventures-pixel-container` (loaded via LOAD_PATH, not declared as a gem dep)

### Notes

- `encode_bmp` always writes top-down (negative biHeight) for simplicity.
- `String#pack("i<")` used for signed 32-bit LE integers (biWidth, biHeight).
- `String#pack("V")` used for unsigned 32-bit LE integers.
