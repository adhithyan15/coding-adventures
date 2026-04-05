# Changelog — coding-adventures-image-codec-ppm (Lua)

All notable changes to the Lua `image-codec-ppm` package are documented here.

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of `coding_adventures.image_codec_ppm` (IC02).
- `encode_ppm(c)` — serialises a PixelContainer to a P6 PPM binary string.
  Alpha is silently dropped; output is `"P6\n<w> <h>\n255\n"` followed by
  raw RGB bytes (3 bytes per pixel, no padding).
- `decode_ppm(data)` — parses a P6 PPM binary string into a PixelContainer.
  Validates magic ('P6'), maxval (255), and pixel data length. Skips '#'
  comment lines during header parsing. Sets alpha = 255 for every pixel.
- Internal `next_token` helper that correctly skips whitespace and comment
  lines when reading the header.
- `codec` table conforming to the ImageCodec interface.
- `VERSION = "0.1.0"` and `mime_type = "image/x-portable-pixmap"` constants.
- Knuth-style literate comments explaining the PPM format, P3 vs P6 variants,
  the comment-skipping header parser, and the alpha = 255 decode convention.
- 28 busted unit tests covering header structure, pixel byte order, alpha
  handling, comment line tolerance, round-trips, and error validation.
