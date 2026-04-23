# Changelog — image-geometric-transforms

All notable changes to this package are documented here.  The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and the package
version follows [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-20

### Added

- **Package scaffold**: `go.mod`, `BUILD`, `README.md`, `CHANGELOG.md`.

- **Type definitions**:
  - `Interpolation` enum (`Nearest`, `Bilinear`, `Bicubic`) selecting the
    resampling filter used by continuous transforms.
  - `RotateBounds` enum (`Fit`, `Crop`) controlling the output canvas size of
    `Rotate`.
  - `OutOfBounds` enum (`Zero`, `Replicate`, `Reflect`, `Wrap`) mirroring the
    four standard GPU texture-wrapping modes.
  - `Rgba8` convenience struct for fill colours in `Pad`.

- **sRGB / linear-light infrastructure**:
  - `var srgbToLinear [256]float64` — 256-entry decode LUT initialised once
    via `init()`.
  - `func encode(v float64) byte` — sRGB re-encoding with clamping and rounding.

- **Out-of-bounds resolver**:
  - `func resolveCoord(x, max int, oob OutOfBounds) (int, bool)` — maps an
    arbitrary integer coordinate into `[0, max-1]` according to the chosen
    wrapping policy; returns `(0, false)` for the `Zero` policy to let callers
    short-circuit to transparent black.

- **Catmull-Rom kernel**:
  - `func catmullRom(d float64) float64` — Keys (1983) cubic spline with
    α = 0.5, providing C¹ continuity for smooth bicubic reconstruction.

- **Sampling functions**:
  - `func sampleNN` — nearest-neighbour, O(1) per output pixel.
  - `func sampleBilinear` — bilinear blend of 4 neighbours in linear-light
    space.
  - `func sampleBicubic` — separable Catmull-Rom blend of a 4×4 neighbourhood
    in linear-light space.
  - `func Sample(img, u, v, mode, oob)` — public dispatcher.

- **Lossless geometric transforms** (byte-copy, no colour-space conversion):
  - `FlipHorizontal` — mirror left-to-right.
  - `FlipVertical` — mirror top-to-bottom.
  - `Rotate90CW` — 90° clockwise rotation; dimensions swap.  Output pixel
    `(x', y')` reads source `(col=y', row=H-1-x')` where `H = src.Height`.
    Note: the original spec listed the formula as `I[y'][W-1-x']`; the correct
    derivation uses `H-1-x'` (src height), not `W-1-x'` (src width), to
    properly invert the CW rotation.
  - `Rotate90CCW` — 90° counter-clockwise rotation; dimensions swap.  Output
    pixel `(x', y')` reads source `(col=W-1-y', row=x')` where `W = src.Width`.
  - `Rotate180` — 180° rotation; dimensions preserved.
  - `Crop(src, x, y, w, h)` — rectangular sub-image extraction; out-of-bounds
    area filled with transparent black.
  - `Pad(src, top, right, bottom, left, fill)` — adds a border of fill pixels.

- **Continuous geometric transforms** (inverse-warp, linear-light sampling):
  - `Scale(src, outW, outH, mode)` — resize to any dimensions using the
    pixel-centre model (`u = (x+0.5)*sx - 0.5`); OOB = Replicate.
  - `Rotate(src, radians, mode, bounds)` — arbitrary-angle rotation with
    inverse-warp formula; Fit or Crop output canvas; OOB = Zero.
  - `Affine(src, matrix, outW, outH, mode, oob)` — arbitrary 2×3 affine
    transform; matrix maps output → source directly (no inversion needed).
  - `PerspectiveWarp(src, h, outW, outH, mode, oob)` — projective homography
    via homogeneous division (`u = uh/w`, `v = vh/w`); horizon guard for
    `w ≈ 0`.

- **Unit test suite** (`image_geometric_transforms_test.go`):
  - 30 test functions covering lossless round-trips, dimension checks, pixel
    position correctness, sampling kernel properties, OOB policies, and
    approximate-identity checks for continuous transforms.
