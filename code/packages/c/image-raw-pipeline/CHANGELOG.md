# Changelog

All notable changes to the C `image-raw-pipeline` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `image-raw-pipeline` crate — the shared
  four-stage RAW colour-development pipeline (normalize → white balance →
  colour matrix → sRGB gamma).
- `irp_srgb_gamma` / `irp_srgb_decode` (IEC 61966-2-1 EOTF and inverse),
  `irp_mat3x3_mul`, `irp_invert_3x3` (Cramer's rule), and
  `irp_apply_color_pipeline`.
- The sRGB gamma's fractional `pow` is computed without `<math.h>` from a
  from-scratch `exp`/`ln`, matching the Rust f64 `powf` to ~1e-12 relative.
- `IrpStatus` + out-parameter API in place of the Rust `Option` / `Vec`; the
  developed-pixel allocation is guarded against `size_t` overflow.
- 286 checks mirroring the Rust crate's own unit tests (fixed points,
  monotonicity, round-trips, matrix inversion identities, and exact 8-bit
  pipeline outputs), run under every available C compiler via the shared
  `iso-harness`; the suite also passes clean under AddressSanitizer +
  UndefinedBehaviorSanitizer.
