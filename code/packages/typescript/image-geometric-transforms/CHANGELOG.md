# Changelog

All notable changes to `@coding-adventures/image-geometric-transforms` are
documented here.  This project adheres to
[Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-20

### Added

- **`flipHorizontal(src)`** — Mirror image left↔right (lossless, raw bytes).
- **`flipVertical(src)`** — Mirror image top↔bottom (lossless, raw bytes).
- **`rotate90CW(src)`** — Rotate 90° clockwise; swaps W and H (lossless).
- **`rotate90CCW(src)`** — Rotate 90° counter-clockwise; swaps W and H (lossless).
- **`rotate180(src)`** — Rotate 180°; preserves dimensions (lossless).
- **`crop(src, x0, y0, w, h)`** — Extract a rectangular sub-region.
- **`pad(src, top, right, bottom, left, fill)`** — Add a solid-colour border.
- **`scale(src, outW, outH, mode?)`** — Rescale with nearest / bilinear / bicubic
  interpolation using pixel-centre model and replicate OOB.
- **`rotate(src, radians, mode?, bounds?)`** — Arbitrary-angle rotation with
  inverse warp; `'fit'` or `'crop'` output sizing.
- **`affine(src, matrix, outW, outH, mode?, oob?)`** — 2×3 affine transform.
- **`perspectiveWarp(src, h, outW, outH, mode?, oob?)`** — 3×3 homography /
  perspective warp.
- **`sample(img, u, v, mode, oob)`** — Low-level sampler (nearest, bilinear,
  bicubic) with full out-of-bounds handling (zero, replicate, reflect, wrap).
- **sRGB LUT** — 256-entry Float32Array for O(1) sRGB→linear decoding, built
  once at module load.
- **Types** — `Interpolation`, `RotateBounds`, `OutOfBounds`, `Rgba8`.
- Full test suite (≥ 25 tests) covering all public functions and all OOB modes.
- `vitest` coverage reporting with 80% line/function/statement thresholds.
