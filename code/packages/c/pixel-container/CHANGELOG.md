# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the `PixelContainer` type from the Rust
  `pixel-container` crate: a flat, row-major RGBA8 pixel buffer.
- `pixel_new` / `pixel_from_data` / `pixel_clone` / `pixel_free`;
  `pixel_at` (zeros out of bounds) / `pixel_set` (no-op out of bounds) /
  `pixel_fill`; `pixel_width` / `height` / `count` / `byte_count` / `data` /
  `pixel_equals`.
- Overflow-guarded `width*height*4` sizing (checked `calloc`); dimension
  overflow and `from_data` length mismatch return NULL instead of aborting
  (the Rust crate panics).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) mirroring the Rust
  crate's own unit tests (offsets, bounds, fill, clone independence, equality,
  overflow).
