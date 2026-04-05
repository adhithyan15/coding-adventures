# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of `CodingAdventures::ImageCodecPPM` (IC02)
- `mime_type()` — returns `'image/x-portable-pixmap'`
- `encode_ppm($container)` — encodes a PixelContainer to a P6 PPM binary
  string; writes `"P6\n<W> <H>\n255\n"` header then W×H×3 raw bytes in RGB
  order using `pack('CCC', ...)` per pixel; alpha channel is silently dropped
- `decode_ppm($bytes)` — decodes a P6 PPM binary string to a PixelContainer;
  uses an offset cursor with skip-whitespace-and-comments logic to parse
  whitespace-delimited header tokens without assuming fixed line structure;
  validates magic ('P6'), positive dimensions, and maxval (1..255);
  scales channel values via `int($v * 255 / maxval + 0.5)` when maxval != 255;
  sets alpha = 255 for all decoded pixels
- Input validation: `EncodeError:` for non-container encode input;
  `PPM:` prefixed errors for decode failures (bad magic, empty input, bad
  dimensions, unsupported maxval > 255, insufficient pixel data)
- Test suite (`t/image_codec_ppm.t`) with 22+ subtests using `Test2::V0`,
  covering: mime_type, header format checks, pixel data layout, round-trips
  (1×1, 3×2, fill_pixels, dimension preservation, min/max channel values),
  comment line skipping, maxval scaling, all error cases
- Knuth-style literate comments: P6 format description, alpha-drop policy,
  parsing strategy, maxval scaling formula, comment-skip algorithm
