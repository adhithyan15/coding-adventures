# Changelog — ImageGeometricTransforms (Swift)

## [0.1.0] — 2026-04-20

### Added

- Initial release implementing IMG04 (geometric / spatial transforms) over `PixelContainer`.

- **Lossless pixel-copy operations** (no interpolation, no colour-space conversion):
  - `flipHorizontal` — mirror left ↔ right.
  - `flipVertical` — mirror top ↔ bottom.
  - `rotate90CW` — 90° clockwise; swaps width and height.
  - `rotate90CCW` — 90° counter-clockwise; swaps width and height.
  - `rotate180` — 180° rotation; equivalent to flipH ∘ flipV.
  - `crop` — extract a rectangular sub-region.
  - `pad` — add constant-colour border rows/columns.

- **Continuous-coordinate resampling operations** (correct linear-light blending):
  - `scale` — resize to arbitrary (outW, outH) with pixel-centre mapping.
  - `rotate` — arbitrary-angle rotation with `.fit` or `.crop` canvas sizing.
  - `affine` — general 2×3 affine warp (rotation, scale, shear, translate).
  - `perspectiveWarp` — general 3×3 homogeneous perspective warp.

- **Interpolation modes** (`Interpolation` enum):
  - `.nearest` — round to nearest integer pixel.
  - `.bilinear` — 2×2 neighbourhood linear blend in linear light.
  - `.bicubic` — 4×4 Catmull-Rom cubic blend in linear light.

- **Out-of-bounds modes** (`OutOfBounds` enum):
  - `.zero` — transparent black.
  - `.replicate` — clamp to border pixel.
  - `.reflect` — mirror across border.
  - `.wrap` — periodic tiling.

- **Canvas sizing** (`RotateBounds` enum):
  - `.fit` — expand canvas to contain entire rotated source.
  - `.crop` — keep original canvas size, clip corners.

- Module-level 256-entry `srgbToLinear` decode LUT (built once at module load).
- Full XCTest suite with 28 tests covering every public function,
  round-trip identities, dimension checks, exact pixel values (lossless
  ops), near-identity checks ±2 per channel (resampling ops), and OOB
  smoke tests.
