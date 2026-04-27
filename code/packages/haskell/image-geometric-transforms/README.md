# image-geometric-transforms (Haskell)

IMG04 — Geometric transforms on a `PixelContainer`.

## Overview

All transforms share one core strategy: /inverse warping/.  For each output
pixel `(x', y')` we compute the non-integer source coordinate `(u, v)` that
maps to it and sample there.  This guarantees every output pixel is written
exactly once and reduces "how do I transform an image?" to "how do I sample
at a fractional coordinate?".

### Lossless transforms

- `flipHorizontal`, `flipVertical`
- `rotate90CW`, `rotate90CCW`, `rotate180`
- `crop`

### Continuous transforms (take `Interpolation` and `OutOfBounds`)

- `scale`
- `rotate` (arbitrary degrees, with `RotateBounds = Fit | Crop`)
- `translate`
- `affine` (2x3 forward matrix)
- `perspectiveWarp` (3x3 forward homography)

### Interpolation

- `Nearest`  — snap to closest pixel.
- `Bilinear` — 4-neighbour blend in linear light.
- `Bicubic`  — 16-neighbour Catmull-Rom in linear light.

### Out-of-bounds policies

- `Zero`, `Replicate`, `Reflect`, `Wrap`.

## Usage

```haskell
import PixelContainer
import ImageGeometricTransforms

main :: IO ()
main = do
    let src = fillPixels (createPixelContainer 100 100) 128 128 128 255
        out = rotate src 45 Fit Bilinear Replicate
    print (pcWidth out, pcHeight out)
```

## Dependencies

- `pixel-container` (IC00)

(The sRGB decode/encode helpers are intentionally duplicated in this package
so it does not take an unnecessary dependency on `image-point-ops`.)
