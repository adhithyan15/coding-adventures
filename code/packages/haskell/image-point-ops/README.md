# image-point-ops (Haskell)

IMG03 — Per-pixel point operations on `PixelContainer`.

## Overview

A /point operation/ is any transform where the output pixel at `(x, y)`
depends only on the input pixel at `(x, y)` — no neighbourhood, no history.
This package provides the full catalogue.

All arithmetic operations (gamma, exposure, greyscale, sepia, saturation,
hue-rotate, colour matrix) are performed in linear-light space so they are
physically correct.  Pure index-remapping ops (invert, threshold, swap, LUTs)
stay in sRGB.

## Usage

```haskell
import PixelContainer
import ImagePointOps

main :: IO ()
main = do
    let pc = fillPixels (createPixelContainer 8 8) 200 100 50 255
    let result = gamma (greyscale pc Rec709) 2.2
    print (pixelAt result 0 0)
```

## API

- `invert`, `threshold`, `thresholdLuminance`, `posterize`
- `swapRgbBgr`, `extractChannel`
- `brightness`, `contrast`, `gamma`, `exposure`
- `greyscale` with `GreyscaleMethod = Rec709 | Bt601 | Average`
- `sepia`, `colourMatrix`, `saturate`, `hueRotate`
- `srgbToLinearImage`, `linearToSrgbImage`
- `applyLut1dU8`, `buildLut1dU8`, `buildGammaLut`

## Dependencies

- `pixel-container` (IC00)
