# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-08-21

### Changed

- Carry the local `image-codec-png`, ZIP, and LZSS replacements required by
  the portable paint PNG adapter so downstream builds use the repository IC18
  codec rather than Go's standard-library decoder.

## [0.1.0] - 2026-04-13

### Added

- High-level 1D barcode orchestrator for Go covering scene construction, pixel rendering, and PNG encoding.
- Native backend selection for Go: Metal via Rust C ABI on macOS arm64, direct GDI on Windows, and raster fallback elsewhere.
