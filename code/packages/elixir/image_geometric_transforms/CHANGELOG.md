# Changelog

All notable changes to `image_geometric_transforms` will be documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — 2026-04-20

### Added

**Integer (lossless) transforms** — copy raw RGBA8 bytes without arithmetic or
colorspace conversion:

- `flip_horizontal/1` — mirror left↔right; two applications are identity
- `flip_vertical/1` — mirror top↔bottom; two applications are identity
- `rotate_90_cw/1` — 90° clockwise rotation; swaps W and H; four applications
  are identity
- `rotate_90_ccw/1` — 90° counter-clockwise rotation; swaps W and H
- `rotate_180/1` — 180° rotation; equivalent to flip_h ∘ flip_v; two
  applications are identity
- `crop/5` — extract a rectangular sub-region at `(x0, y0)` with size `(w, h)`
- `pad/6` — extend the canvas with a configurable fill colour (default:
  transparent black `{0, 0, 0, 0}`)

**Continuous transforms** — linear-light bilinear/bicubic interpolation with
configurable out-of-bounds handling:

- `scale/4` — resize to `(out_w, out_h)` using the pixel-centre model;
  supports `:nearest`, `:bilinear` (default), `:bicubic`
- `rotate/4` — arbitrary angle rotation (radians, CCW positive); `:fit` or
  `:crop` bounds; `:zero` OOB background
- `affine/6` — apply a pre-inverted 2×3 affine matrix; configurable OOB mode
- `perspective_warp/6` — apply a pre-inverted 3×3 homography; homogeneous
  division with degenerate-w guard

**Sampling kernels (private)**:

- Nearest-neighbour — no blending; O(1) per output pixel
- Bilinear — 2×2 linear-light blend; O(1) per output pixel
- Bicubic (Catmull-Rom) — 4×4 separable kernel; O(1) per output pixel

**Out-of-bounds modes (private)**:

- `:zero` — transparent black for out-of-range coordinates
- `:replicate` — clamp to nearest edge pixel
- `:reflect` — mirror via period = 2 * dimension
- `:wrap` — modular (tiling)

**sRGB LUT** — 256-entry compile-time decode table (same pattern as
`image_point_ops`); `decode/1` and `encode/1` helpers shared with sampling.

**Test suite** — 50 ExUnit tests with `async: true`, covering:

- All seven lossless transforms including double-flip identity, rotate round-trips,
  crop value extraction, pad fill + interior preservation
- Scale output dimensions and solid-colour preservation
- Rotate(0) ≈ identity, `:fit` vs `:crop` bounds, rotate(2π) ≈ identity
- Affine identity, affine translation
- Perspective identity, perspective degenerate w=0 path
- All four OOB modes via affine with controlled out-of-range coordinates
- Nearest exact read, bilinear midpoint linear-light blend
- Bicubic solid-colour preservation, bicubic OOB nil path, bicubic affine identity
- sRGB encode/decode round-trip

Coverage: **99.47%** (threshold: 90%).
