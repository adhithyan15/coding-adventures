# CodingAdventures.ImagePointOps (IMG03)

Per-pixel point operations on `PixelContainer`. Every operation transforms
each pixel independently using only that pixel's value — no neighbourhood,
no frequency domain, no geometry.

## Where it fits in the stack

```
[IMG04 geometric transforms]       [image-point-ops  ← you are here]
                  \                          /
                   \                        /
                    └── IC00 pixel-container ──┘
```

`image-point-ops` and `image-geometric-transforms` are sibling packages —
neither depends on the other. Both sit directly on top of `pixel-container`
so they can be mixed and matched in any order (point-ops first for tonemapping,
then a geometric warp, or the other way around — both are valid).

## Two domains

- **u8 domain** — `Invert`, `Threshold`, `ThresholdLuminance`, `Posterize`,
  `SwapRgbBgr`, `ExtractChannel`, `Brightness`, `Contrast`. These treat
  sRGB bytes as plain integers to twiddle. No decode/encode round-trip.
- **Linear light** — `Gamma`, `Exposure`, `Greyscale`, `Sepia`, `ColourMatrix`,
  `Saturate`, `HueRotate`. These decode to linear [0,1], operate physically,
  then re-encode. Alpha is preserved.

## Utilities

- `SrgbToLinearImage` / `LinearToSrgbImage` — reinterpret the bytes of a
  container as sRGB-encoded or linearly-encoded and apply the opposite curve.
- `ApplyLut1dU8(src, lutR, lutG, lutB)` — apply independent 256-entry LUTs
  per channel. Fastest way to implement any monotone curve.
- `BuildLut1dU8(f)` — build a u8→u8 LUT from a linear-domain function.
- `BuildGammaLut(g)` — convenience wrapper for power-law curves.

## Usage

```csharp
using CodingAdventures.PixelContainer;
using CodingAdventures.ImagePointOps;

var img = PixelContainers.Create(256, 256);
img.Fill(100, 150, 200, 255);

var inverted = ImagePointOps.Invert(img);
var posterized = ImagePointOps.Posterize(img, 4);
var sepia = ImagePointOps.Sepia(img);
var warmed = ImagePointOps.ColourMatrix(img, new double[,] {
    { 1.1, 0, 0 },
    { 0, 1.0, 0 },
    { 0, 0, 0.9 }
});
```

## Build

```
./build-tool --only csharp/image-point-ops
```

The BUILD script sets `HOME`/`DOTNET_CLI_HOME` to a local `.dotnet/` directory
so CI doesn't need a pre-populated user profile. Coverage threshold is 80%
line coverage enforced by `coverlet.msbuild`.
