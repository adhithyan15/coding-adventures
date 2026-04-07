# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of `CodingAdventures::ImageCodecBMP` (IC01)
- `mime_type()` — returns `'image/bmp'`
- `encode_bmp($container)` — encodes a PixelContainer to a 32bpp top-down BMP
  byte string; builds BITMAPFILEHEADER (14 bytes) and BITMAPINFOHEADER (40 bytes)
  using `pack('a2 V v v V', ...)` and `pack('V l< l< v v V V l< l< V V', ...)`
  respectively; writes pixel rows in BGRA order (R↔B swap)
- `decode_bmp($bytes)` — decodes a BMP byte string to a PixelContainer;
  validates magic bytes, info header size, bit depth (24 or 32), and compression;
  handles both top-down (negative biHeight) and bottom-up (positive biHeight)
  row ordering; computes row stride as `(W*bpp + 3) & ~3` for padding; swaps
  B↔R back to RGBA storage order
- Input validation: `EncodeError:` for non-container encode input;
  `BMP:` prefixed errors for all decode failures (bad magic, short file,
  unsupported bit depth, unsupported compression, file too short for pixel data)
- Test suite (`t/image_codec_bmp.t`) with 25+ subtests using `Test2::V0`,
  covering: mime_type, header field values, BGRA byte order, round-trip
  for 1×1 / 4×4 / arbitrary sizes, alpha preservation, bottom-up BMP,
  all error/invalid-input cases
- Knuth-style literate comments: BMP format table, BGRA explanation, row
  padding formula, top-down vs bottom-up explanation, pack format cheat-sheet
