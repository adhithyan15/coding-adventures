# Changelog

All notable changes to `coding-adventures-image_geometric_transforms` are documented here.

This file follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
the project uses [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Fixed

- Added `BUILD_windows`. The Unix `BUILD` calls `.venv/bin/python`, which does not
  exist on Windows (the interpreter is `.venv\Scripts\python.exe`), so the package
  failed in the first Windows full build with `'.venv' is not recognized as an
  internal or external command`. The new file runs the same steps with Windows
  paths; no source change.

## [0.1.0] — 2026-04-20

### Added

- **`flip_horizontal`** — Mirror image left-to-right by reversing pixel order
  within each row.  Pure byte copy; no sRGB conversion needed.

- **`flip_vertical`** — Mirror image top-to-bottom by reversing row order.
  Copies full rows at once for cache efficiency.

- **`rotate_90_cw`** — 90° clockwise rotation.  Output dimensions are the
  transpose (H×W becomes W×H).  Mapping: `out[x'][y'] = src[y'][W-1-x']`.

- **`rotate_90_ccw`** — 90° counter-clockwise rotation.  Round-trips exactly
  with `rotate_90_cw`.  Mapping: `out[x'][y'] = src[H-1-y'][x']`.

- **`rotate_180`** — 180° rotation.  Equivalent to horizontal followed by
  vertical flip.  Mapping: `out[x'][y'] = src[W-1-x'][H-1-y']`.

- **`crop`** — Extract a rectangular sub-region at `(x0, y0)` with dimensions
  `(w, h)`.  Out-of-boundary coordinates fill with transparent black via
  `pixel_at`'s built-in OOB guard.

- **`pad`** — Add a border of configurable RGBA fill colour.  Supports
  independent top/right/bottom/left widths.

- **`scale`** — Resize to any `(out_w, out_h)` using the pixel-centre model
  `u = (x'+0.5)*W/W' - 0.5`.  Supports NEAREST, BILINEAR, BICUBIC
  interpolation.  Out-of-bounds policy: REPLICATE.

- **`rotate`** — Rotate by arbitrary angle in radians using inverse warp.
  Supports `RotateBounds.FIT` (expand canvas) and `RotateBounds.CROP` (keep
  size).  Out-of-bounds policy: ZERO (transparent corners).

- **`affine`** — Apply a 2×3 inverse-warp affine matrix.  Supports configurable
  `Interpolation` and `OutOfBounds` policies.

- **`perspective_warp`** — Apply a 3×3 homographic transform.  Performs the
  homogeneous divide `u = (H·[x',y',1])[0] / (H·[x',y',1])[2]` per pixel.
  Supports configurable interpolation and OOB.

- **`Interpolation` enum** — NEAREST, BILINEAR, BICUBIC.  Bicubic uses the
  Catmull-Rom (α=-0.5) cubic kernel over a 4×4 neighbourhood.

- **`RotateBounds` enum** — FIT (bounding-box expand), CROP (same size).

- **`OutOfBounds` enum** — ZERO, REPLICATE, REFLECT, WRAP with unified
  `_resolve()` helper.

- **`_SRGB_TO_LINEAR` LUT** — 256-entry precomputed sRGB-to-linear table
  shared with bilinear and bicubic samplers for colour-correct blending.

- **`_catmull_rom(d)`** — Catmull-Rom kernel weight function for bicubic
  interpolation.

- **Test suite** — 30 pytest tests covering lossless round-trips, dimension
  contracts, pixel-mapping correctness, all interpolation modes, all OOB
  modes, and edge cases.  Coverage exceeds 80%.

[0.1.0]: https://github.com/adhithyan/coding-adventures/releases/tag/image_geometric_transforms-v0.1.0
