# CodingAdventures.ImagePointOps (F#)

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

## Stack position

```
IMG03 CodingAdventures.ImagePointOps
 └── IC00 CodingAdventures.PixelContainer
```

## Operations

| Category     | Functions |
|--------------|-----------|
| u8-domain    | `invert`, `threshold`, `thresholdLuminance`, `posterize`, `swapRGBBGR`, `extractChannel`, `brightness` |
| Linear-light | `contrast`, `gamma`, `exposure`, `greyscale`, `sepia`, `colourMatrix`, `saturate`, `hueRotate` |
| Colorspace   | `srgbToLinearImage`, `linearToSRGBImage` |
| LUT          | `applyLUT1DU8`, `buildLUT1DU8`, `buildGammaLUT` |

## Testing

```
dotnet test tests/CodingAdventures.ImagePointOps.Tests/CodingAdventures.ImagePointOps.Tests.fsproj
```
