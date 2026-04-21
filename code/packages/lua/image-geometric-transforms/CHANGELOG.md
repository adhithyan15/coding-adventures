# Changelog — coding-adventures-image-geometric-transforms (Lua)

## [0.1.0] — 2026-04-20

### Added

- Initial release implementing IMG04 (geometric transforms) over `pixel_container`.
- **Lossless operations** (exact byte copy, no colour arithmetic):
  - `flip_horizontal` — mirror each row left-to-right; double-apply is identity.
  - `flip_vertical` — mirror each column top-to-bottom; double-apply is identity.
  - `rotate_90_cw` — rotate 90° clockwise; four applications are identity.
  - `rotate_90_ccw` — rotate 90° counter-clockwise; CW then CCW is identity.
  - `rotate_180` — rotate 180°; double-apply is identity.
  - `crop` — extract a rectangular sub-image (0-indexed coords); OOB pads with transparent black.
  - `pad` — add a constant-colour border; configurable per-side thickness and fill colour.
- **Continuous operations** (inverse-mapped, sampled):
  - `scale` — resize to arbitrary dimensions using nearest/bilinear/bicubic filter with replicate OOB.
  - `rotate` — arbitrary-angle rotation with fit/crop canvas modes and zero OOB fill.
  - `affine` — 2×3 forward affine matrix warp (auto-inverted); errors on singular matrix.
  - `perspective_warp` — 3×3 homography warp (auto-inverted); errors on singular matrix.
- **Interpolation filters**:
  - `nearest` — snap to nearest pixel (no colour arithmetic).
  - `bilinear` — 2×2 weighted average in linear light.
  - `bicubic` — 4×4 Catmull-Rom in linear light.
- **Out-of-bounds modes**: `zero`, `replicate`, `reflect`, `wrap`.
- Module-level 256-entry `SRGB_TO_LINEAR` LUT (built once at load).
- 41 Busted unit tests covering all operations, interpolation modes, OOB modes,
  dimension checks, identity invariants, and pixel value correctness.
