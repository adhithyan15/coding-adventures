# Changelog — coding-adventures-image-codec-qoi (Lua)

All notable changes to the Lua `image-codec-qoi` package are documented here.

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of `coding_adventures.image_codec_qoi` (IC03).
- `encode_qoi(c)` — encodes a PixelContainer using all six QOI operations
  (RUN, INDEX, DIFF, LUMA, RGB, RGBA) with correct priority ordering.
  Uses `string.pack(">c4I4I4BB", ...)` for the 14-byte big-endian header.
  Flushes run buffer before emitting a non-run op.
- `decode_qoi(data)` — decodes a QOI binary string using the same state
  (previous pixel starts at (0,0,0,255); 64-slot seen table all zeros).
  Uses Lua 5.4 `goto continue` to handle RUN without double-writing.
  Validates "qoif" magic, channels (3 or 4), and minimum data length.
- Internal `hash_pixel(r, g, b, a)` — `(r*3 + g*5 + b*7 + a*11) % 64`.
- Internal `wrap_delta(cur, prev)` — signed 8-bit delta with `(cur-prev) & 0xFF`
  then subtract 256 if >= 128; documented with examples.
- `codec` table conforming to the ImageCodec interface.
- `VERSION = "0.1.0"` and `mime_type = "image/qoi"` constants.
- Knuth-style literate comments explaining every QOI operation, the hash
  formula, the signed-delta wrapping technique, the run-length limit of 62,
  and the 8-byte end-of-stream sentinel.
- 32 busted unit tests covering header bytes, end marker, all six ops
  individually, round-trips (solid, checkerboard, gradient, noisy, alpha-varying
  images), and error validation.
