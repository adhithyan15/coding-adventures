# @coding-adventures/image-geometric-transforms

**IMG04** — Geometric transforms for `PixelContainer` images.

This package implements spatial image transforms: lossless pixel-exact
operations (flip, 90°/180° rotation, crop, pad) and continuous warps that
require resampling (scale, arbitrary rotation, affine, perspective/homography).

It sits directly above `@coding-adventures/pixel-container` (IC00) in the
image stack and depends on nothing else.

## How it fits in the stack

```
IC00 pixel-container           – RGBA8 pixel buffer
IMG01 image-codec-bmp          – encode/decode BMP
IMG02 image-codec-qoi          – encode/decode QOI
IMG03 image-point-ops          – per-pixel colour operations
IMG04 image-geometric-transforms  ← this package
```

## Installation

```bash
npm install @coding-adventures/image-geometric-transforms
```

## Quick start

```typescript
import {
  createPixelContainer,
  setPixel,
} from "@coding-adventures/pixel-container";
import {
  flipHorizontal,
  scale,
  rotate,
  affine,
  perspectiveWarp,
} from "@coding-adventures/image-geometric-transforms";

const src = createPixelContainer(200, 100);
// ... fill src with pixel data ...

// Lossless flip
const flipped = flipHorizontal(src);

// Scale to 400×200 using bilinear interpolation
const bigger = scale(src, 400, 200, "bilinear");

// Rotate 30° counter-clockwise, fitting the output to show all pixels
const rotated = rotate(src, Math.PI / 6, "bilinear", "fit");

// Affine shear: shift each row right by 0.3 * y
const sheared = affine(src, [[1, 0.3, 0], [0, 1, 0]], src.width + 30, src.height);

// Perspective warp (identity in this example)
const warped = perspectiveWarp(src, [[1,0,0],[0,1,0],[0,0,1]], src.width, src.height);
```

## API reference

### Types

| Type | Values | Description |
|------|--------|-------------|
| `Interpolation` | `'nearest' \| 'bilinear' \| 'bicubic'` | Sampling kernel |
| `RotateBounds` | `'fit' \| 'crop'` | Output sizing for arbitrary-angle rotation |
| `OutOfBounds` | `'zero' \| 'replicate' \| 'reflect' \| 'wrap'` | How to handle source coordinates outside the image |
| `Rgba8` | `[r, g, b, a]` | One pixel, each channel 0–255 |

### Lossless transforms (no sRGB conversion)

| Function | Signature | Notes |
|----------|-----------|-------|
| `flipHorizontal` | `(src) → PixelContainer` | Mirror left↔right |
| `flipVertical` | `(src) → PixelContainer` | Mirror top↔bottom |
| `rotate90CW` | `(src) → PixelContainer` | Swaps W and H |
| `rotate90CCW` | `(src) → PixelContainer` | Swaps W and H |
| `rotate180` | `(src) → PixelContainer` | Preserves W and H |
| `crop` | `(src, x0, y0, w, h) → PixelContainer` | Extract sub-region |
| `pad` | `(src, top, right, bottom, left, fill) → PixelContainer` | Add border |

### Continuous transforms (with interpolation)

| Function | Signature | Notes |
|----------|-----------|-------|
| `scale` | `(src, outW, outH, mode?) → PixelContainer` | Default mode `'bilinear'`, uses `'replicate'` OOB |
| `rotate` | `(src, radians, mode?, bounds?) → PixelContainer` | Default `'bilinear'`, `'fit'`; uses `'zero'` OOB |
| `affine` | `(src, matrix, outW, outH, mode?, oob?) → PixelContainer` | 2×3 matrix |
| `perspectiveWarp` | `(src, h, outW, outH, mode?, oob?) → PixelContainer` | 3×3 homography |
| `sample` | `(img, u, v, mode, oob) → Rgba8` | Low-level sampler |

## Design notes

### Inverse warp
All continuous transforms iterate over *output* pixels and look up the
corresponding *source* coordinate.  This "pull-based" approach guarantees every
output pixel is visited exactly once and avoids holes or write races.

### Pixel-centre model
Pixel (x, y) is treated as occupying the unit square centred at (x+0.5, y+0.5)
in continuous space.  The scale formula uses `(x' + 0.5) / sx - 0.5` to keep
the pixel grid aligned at both edges.

### Linear-light blending
Bilinear and bicubic interpolation decode sRGB bytes to linear-light floats,
blend there, then re-encode to sRGB.  Blending directly in gamma-compressed
space would produce colours that appear too dark.

### Catmull-Rom kernel
The bicubic sampler uses the Catmull-Rom (B=0, C=0.5) Mitchell–Netravali kernel.
It is interpolating (passes exactly through sample points), has compact support
of ±2 pixels, and is C1 continuous.  Negative lobes can produce values slightly
outside [0, 1]; these are clamped during sRGB re-encoding.

## Running tests

```bash
npm install
npx vitest run --coverage
```

## License

MIT
