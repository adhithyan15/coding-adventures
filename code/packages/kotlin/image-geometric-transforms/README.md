# image-geometric-transforms (Kotlin)

**Spec:** IMG04 — Geometric transforms on `PixelContainer`.

Flips, 90° rotations, crop, scale, arbitrary-angle rotate, translate, affine,
and perspective warp — every operation that moves pixels rather than
recolouring them.

## What's implemented

### Lossless, byte-level (no interpolation)

- `flipHorizontal` / `flipVertical`
- `rotate90CW` / `rotate90CCW` / `rotate180`
- `crop(x, y, w, h)` — OOB areas fill transparent black

### Continuous (interpolated)

- `scale(w, h, interp)`
- `rotate(radians, interp, bounds)` — `bounds` is `FIT` or `CROP`
- `translate(tx, ty, interp)`
- `affine(m: 2x3, interp, oob)` — `m` is forward; inverted internally
- `perspectiveWarp(h: 3x3, interp, oob)` — `h` is the inverse map

### Helpers

- `invert3x3(m)` — for callers composing homographies by hand

## Interpolation kernels

`NEAREST`, `BILINEAR`, `BICUBIC` (Catmull-Rom, B=0 C=0.5).
Bilinear/bicubic blend in **linear light** — sRGB bytes decoded, blended,
re-encoded. Nearest neighbour does not blend, so it passes raw bytes.

## OOB policies

`ZERO`, `REPLICATE`, `REFLECT`, `WRAP`. Applied whenever a sample falls
outside `[0, w) × [0, h)`.

## Conventions

- Inverse-warp / pull-based: every output pixel samples once; no holes.
- Pixel-centre model: pixel `(x, y)` occupies `[x, x+1] × [y, y+1]`.
  Resampling uses `u = (x' + 0.5) / sx - 0.5`.

## Dependencies

- `pixel-container` (IC00)

## Tests

```
gradle test
```
