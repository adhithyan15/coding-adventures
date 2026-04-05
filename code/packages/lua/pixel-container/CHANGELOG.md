# Changelog — coding-adventures-pixel-container (Lua)

All notable changes to the Lua `pixel-container` package are documented here.

## [0.1.0] — 2026-04-05

### Added

- Initial implementation of `coding_adventures.pixel_container` (IC00).
- `new(width, height)` — allocate a blank RGBA8 container; validates that width
  and height are positive integers.
- `pixel_at(c, x, y)` — read RGBA tuple at 0-indexed (x, y); returns 0,0,0,0
  for out-of-bounds coordinates.
- `set_pixel(c, x, y, r, g, b, a)` — write RGBA tuple; silent no-op for
  out-of-bounds coordinates.
- `fill_pixels(c, r, g, b, a)` — O(width*height) bulk fill of every pixel.
- `clone(c)` — deep copy returning a new container with its own `data` table.
- `equals(a, b)` — pixel-exact comparison (checks dimensions then all bytes).
- `VERSION = "0.1.0"` constant.
- Knuth-style literate comments explaining row-major layout, 0-indexed
  coordinates, 1-based Lua table indexing, alpha conventions, and the
  ImageCodec interface pattern.
- 35 busted unit tests covering constructor validation, read/write round-trips,
  out-of-bounds safety, fill, clone aliasing, and equality comparison.
