# IMG04 — Geometric Transforms

## Overview

A **geometric transform** remaps pixel *locations* rather than pixel *values*.
The output image O is constructed by choosing, for each output coordinate
(x′, y′), a source coordinate (u, v) in the input image I and sampling I there:

```
(u, v) = T⁻¹(x′, y′)          — inverse warp: where to read from input
O(x′, y′) = sample(I, u, v)   — sampling with interpolation
```

This **inverse warp** convention is used throughout: we iterate over output
pixels and ask "which input pixel contributes here?" rather than iterating
over input pixels and asking "where does this pixel go?" The inverse warp
avoids holes (unmapped output pixels) and maps cleanly to parallelism — each
output pixel is computed independently.

Geometric transforms fall into two families:

| Family             | Examples                                   | Pixel quality loss? |
|--------------------|---------------------------------------------|---------------------|
| Integer (lossless) | Flip H/V, rotate 90°/180°/270°, crop       | None — u=integer    |
| Continuous (lossy) | Arbitrary rotate, scale, affine, perspective| Interpolation error |

Lossless transforms copy bytes without any arithmetic; continuous transforms
require **interpolation** because (u, v) is generally non-integer.

---

## 1. `Image` as the Base Type

All operations in this spec accept and return `Image` (IC00).
For **lossless transforms** the RGBA8 bytes are copied directly with no
colorspace conversion. For **continuous transforms** that require
interpolation, the sRGB u8 values are first decoded to linear f32, the
interpolation is performed in linear light, and the result is re-encoded
to sRGB u8. (Interpolating in sRGB is incorrect — see §3.4.)

---

## 2. Integer (Lossless) Transforms

These transforms map every output pixel to an exact integer input coordinate.
No interpolation, no precision loss, no colorspace conversion.

### 2.1 Flip horizontal

```
T⁻¹(x′, y′) = (W − 1 − x′, y′)

O(x′, y′) = I(W − 1 − x′, y′)
```

Mirror left↔right. Each row is reversed in place.

Implementation: for each row, copy bytes in reverse order. Since each pixel
is 4 bytes (RGBA8), reverse 4-byte groups rather than individual bytes.

### 2.2 Flip vertical

```
T⁻¹(x′, y′) = (x′, H − 1 − y′)
```

Mirror top↔bottom. Rows are reordered; bytes within a row are unchanged.

Implementation: swap row i with row (H−1−i) for i in [0, H/2).

### 2.3 Rotate 90° clockwise

Output dimensions: W′ = H, H′ = W.

```
T⁻¹(x′, y′) = (y′, W − 1 − x′)

O(x′, y′) = I(y′, W − 1 − x′)
```

Visualisation:
```
Input:        Output (90° CW):
  A B C         D A
  D E F   →     E B
                F C
```

### 2.4 Rotate 90° counter-clockwise

Output dimensions: W′ = H, H′ = W.

```
T⁻¹(x′, y′) = (H − 1 − y′, x′)
```

### 2.5 Rotate 180°

Output dimensions: W′ = W, H′ = H.

```
T⁻¹(x′, y′) = (W − 1 − x′, H − 1 − y′)
```

Equivalent to flip-horizontal then flip-vertical (or vice versa).

### 2.6 Crop

Extract a rectangular sub-region. The output has dimensions (w, h) starting
at input pixel (x₀, y₀):

```
O(x′, y′) = I(x₀ + x′, y₀ + y′)
    for x′ in [0, w) and y′ in [0, h)
```

Crop always allocates a new Image and copies the pixel data. (A
zero-copy crop would require exposing stride, which the extended Image
supports — see IMG00 §10. Until that migration happens, crop copies.)

### 2.7 Pad

Extend the image with a border of colour C (default: transparent black):

```
Input (W×H) → Output ((W + left + right) × (H + top + bottom))

O(x′, y′):
  if x′ in [left, left+W) and y′ in [top, top+H):
      = I(x′ − left, y′ − top)
  else:
      = C
```

---

## 3. Continuous (Lossy) Transforms

### 3.1 Interpolation modes

When the source coordinate (u, v) is non-integer, the sampled value is
estimated from the surrounding pixel grid. The choice of interpolation
mode trades quality against speed.

#### Nearest-neighbour

```
sample_nn(I, u, v) = I(round(u), round(v))
```

Round each coordinate to the nearest integer and read that pixel. No
blending. Output: sharp but jagged ("pixelated") on strong scaling or
rotation.

Cost: O(1) per output pixel — one array lookup.

#### Bilinear

Take the 2×2 grid of pixels surrounding (u, v) and blend them with
weights proportional to the area of the opposing sub-rectangle:

```
x0 = floor(u);  x1 = x0 + 1;  fx = u − x0
y0 = floor(v);  y1 = y0 + 1;  fy = v − y0

p00 = I(x0, y0);  p10 = I(x1, y0)
p01 = I(x0, y1);  p11 = I(x1, y1)

sample_bilinear(I, u, v) =
    p00 * (1−fx) * (1−fy)
  + p10 *   fx   * (1−fy)
  + p01 * (1−fx) *   fy
  + p11 *   fx   *   fy
```

Cost: O(1) — four array lookups and eight multiply-adds.
Output: smooth, no jaggies, slight blur.

#### Bicubic (Catmull-Rom)

Sample from the 4×4 grid surrounding (u, v) using cubic weights:

```
For a coordinate t ∈ [0, 1) between samples at positions 0 and 1,
the Catmull-Rom cubic weight for position p ∈ {−1, 0, 1, 2} is:

w(t, p):
  d = |p − t|    (distance from the sample position)
  if d < 1:  w = 1.5d³ − 2.5d² + 1
  if d < 2:  w = −0.5d³ + 2.5d² − 4d + 2
  else:      w = 0

Applied in 2D: compute 4 horizontal blends (one per row), then blend
the 4 results vertically.
```

Cost: O(1) — 16 array lookups, 32 multiply-adds per pixel.
Output: sharp and smooth. The default for high-quality scaling.

#### Interpolation mode summary

```
Mode            Speed     Quality   Use case
────────────────────────────────────────────────────────────────
Nearest         Fast      Low       Pixel art, thumbnail preview
Bilinear        Medium    Good      General purpose downscaling
Bicubic         Slow      Best      Upscaling, print, final output
```

### 3.2 Out-of-bounds handling

When the source coordinate falls outside the image boundary, use one of the
same padding modes as IMG01 §4:

| Mode       | Behaviour                              |
|------------|----------------------------------------|
| Zero       | Return transparent black (0,0,0,0)     |
| Replicate  | Clamp to the nearest edge pixel        |
| Reflect    | Mirror at the boundary                 |
| Wrap       | Tile the image                         |

`Replicate` is the default for most transforms (avoids dark halos at edges).
`Zero` is appropriate when the background is expected to show through
(e.g., rotating an image with no background fill).

### 3.3 Linear-light requirement

Bilinear and bicubic interpolation compute weighted averages. Averaging
must happen in **linear light** to be physically correct (see IMG00 §2,
IMG03 §1):

```
Blending 50% black and 50% white:
  sRGB blend:    (0   + 255  ) / 2 = 127  → perceptually dark (wrong)
  linear blend:  encode(0.0 + 1.0) / 2) = encode(0.5) ≈ 188  (correct)
```

Procedure for all continuous transforms:
1. Decode source pixels to linear f32 as they are sampled.
2. Perform the weighted blend in linear f32.
3. Re-encode the result to sRGB u8 for the output Image.

For nearest-neighbour (no blending), colorspace conversion is not needed —
the source RGBA8 pixel is copied as-is.

---

## 4. Scale (Resize)

Resize an image from (W, H) to (W′, H′). The scale factors:

```
sx = W′ / W
sy = H′ / H
```

Inverse warp: for each output pixel (x′, y′), the source coordinate is:

```
u = (x′ + 0.5) / sx − 0.5
v = (y′ + 0.5) / sy − 0.5
```

The +0.5 / −0.5 terms implement the **pixel-centre model** (IMG00 §4):
the centre of output pixel x′ maps to the centre of the corresponding
input pixel. This prevents the one-pixel shift that occurs when using
the simpler formula u = x′ / sx.

### Downscaling artefacts

When shrinking an image (sx < 1 or sy < 1), each output pixel covers more
than one input pixel. Bilinear interpolation at the output pixel centre
misses the contributions of the surrounding input pixels, causing **aliasing**
(moiré patterns, jagged edges).

The correct approach for downscaling is to pre-blur the source with a
Gaussian of σ proportional to 1/sx before sampling:

```
// Downscaling by factor k (output is 1/k of input in each dimension):
σ = 0.5 / k                   (half the inter-pixel spacing in output space)
blurred = gaussian_blur(src, σ, padding=Replicate)
result = scale(blurred, W′, H′, interpolation=Bilinear)
```

This is equivalent to the "area sampling" approach and avoids aliasing at the
cost of one extra blur pass.

For upscaling (sx > 1), no pre-blur is needed — use bicubic directly.

---

## 5. Rotate (Arbitrary Angle)

Rotate the image by angle θ (radians, counter-clockwise positive) around a
centre point (cx, cy):

```
Inverse warp:
  dx = x′ − cx
  dy = y′ − cy
  u = cx +  cos(θ) * dx + sin(θ) * dy
  v = cy + −sin(θ) * dx + cos(θ) * dy
```

The output image dimensions can be:
- **Fit** (default): output is large enough to contain the entire rotated
  source. Dimensions are:
  ```
  W′ = |W cos θ| + |H sin θ|
  H′ = |W sin θ| + |H cos θ|
  ```
- **Crop**: output has the same dimensions as the input. Corners of the
  input are clipped.

The centre point (cx, cy) defaults to (W/2, H/2) — the image centre.

Background pixels (where the inverse warp falls outside the source) use the
configured out-of-bounds mode (default: transparent black).

---

## 6. Affine Transform

An affine transform is any combination of translation, rotation, scale, shear,
and reflection that preserves parallel lines. It is represented as a 2×3 matrix
M applied to homogeneous coordinates:

```
[u]   [m00 m01 m02]   [x′]
[v] = [m10 m11 m12] × [y′]
                       [1 ]

u = m00*x′ + m01*y′ + m02
v = m10*x′ + m11*y′ + m12
```

Common affine matrices:

```
Translation by (tx, ty):
  [1  0  tx]
  [0  1  ty]

Scale by (sx, sy):
  [sx  0   0]
  [ 0  sy  0]

Rotation by θ (CW):
  [ cos θ  sin θ  0]
  [−sin θ  cos θ  0]

Shear in X by factor k:
  [1  k  0]
  [0  1  0]
```

Affine matrices compose by multiplication (M_total = M_last × … × M_first).

The output image size must be specified by the caller (unlike the rotation
convenience function which auto-computes it). The forward transform M maps
input coordinates to output coordinates; the implementation uses M⁻¹ for the
inverse warp. The 2×3 inverse of a 2×3 affine matrix:

```
det = m00*m11 − m01*m10

M⁻¹ = (1/det) × [ m11   −m01  m01*m12 − m02*m11 ]
                  [−m10    m00  m02*m10 − m00*m12 ]
```

---

## 7. Perspective Transform

A perspective (projective) transform maps straight lines to straight lines
but does not preserve parallelism. It is represented as a 3×3 homogeneous
matrix H:

```
[u_h]   [h00 h01 h02]   [x′]
[v_h] = [h10 h11 h12] × [y′]
[ w ]   [h20 h21 h22]   [1 ]

u = u_h / w
v = v_h / w
```

The most common use case: correct for perspective distortion in a photograph
of a flat surface shot at an angle (document scanning, whiteboard capture,
sports tracking).

The matrix H is typically computed from four point correspondences — four
points in the output that map to four known points in the input:

```
output points: [(x′₀,y′₀), (x′₁,y′₁), (x′₂,y′₂), (x′₃,y′₃)]
input points:  [(u₀,v₀),   (u₁,v₁),   (u₂,v₂),   (u₃,v₃)  ]
```

This gives 8 equations in 8 unknowns (H has 9 elements but is determined
up to scale, so 8 degrees of freedom). Solving the resulting linear system
via Gaussian elimination or SVD yields H.

The full derivation of the linear system is:

```
For each correspondence i:
  u_i * (h20*x′_i + h21*y′_i + h22) = h00*x′_i + h01*y′_i + h02
  v_i * (h20*x′_i + h21*y′_i + h22) = h10*x′_i + h11*y′_i + h12

Rearrange into the form A × h = 0 where h = [h00…h22]ᵀ, solve via SVD.
```

---

## 8. Seam Carving (Content-Aware Scaling)

Seam carving is a special case of geometric transform that removes or inserts
**seams** — connected paths of low-energy pixels — to resize an image while
preserving visually important content. It is not a simple geometric warp but
belongs in this spec because it is a lossless (in terms of remaining content)
resize operation.

### Algorithm outline

```
1. Compute an energy map E(x, y) = |∂I/∂x| + |∂I/∂y|   (Sobel magnitude)
2. Compute the cumulative minimum energy M(x, y) via dynamic programming:
     M(x, y) = E(x, y) + min(M(x−1,y−1), M(x,y−1), M(x+1,y−1))
3. Trace back the minimum-cost vertical seam from M(x, H−1)
4. Remove the seam: shift all pixels to the right of it one column left
5. Repeat (1–4) until the desired width is reached
```

To increase width, insert duplicate seams at the identified positions.

Seam carving is computationally intensive (O(WH) per seam removed) and
is most useful for asymmetric content-aware thumbnail generation. It is
specified here for completeness; GPU acceleration (IMG06) is strongly
recommended for real-time use.

---

## 9. Interface

```
// Lossless integer transforms — no interpolation, no colorspace conversion:
fn flip_horizontal(src: &Image) -> Image
fn flip_vertical(src: &Image) -> Image
fn rotate_90_cw(src: &Image) -> Image
fn rotate_90_ccw(src: &Image) -> Image
fn rotate_180(src: &Image) -> Image
fn crop(src: &Image, x: u32, y: u32, w: u32, h: u32) -> Image
fn pad(src: &Image, top: u32, right: u32, bottom: u32, left: u32, fill: Rgba8) -> Image

// Continuous transforms — linear-light interpolation:
fn scale(src: &Image, w: u32, h: u32, mode: Interpolation) -> Image
fn rotate(src: &Image, radians: f32, mode: Interpolation, bounds: RotateBounds) -> Image
fn affine(src: &Image, matrix: [[f32; 3]; 2], w: u32, h: u32, mode: Interpolation, oob: OutOfBounds) -> Image
fn perspective(src: &Image, h: [[f32; 3]; 3], w: u32, h: u32, mode: Interpolation, oob: OutOfBounds) -> Image

// enums:
enum Interpolation { Nearest, Bilinear, Bicubic }
enum RotateBounds  { Fit, Crop }
enum OutOfBounds   { Zero, Replicate, Reflect, Wrap }
type Rgba8 = (u8, u8, u8, u8)
```
