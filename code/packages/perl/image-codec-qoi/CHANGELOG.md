# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of `CodingAdventures::ImageCodecQOI` (IC03)
- `mime_type()` — returns `'image/qoi'`
- `encode_qoi($container)` — encodes a PixelContainer as a QOI binary string;
  implements all 6 chunk types: QOI_OP_RUN, QOI_OP_INDEX, QOI_OP_DIFF,
  QOI_OP_LUMA, QOI_OP_RGB, QOI_OP_RGBA; uses 64-entry seen-pixels hash table
  with `(R*3 + G*5 + B*7 + A*11) % 64`; flushes run at max length 62;
  writes 14-byte big-endian header and 8-byte end marker
- `decode_qoi($bytes)` — decodes a QOI binary string to a PixelContainer;
  validates magic, reads big-endian header, dispatches on top-2 bits of each
  chunk byte; applies DIFF/LUMA deltas with modulo-256 wrap-around via `& 0xFF`;
  reconstructs seen-pixels table as pixels are decoded; verifies end marker
- `_hash_pixel($r,$g,$b,$a)` — internal hash function for the seen table
- `_signed_delta($new,$prev)` — wrap-around signed byte delta helper
- Chunk constants: `QOI_OP_RGB` (0xFE), `QOI_OP_RGBA` (0xFF),
  `QOI_OP_INDEX` (0x00), `QOI_OP_DIFF` (0x40), `QOI_OP_LUMA` (0x80),
  `QOI_OP_RUN` (0xC0)
- Input validation: `EncodeError:` for non-container input;
  `QOI:` prefixed errors for decode failures (bad magic, short file,
  unexpected end of data mid-chunk, corrupt end marker, undef input)
- Test suite (`t/image_codec_qoi.t`) with 25+ subtests using `Test2::V0`,
  covering: mime_type, header fields, end marker, run encoding compactness,
  run length cap, round-trips (1×1, checkerboard, gradient, varying alpha,
  all-zeros, all-255, large mixed, single-row long image), RGBA op path,
  all error cases
- Knuth-style literate comments: QOI file structure table, all 6 chunk type
  descriptions, signed delta arithmetic, DIFF/LUMA bias formulas, run
  length storage convention, seen-pixels hash motivation
