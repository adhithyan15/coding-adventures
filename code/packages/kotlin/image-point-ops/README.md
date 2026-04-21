# image-point-ops (Kotlin)

**Spec:** IMG03 — Per-pixel point operations on `PixelContainer`.

This package implements every operation where the new value of a pixel is a
function of only that pixel's own value. No neighbourhood, no frequency
domain, no geometric resampling. Point ops are the simplest layer of the
image pipeline and the one most frequently done wrong — almost always by
omitting sRGB/linear conversion where it matters.

## Two domains

- **u8 domain** (fast, ignores photometry): `invert`, `threshold`,
  `thresholdLuminance`, `posterize`, `swapRgbBgr`, `extractChannel`,
  `brightness`, `contrast`.
- **Linear-light domain** (decodes sRGB, works on actual intensity, re-encodes):
  `gamma`, `exposure`, `greyscale`, `sepia`, `colourMatrix`, `saturate`,
  `hueRotate`, `srgbToLinearImage`, `linearToSrgbImage`.

`sRGB -> linear` uses a precomputed 256-entry lookup table. `linear -> sRGB`
is computed per call (pow is unavoidable without quantisation).

## LUTs

`buildLut1dU8 { x -> ... }` builds a 256-entry u8 lookup table from a
function over `[0, 1]`. `buildGammaLut(g)` is a convenience for `x^g`.
`applyLut1dU8` applies independent R/G/B tables to an image.

## Usage

```kotlin
import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.imagepointops.ImagePointOps

val grey = ImagePointOps.greyscale(src, ImagePointOps.GreyscaleMethod.REC709)
val darker = ImagePointOps.gamma(src, 2.2)
val warm = ImagePointOps.sepia(src)
```

## Dependencies

- `pixel-container` (IC00)

## Tests

```
gradle test
```
