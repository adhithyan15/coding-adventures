# Java Image Geometric Transforms

IMG04 — geometric transforms on `PixelContainer`. Where IMG03 (point ops)
changes a pixel's *value*, IMG04 changes a pixel's *position*. All
spatial operations live here: flips, 90° rotations, crop, scale, free
rotation, affine warp, and perspective warp.

## Axes of variation

1. **Lossless vs continuous**. Flips, 90° rotations, and crop are pure
   byte reshuffles and need no sampling. Scale, rotate, translate,
   affine, and perspective warp all need to sample at fractional
   coordinates.
2. **Interpolation**. `NEAREST`, `BILINEAR`, `BICUBIC` (Catmull-Rom). All
   colour blending happens in linear light; alpha blends in u8.
3. **Out-of-bounds**. `ZERO` (transparent black), `REPLICATE` (clamp),
   `REFLECT` (mirror), `WRAP` (tile).

## Warp model

All continuous ops use *inverse* warps: for each output pixel, compute
the source coordinate and sample. That avoids output holes you'd get
from a forward warp, at the cost of an extra matrix inversion where
matrices are involved.

## Usage

```java
import com.codingadventures.imagegeometrictransforms.*;

PixelContainer flipped = ImageGeometricTransforms.flipHorizontal(src);
PixelContainer halved  = ImageGeometricTransforms.scale(
    src, src.width/2, src.height/2, Interpolation.BILINEAR, OutOfBounds.REPLICATE);
PixelContainer rotated = ImageGeometricTransforms.rotate(
    src, 30, RotateBounds.FIT, Interpolation.BICUBIC, OutOfBounds.ZERO);
```

## Depends on

- `pixel-container` (IC00).
