# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-01

### Added

- `PixelBuffer` struct (RGBA8, row-major, top-left origin)
- `PixelEncoder` trait for pluggable image format encoders
- Helper methods: `new`, `from_data`, `pixel_at`, `set_pixel`, `pixel_count`, `byte_count`
