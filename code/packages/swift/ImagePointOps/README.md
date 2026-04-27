# ImagePointOps (Swift)

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

## Stack position

```
IMG03 ImagePointOps
 └── IC00 PixelContainer   (PixelContainer — RGBA8 raster store)
```

## Operations

| Category     | Functions |
|--------------|-----------|
| u8-domain    | `invert`, `threshold`, `thresholdLuminance`, `posterize`, `swapRGBBGR`, `extractChannel`, `brightness` |
| Linear-light | `contrast`, `gamma`, `exposure`, `greyscale`, `sepia`, `colourMatrix`, `saturate`, `hueRotate` |
| Colorspace   | `srgbToLinearImage`, `linearToSRGBImage` |
| LUT          | `applyLUT1DU8`, `buildLUT1DU8`, `buildGammaLUT` |

## Usage

```swift
import PixelContainer
import ImagePointOps

var img = createPixelContainer(width: 640, height: 480)
// … fill with pixels …

let inverted = invert(img)
let darkened  = gamma(img, g: 2.0)
let bw        = greyscale(img, method: .rec709)
```

## Testing

```
swift test
```
