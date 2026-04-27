# image-point-ops (Go)

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

## Stack position

```
IMG03 image-point-ops
 └── IC00 pixel-container   (PixelContainer — RGBA8 raster store)
```

## Operations

| Category     | Functions |
|--------------|-----------|
| u8-domain    | `Invert`, `Threshold`, `ThresholdLuminance`, `Posterize`, `SwapRGBBGR`, `ExtractChannel`, `Brightness` |
| Linear-light | `Contrast`, `Gamma`, `Exposure`, `Greyscale`, `Sepia`, `ColourMatrix`, `Saturate`, `HueRotate` |
| Colorspace   | `SRGBToLinearImage`, `LinearToSRGBImage` |
| LUT          | `ApplyLUT1DU8`, `BuildLUT1DU8`, `BuildGammaLUT` |

## Usage

```go
import (
    ops "github.com/adhithyan15/coding-adventures/code/packages/go/image-point-ops"
    pc  "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

img := pc.New(640, 480)
// … fill with pixels …

inverted := ops.Invert(img)
darkened  := ops.Gamma(img, 2.0)        // γ > 1 → darker
bw        := ops.Greyscale(img, ops.Rec709)
```

## Testing

```
go test ./...
```
