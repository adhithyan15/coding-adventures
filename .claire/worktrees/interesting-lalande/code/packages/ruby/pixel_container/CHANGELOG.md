# Changelog — PixelContainer (Ruby)

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-05

### Added

- **`CodingAdventures::PixelContainer::Container`** — Struct holding `width`, `height`, and
  a binary `data` String. Helpers: `pixel_count`, `byte_count`, `to_s`.
- **`CodingAdventures::PixelContainer::ImageCodec`** — Marker module / interface for image
  format implementations (`mime_type`, `encode`, `decode`).
- **`PC.create(width, height)`** — Allocates a zeroed RGBA8 buffer; raises `ArgumentError`
  on non-positive dimensions.
- **`PC.pixel_at(container, x, y)`** — Returns `[r, g, b, a]` array; `[0,0,0,0]` for OOB.
- **`PC.set_pixel(container, x, y, r, g, b, a)`** — Writes one pixel; no-op for OOB coords;
  masks each channel with `& 0xFF`.
- **`PC.fill_pixels(container, r, g, b, a)`** — Fills entire buffer with one colour.
- 40 minitest tests covering create, struct helpers, pixel_at, set_pixel, fill_pixels,
  round-trips, and raw offset verification.

### Notes

- Uses `String#getbyte` / `String#setbyte` for O(1) byte-level pixel access.
- Buffer encoding is forced to `ASCII-8BIT` (BINARY) to prevent encoding errors.
