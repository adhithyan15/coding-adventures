# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-11

### Added

- Initial implementation, closing `VIS00-vision-roadmap.md`'s L2 gap:
  producing the 3x3 homogeneous transform matrix
  `image-geometric-transforms::perspective_warp` already applies but
  nothing in this workspace previously produced from real point
  measurements.
- `estimate_affine(from, to) -> Result<[[f64; 3]; 3], EstimationError>`
  — exactly 3 correspondences, two independent 3x3 `matrix::solve`
  calls sharing the same matrix (one for each row of the affine map).
- `estimate_homography(from, to) -> Result<[[f64; 3]; 3], EstimationError>`
  — exactly 4 correspondences, standard DLT construction, one 8x8
  `matrix::solve` call.
- `EstimationError::{WrongPointCount, Degenerate}` — never panics on
  any input shape; degenerate (collinear/coincident) point
  configurations are a documented `Err`, not a wrong-but-plausible
  matrix.
- 12 unit tests + 1 doc-test: known-transform round trips (identity,
  pure translation, rotation+scale+translation for affine; identity
  and a genuine perspective term for homography) verified against a
  held-out point not used in the estimate, degenerate-input rejection
  (collinear and coincident points), wrong-point-count rejection
  (including a from/to length mismatch), a cross-validation proving
  `estimate_homography` recovers the same transform as
  `estimate_affine` when the true transform has no perspective
  component, and empty-slice input proven not to panic.

### Out of scope (documented, not silently dropped)

- Fitting from more than the minimum point count (least-squares).
- RANSAC or other outlier-robust estimation.
- Enforcing the `from`/`to` direction convention at the type level.
