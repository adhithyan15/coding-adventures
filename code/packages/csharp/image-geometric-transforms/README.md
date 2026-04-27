# CodingAdventures.ImageGeometricTransforms (IMG04)

Geometric transforms on `PixelContainer`: flips, 90°/180° rotations, cropping,
scaling, arbitrary-angle rotation, translation, affine, and perspective.

## Where it fits in the stack

```
[IMG03 point-ops]       [image-geometric-transforms  ← you are here]
            \                          /
             \                        /
              └── IC00 pixel-container ──┘
```

Sibling of `image-point-ops`. Neither depends on the other. The sRGB
transfer-function LUT is duplicated so you can adopt this package without
pulling in point-ops.

## Operations

### Lossless (exact pixel relocations)
| Op | Output dimensions |
|----|-------------------|
| `FlipHorizontal`  | same as input |
| `FlipVertical`    | same as input |
| `Rotate90CW`      | (H, W)        |
| `Rotate90CCW`     | (H, W)        |
| `Rotate180`       | same as input |
| `Crop(x0,y0,w,h)` | (w, h); OOB → transparent black |

### Continuous (need interpolation)
| Op | Parameters |
|----|------------|
| `Scale(outW, outH, interp, oob)` | pixel-centre convention |
| `Rotate(deg, bounds, interp, oob)` | `Fit` expands canvas, `Crop` keeps size |
| `Translate(tx, ty, interp, oob)` | same dims; revealed area → OOB |
| `Affine(m[2,3], interp, oob)` | matrix is forward; inverted internally |
| `PerspectiveWarp(m[3,3], interp, oob)` | homography; inverted internally |

## Interpolation

- `Nearest` — one pixel lookup, blocky, best for pixel art.
- `Bilinear` — four pixels, smooth, cheap.
- `Bicubic` — sixteen pixels via Catmull-Rom, sharper than bilinear.

All continuous kernels blend **RGB in linear light** (decode → blend →
encode). Alpha stays in u8 space because it's already a linear coverage
value. Results are clamped to [0, 255] on encode.

## Out-of-bounds policies

- `Zero` — return (0, 0, 0, 0).
- `Replicate` — clamp to the nearest edge.
- `Reflect` — mirror at the edge.
- `Wrap` — modulo, for seamless tiles.

## Usage

```csharp
using CodingAdventures.PixelContainer;
using CodingAdventures.ImageGeometricTransforms;

var img = PixelContainers.Create(512, 512);

var flipped = ImageGeometricTransforms.FlipHorizontal(img);
var thumb   = ImageGeometricTransforms.Scale(img, 128, 128, Interpolation.Bilinear, OutOfBounds.Replicate);
var tilted  = ImageGeometricTransforms.Rotate(img, 15.0, RotateBounds.Fit, Interpolation.Bicubic, OutOfBounds.Zero);

// Arbitrary 2×3 affine:
var m = new double[,] { { 1.1, 0.1, 10 }, { 0.0, 0.9, 5 } };
var sheared = ImageGeometricTransforms.Affine(img, m, Interpolation.Bilinear, OutOfBounds.Replicate);
```

## Build

```
./build-tool --only csharp/image-geometric-transforms
```

Coverage threshold is 80% line coverage, enforced by `coverlet.msbuild`.
