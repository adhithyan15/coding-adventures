# CodingAdventures.ImageGeometricTransforms

**IMG04** — Geometric transforms on `PixelContainer`.

Sits in the image-processing stack directly above
[`CodingAdventures.PixelContainer`](../pixel-container) and alongside
[`CodingAdventures.ImagePointOps`](../image-point-ops).

---

## What it does

This library provides functions that rearrange, resize, or warp a
`PixelContainer` image.  Operations fall into two categories:

| Category | Functions |
|---|---|
| **Lossless** (exact byte copies) | `flipHorizontal`, `flipVertical`, `rotate90CW`, `rotate90CCW`, `rotate180`, `crop`, `pad` |
| **Continuous** (require interpolation) | `scale`, `rotate`, `affine`, `perspectiveWarp` |

Lossless transforms rearrange pixels without any floating-point arithmetic.
Continuous transforms use reverse mapping: for every output pixel (x′, y′) the
library computes the corresponding source coordinate (u, v) and calls the
selected interpolation filter.

---

## Coordinate model

We use the **pixel-centre** model throughout.  The centre of pixel (x, y)
sits at continuous coordinate (x + 0.5, y + 0.5).  This keeps sampling
symmetric under scale and avoids off-by-half-pixel drift.

---

## Colour-correct interpolation

Bilinear and bicubic filters decode each sRGB byte to linear light before
blending, then re-encode the result.  Blending in sRGB space is incorrect
because the sRGB transfer function is non-linear; averaging γ-encoded values
produces a darker result than averaging the underlying physical light values.
Nearest-neighbour copies bytes directly without conversion (no blending occurs).

---

## Interpolation modes

```fsharp
type Interpolation = Nearest | Bilinear | Bicubic
```

| Mode | Quality | Speed |
|---|---|---|
| `Nearest` | Aliased; exact on integer-ratio scales | Fastest |
| `Bilinear` | Smooth; 2×2 neighbourhood | Medium |
| `Bicubic` | Sharp; 4×4 Catmull-Rom spline | Slower |

---

## Out-of-bounds policies

```fsharp
type OutOfBounds = Zero | Replicate | Reflect | Wrap
```

| Policy | Behaviour |
|---|---|
| `Zero` | Transparent black (0, 0, 0, 0) |
| `Replicate` | Clamp to the nearest edge pixel |
| `Reflect` | Mirror at each edge (period = 2 × dimension) |
| `Wrap` | Tile the image (modular arithmetic) |

---

## Usage examples

```fsharp
open CodingAdventures.PixelContainer
open CodingAdventures.ImageGeometricTransforms

// Load your image into a PixelContainer (codec not shown).
let src = PixelContainer(640, 480)

// --- Lossless ---
let flipped = flipHorizontal src          // mirror left-right
let cropped = crop src 10 10 100 100      // 100×100 region at (10,10)
let padded  = pad src 10 10 10 10 (0uy, 0uy, 0uy, 255uy)  // 10-px black border

// --- Scale to 320×240 using bilinear ---
let small = scale src 320 240 Bilinear

// --- Rotate 45° with expanded canvas ---
let rotated = rotate src (System.Math.PI / 4.0) Bicubic Fit

// --- Arbitrary affine (identity): u = 1*x + 0*y + 0;  v = 0*x + 1*y + 0 ---
let identityMatrix = array2D [[1.0; 0.0; 0.0]; [0.0; 1.0; 0.0]]
let warped = affine src identityMatrix src.Width src.Height Bilinear Replicate

// --- Perspective warp with identity homography ---
let h = array2D [[1.0; 0.0; 0.0]; [0.0; 1.0; 0.0]; [0.0; 0.0; 1.0]]
let perspective = perspectiveWarp src h src.Width src.Height Bicubic Zero
```

---

## How it fits in the stack

```
PixelContainer          — raw RGBA8 pixel buffer
ImagePointOps (IMG03)   — per-pixel colour operations
ImageGeometricTransforms (IMG04)  ← you are here
```

---

## Running the tests

```sh
bash BUILD
```

The BUILD script runs `dotnet test` with code-coverage collection.
Coverage threshold is 80% line coverage (currently ≈ 83%).

---

## License

MIT
