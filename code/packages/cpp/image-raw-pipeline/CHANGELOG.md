# Changelog

All notable changes to the C++ `image-raw-pipeline` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `image-raw-pipeline`
  crate (namespace `ca::image_raw_pipeline`) — the shared four-stage RAW
  colour-development pipeline (normalize → white balance → colour matrix →
  sRGB gamma).
- `srgb_gamma` / `srgb_decode` (IEC 61966-2-1 EOTF and inverse), `mat3x3_mul`,
  `invert_3x3` returning `std::optional<Mat3>`, and `apply_color_pipeline`
  returning `std::vector<Rgb8>`; `Mat3`/`Vec3` mirror the Rust array types.
- The sRGB gamma's fractional `pow` is computed without `<cmath>` from a
  from-scratch `exp`/`ln`, matching the Rust f64 `powf` to ~1e-12 relative.
- 273 checks mirroring the Rust crate's own unit tests, run under every
  available C++ compiler via the shared `iso-harness`.
