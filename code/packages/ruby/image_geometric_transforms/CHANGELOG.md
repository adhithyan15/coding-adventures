# Changelog — ImageGeometricTransforms (Ruby)

All notable changes are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning: [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-20

### Added

- **`ImageGeometricTransforms.flip_horizontal(src)`** — Mirror image left-to-right
  by remapping each pixel `I[x, y]` to `O[W-1-x, y]`.  Raw-byte copy; no sRGB
  conversion.

- **`ImageGeometricTransforms.flip_vertical(src)`** — Mirror image top-to-bottom.
  Raw-byte copy.

- **`ImageGeometricTransforms.rotate_90_cw(src)`** — Rotate 90° clockwise.  Output
  dimensions swap (W'=H, H'=W).  Backward mapping: `O[x',y'] = I[y', W-1-x']`.
  Raw-byte copy.

- **`ImageGeometricTransforms.rotate_90_ccw(src)`** — Rotate 90° counter-clockwise.
  Backward mapping: `O[x',y'] = I[H-1-y', x']`.  Raw-byte copy.

- **`ImageGeometricTransforms.rotate_180(src)`** — Rotate 180°.  Same dimensions.
  Backward mapping: `O[x,y] = I[W-1-x, H-1-y]`.  Raw-byte copy.

- **`ImageGeometricTransforms.crop(src, x0, y0, w, h)`** — Extract a `w×h`
  rectangle with top-left at `(x0, y0)`.  Pixels outside source are `[0,0,0,0]`.

- **`ImageGeometricTransforms.pad(src, top, right, bottom, left, fill:)`** — Add a
  border of configurable width.  Border pixels receive `fill` (default transparent
  black).  Output size: `W' = left+W+right`, `H' = top+H+bottom`.

- **`ImageGeometricTransforms.scale(src, out_w, out_h, mode:)`** — Resize using the
  pixel-centre model.  Default mode `:bilinear`.  OOB strategy `:replicate`.

- **`ImageGeometricTransforms.rotate(src, radians, mode:, bounds:)`** — Rotate
  counter-clockwise about image centre.
  - `bounds: :fit` (default) — canvas sized to `ceil(W|cos|+H|sin|)` ×
    `ceil(W|sin|+H|cos|)` so no pixels are clipped.
  - `bounds: :crop` — canvas matches input dimensions.
  - OOB strategy `:zero` (outside areas become transparent).

- **`ImageGeometricTransforms.affine(src, matrix, out_w, out_h, mode:, oob:)`** —
  Apply a 2×3 affine matrix.  Backward mapping solved via 2×2 inverse.  Handles
  singular matrices gracefully (transparent output).

- **`ImageGeometricTransforms.perspective_warp(src, h, out_w, out_h, mode:, oob:)`** —
  Apply a 3×3 projective homography.  Backward mapping computed via full 3×3
  matrix inverse (Cramer's rule).  Handles singular matrices and degenerate
  `w̃ ≈ 0` gracefully.

- **`SRGB_TO_LINEAR`** — 256-entry precomputed LUT for sRGB→linear decoding,
  frozen at module load.

- Private helpers: `decode(b)`, `encode(v)`, `resolve_coord(x, max, oob)`,
  `catmull_rom(d)`, `sample_nearest`, `sample_bilinear`, `sample_bicubic`,
  `sample` dispatch.

- Interpolation modes: `:nearest`, `:bilinear`, `:bicubic` (Catmull-Rom 4×4).

- Out-of-bounds modes: `:zero`, `:replicate`, `:reflect`, `:wrap`.

- 37 minitest tests covering: flip identity round-trips, rotate-90 CW/CCW
  round-trip, rotate-180 twice identity, pixel mapping correctness, crop
  dimensions and OOB behaviour, pad dimensions and fill, scale up/down
  dimensions, scale solid colour fidelity, rotate zero identity, rotate fit
  canvas enlargement, rotate crop size preservation, affine identity and
  translation, perspective-warp identity, nearest-neighbour exact pixel,
  bilinear midpoint gradient, all four OOB modes smoke-tested, bicubic
  smoke test.

### Dependencies

- `coding-adventures-pixel-container` (loaded via `LOAD_PATH`)

### Notes

- All continuous transforms use backward (inverse) mapping to avoid holes.
- Bilinear and bicubic blending is performed in linear light; alpha is always
  treated as linear.
- The Catmull-Rom kernel is piecewise cubic, yielding C1-continuous results.
- Ruby's `%` operator always returns a non-negative value for a positive
  divisor, making `:wrap` OOB trivial.
