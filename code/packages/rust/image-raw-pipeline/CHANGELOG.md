# Changelog — image-raw-pipeline

---

## [0.1.0] — 2026-06-13

### Added

- **`srgb_gamma(linear: f64) -> f64`** — IEC 61966-2-1 EOTF. Linear segment
  for L ≤ 0.0031308; power segment (1.055 × L^(1/2.4) − 0.055) otherwise.

- **`srgb_decode(encoded: f64) -> f64`** — Inverse EOTF. Linear segment for
  V ≤ 0.04045; power segment otherwise. Satisfies `srgb_decode(srgb_gamma(x)) ≈ x`.

- **`mat3x3_mul(m: &[[f64;3];3], v: [f64;3]) -> [f64;3]`** — 3×3 ×
  column-vector multiply, stack-allocated, called once per pixel in the RAW
  pipeline without any heap allocation.

- **`invert_3x3(m: &[[f64;3];3]) -> Option<[[f64;3];3]>`** — Analytic 3×3
  inversion via Cramer's rule. Returns `None` for singular matrices
  (`|det| < 1e-12`). Used by `image-codec-dng` for ForwardMatrix inversion.

- **`apply_color_pipeline`** — Full four-stage RAW development pipeline:
  black-level subtraction → normalisation → white balance → colour matrix →
  sRGB gamma. Replaces identical implementations in `image-codec-tiff`,
  `image-codec-raf`, and `image-codec-rw2`.

- **43 tests** (40 unit + 3 doc-tests): sRGB gamma at boundary/midpoint/
  endpoints; round-trip encode↔decode; mat3x3_mul identity/swap/scaling;
  invert_3x3 identity/diagonal/self-inverse/singular/M×inv(M)=I; pipeline
  identity/black-level/white-level/WB-saturation/channel-swap/multi-pixel.

[0.1.0]: https://github.com/adhithyan15/coding-adventures/tree/feat/img07-image-raw-pipeline
