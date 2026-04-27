# @coding-adventures/image-point-ops

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

A *point operation* transforms each pixel independently using only that
pixel's value — no neighbourhood, no frequency domain, no geometry.

## Stack position

```
IMG03 image-point-ops
 └── IC00 pixel-container   (PixelContainer — RGBA8 raster store)
```

## Operations

| Category     | Function(s) |
|--------------|-------------|
| u8-domain    | `invert`, `threshold`, `thresholdLuminance`, `posterize`, `swapRgbBgr`, `extractChannel`, `brightness` |
| Linear-light | `contrast`, `gamma`, `exposure`, `greyscale`, `sepia`, `colourMatrix`, `saturate`, `hueRotate` |
| Colorspace   | `srgbToLinearImage`, `linearToSrgbImage` |
| LUT          | `applyLut1dU8`, `buildLut1dU8`, `buildGammaLut` |

## Usage

```typescript
import { createPixelContainer, setPixel } from "@coding-adventures/pixel-container";
import { invert, gamma, greyscale } from "@coding-adventures/image-point-ops";

const img = createPixelContainer(640, 480);
// … fill with pixels …

const inverted = invert(img);
const darkened  = gamma(img, 2.0);   // γ > 1 → darker
const bw        = greyscale(img, "rec709");
```

## Testing

```
npm test
```
