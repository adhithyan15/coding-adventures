# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-05

### Added

- Initial implementation of `CodingAdventures::PixelContainer` (IC00)
- `new($width, $height)` — allocates W×H RGBA8 pixel buffer as a flat Perl
  byte string; validates positive integer dimensions; dies `InvalidInput` on bad args
- `width()` / `height()` — accessors returning image dimensions
- `data()` — returns scalar ref to the internal byte buffer for zero-copy access
  by codec modules
- `pixel_at($x, $y)` — decodes four bytes at offset `(y*W+x)*4` via
  `unpack('CCCC', ...)`, returning `(0,0,0,0)` for out-of-bounds coordinates
- `set_pixel($x, $y, $r, $g, $b, $a)` — encodes four bytes via `pack('CCCC', ...)`
  and splices them into the buffer with `substr` lvalue; silent no-op out-of-bounds
- `fill_pixels($r, $g, $b, $a)` — fills the entire buffer using Perl `x`
  repetition operator for O(1) Perl-level work
- `CodingAdventures::ImageCodec` — interface/documentation module listing the
  `mime_type()`, `encode($container)`, `decode($bytes)` contract for all codecs
- Test suite (`t/pixel_container.t`) with 30+ assertions using `Test2::V0`,
  covering: construction, invalid dims, initial zero state, accessor values,
  buffer size, pixel round-trips, multiple pixels, bottom-right pixel, max
  values, overwrite, OOB reads/writes, fill, 1×1 edge case
- Knuth-style literate inline comments throughout — byte offset formula,
  RGBA layout table, pack/unpack usage explanation, fill_pixels efficiency note
