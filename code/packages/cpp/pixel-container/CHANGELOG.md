# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `pixel-container` crate,
  in namespace `ca`: a flat, row-major RGBA8 `PixelContainer` plus the abstract
  `ImageCodec` interface.
- `PixelContainer` value type (deep-copy semantics, `operator==`): constructor,
  `from_data`, `pixel_at`, `set_pixel`, `fill`, `pixel_count`, `byte_count`.
- `from_data` throws `std::invalid_argument` on length mismatch; the constructor
  throws `std::length_error` on dimension overflow (the Rust crate panics).
- `ImageCodec` abstract base (`mime_type` / `encode` / `decode`) for user codecs.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) mirroring the Rust
  crate's own unit tests, including a StubCodec round trip.
