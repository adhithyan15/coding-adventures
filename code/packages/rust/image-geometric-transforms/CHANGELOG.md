# Changelog — image-geometric-transforms

All notable changes to this crate are recorded here.
Dates in YYYY-MM-DD format.

---

## [0.1.0] — 2026-04-20

### Added

- Initial release implementing IMG04 (geometric transforms) over `PixelContainer`.

- **Lossless integer transforms** (raw byte shuffle, no colour-space conversion):
  - `flip_horizontal` — mirror the image left ↔ right within each row.
  - `flip_vertical` — mirror the image top ↔ bottom by swapping rows.
  - `rotate_90_cw` — rotate 90° clockwise; output dimensions are swapped (W'=H, H'=W).
  - `rotate_90_ccw` — rotate 90° counter-clockwise; output dimensions are swapped.
  - `rotate_180` — half-turn rotation; same dimensions as source.
  - `crop` — extract a rectangular sub-region; out-of-bounds region silently clamped.
  - `pad` — add a solid-colour border of specified widths on all four sides.

- **Continuous resampled transforms** (inverse-warp model, linear-light interpolation):
  - `scale` — resize to any output dimensions with pixel-centre correction.
  - `rotate` — arbitrary angle (radians) with `RotateBounds::Fit` or `RotateBounds::Crop`.
  - `affine` — 2×3 inverse-warp matrix; supports translation, rotation, scale, shear.
  - `perspective_warp` — 3×3 homogeneous inverse-warp matrix for full projective mapping.

- **Interpolation modes** (`Interpolation` enum):
  - `Nearest` — round to nearest integer; fast, exact byte copy.
  - `Bilinear` — 2×2 neighbourhood blend in linear f32; smooth.
  - `Bicubic` — 4×4 Catmull-Rom cubic blend in linear f32; sharp, minimal ringing.

- **Out-of-bounds policies** (`OutOfBounds` enum):
  - `Zero` — return transparent black `(0,0,0,0)` outside the image.
  - `Replicate` — clamp to the nearest edge pixel.
  - `Reflect` — mirror-reflect with period 2×dimension.
  - `Wrap` — modular tile wrap.

- **sRGB LUT**: lazy 256-entry `SRGB_TO_LINEAR` decode table (shared pattern with IMG03).

- **`Rgba8` type alias** (`(u8, u8, u8, u8)`).

- **Public sampling helpers**: `sample`, `sample_nn`, `sample_bilinear`, `sample_bicubic`
  (the individual samplers are internal but `sample` is the public dispatcher).

- **Catmull-Rom weight function** `catmull_rom(d)` — piecewise cubic; partition-of-unity
  verified in tests.

- **Unit tests** (35 total), covering:
  - `flip_horizontal` reverses pixels; double-flip is identity.
  - `flip_vertical` reverses rows; double-flip is identity.
  - `rotate_90_cw` + `rotate_90_ccw` are mutual inverses; dimensions swap correctly.
  - `rotate_180` applied twice is identity.
  - `rotate_90_cw` pixel position correctness.
  - `crop` extracts correct sub-region; clamps when request exceeds image bounds.
  - `pad` output dimensions; interior matches source; border matches fill colour.
  - `scale` up doubles dimensions; scale down halves dimensions.
  - `scale` with Replicate OOB does not panic at edges.
  - `OutOfBounds::Wrap` tiling via `affine` identity.
  - `rotate(0.0)` is approximately identity (±1 per channel).
  - `rotate` Fit canvas larger than Crop canvas.
  - `affine` identity matrix is identity; translation shifts correctly.
  - `perspective_warp` identity matrix is identity; uniform scale-in-w is identity.
  - Nearest-neighbour produces exact pixel values.
  - Bilinear midpoint blend of 2-pixel gradient is correct (within sRGB round-trip tolerance).
  - `resolve` unit tests for all four OOB modes.
  - `catmull_rom` value at 0 is 1; value at 1 is 0; partition-of-unity holds for all fx.
