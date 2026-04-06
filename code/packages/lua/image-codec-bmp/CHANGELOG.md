# Changelog — coding-adventures-image-codec-bmp (Lua)

All notable changes to the Lua `image-codec-bmp` package are documented here.

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of `coding_adventures.image_codec_bmp` (IC01).
- `encode_bmp(c)` — serialises a PixelContainer into a 32-bit RGBA BMP binary
  string with negative biHeight (top-down), BI_RGB compression, and BGRA pixel
  order. Uses `string.pack` with format `<c2I4I2I2I4` for the file header and
  `<I4i4i4I2I2I4I4i4i4I4I4` for the DIB header.
- `decode_bmp(data)` — parses a 32-bit BMP binary string back into a
  PixelContainer. Validates 'BM' magic, 32 bpp bit depth, BI_RGB or
  BI_BITFIELDS compression, and data length. Handles both top-down (negative
  biHeight) and bottom-up (positive biHeight) row ordering.
- `codec` table conforming to the ImageCodec interface: `{ mime_type, encode, decode }`.
- `VERSION = "0.1.0"` and `mime_type = "image/bmp"` constants.
- Knuth-style literate comments explaining the BMP file structure, BGRA byte
  order, negative biHeight convention, row padding (not needed for 32 bpp),
  and `string.pack` format strings.
- 30 busted unit tests covering header structure, pixel byte order, round-trips,
  alpha preservation, dimension correctness, and error handling.
