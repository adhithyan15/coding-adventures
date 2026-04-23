# image-geometric-transforms

**IMG04** — Geometric transforms for `PixelContainer` images.

This package implements the spatial-transform layer of the coding-adventures
image pipeline.  It operates on the `*pc.PixelContainer` type from
[pixel-container](../pixel-container/README.md) and sits above the point-ops
layer (IMG03) and below any compositing or text-rendering layers.

---

## What This Package Does

| Category | Operations |
|---|---|
| Lossless pixel-copy | `FlipHorizontal`, `FlipVertical`, `Rotate90CW`, `Rotate90CCW`, `Rotate180`, `Crop`, `Pad` |
| Continuous resampling | `Scale`, `Rotate`, `Affine`, `PerspectiveWarp` |
| Public sampling API | `Sample(img, u, v, mode, oob)` |

Lossless operations copy pixel bytes verbatim — no colour-space conversion,
no rounding error.  Continuous operations perform an inverse warp and
reconstruct source values using one of three interpolation filters.

---

## Stack Position

```
IMG04  image-geometric-transforms  ← this package
IMG03  image-point-ops
IMG02  image-codec-qoi / image-codec-ppm / image-codec-bmp
IMG01  pixel-container
```

---

## Key Concepts

### Inverse Warp ("Pull" Sampling)

Every continuous transform iterates over output pixels and asks "which source
coordinate maps here?" rather than pushing source pixels forward.  This
guarantees no holes and no double-writes.

### Pixel-Centre Model

A pixel at integer index `x` occupies the interval `[x, x+1)`.  Its centre is
at `x + 0.5`.  Scale uses:

```
u = (x' + 0.5) * (srcW / outW) - 0.5
```

to align source and output pixel centres correctly, avoiding the half-pixel
shift that occurs with the simpler `u = x' * sx`.

### Linear-Light Interpolation

Pixel bytes are in the sRGB colour space (≈ gamma 2.2).  Averaging sRGB values
directly produces visually dark results.  All continuous operations decode bytes
to linear-light floats via a 256-entry LUT, blend in linear space, then
re-encode to sRGB.

### Interpolation Filters

| Mode | Quality | Speed | When to use |
|---|---|---|---|
| `Nearest` | Blocky | Fastest | Pixel art, lossless preview |
| `Bilinear` | Smooth | Fast | General resizing |
| `Bicubic` | Very smooth | Moderate | High-quality upscaling |

Bicubic uses the Catmull-Rom kernel (α = 0.5, Keys 1983) over a 4×4
neighbourhood.

### Out-of-Bounds Policies

| Policy | Behaviour |
|---|---|
| `Zero` | Out-of-bounds → transparent black `(0,0,0,0)` |
| `Replicate` | Clamp to nearest edge pixel |
| `Reflect` | Mirror at edges |
| `Wrap` | Tile (modulo) |

---

## Usage

```go
import (
    igt "github.com/adhithyan15/coding-adventures/code/packages/go/image-geometric-transforms"
    pc  "github.com/adhithyan15/coding-adventures/code/packages/go/pixel-container"
)

src := pc.New(640, 480)
// ... fill src with pixels ...

// Flip horizontally (lossless).
flipped := igt.FlipHorizontal(src)

// Scale to thumbnail (bilinear, no colour shift).
thumb := igt.Scale(src, 160, 120, igt.Bilinear)

// Rotate 30° with Fit canvas, bicubic quality.  Use igt.CropBounds to keep src size.
rotated := igt.Rotate(src, math.Pi/6, igt.Bicubic, igt.Fit)

// Add a 10-pixel white border on all sides.
padded := igt.Pad(src, 10, 10, 10, 10, igt.Rgba8{255, 255, 255, 255})

// Crop a 100×80 region starting at (50, 40).
cropped := igt.Crop(src, 50, 40, 100, 80)

// Arbitrary affine: scale x by 0.5, keep y (inverse matrix).
matrix := [2][3]float64{{2.0, 0, 0}, {0, 1, 0}}
stretched := igt.Affine(src, matrix, 320, 480, igt.Bilinear, igt.Zero)

// Perspective homography (identity shown).
h := [3][3]float64{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}
warped := igt.PerspectiveWarp(src, h, 640, 480, igt.Bilinear, igt.Zero)

// Low-level sampling at a continuous coordinate.
r, g, b, a := igt.Sample(src, 3.7, 2.1, igt.Bicubic, igt.Replicate)
_ = r; _ = g; _ = b; _ = a
```

---

## Tests

```
go test ./...
```

The test suite contains 30 tests covering:

- Lossless identity round-trips (double-flip, CW+CCW rotation, 180°×2)
- Dimension invariants for every operation
- Pixel-position correctness for flips, crops, pads, and 90° rotations
- Continuous-transform approximate-identity checks (±2–3 per channel)
- Nearest-neighbour exact-pixel and OOB behaviour
- Bilinear midpoint blend correctness in linear-light space
- Bicubic stability on solid images

---

## Module Path

```
github.com/adhithyan15/coding-adventures/code/packages/go/image-geometric-transforms
```

Requires Go 1.26.
