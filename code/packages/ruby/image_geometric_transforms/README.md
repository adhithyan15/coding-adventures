# CodingAdventures::ImageGeometricTransforms

IMG04 — Geometric transforms on RGBA8 PixelContainers.

Implements lossless pixel-copy operations (flip, rotate-90, crop, pad) and
continuous backward-mapping transforms (scale, rotate, affine,
perspective-warp) with three interpolation modes and four out-of-bounds
strategies, all using correct sRGB ↔ linear-light conversion where required.

## Stack position

```
pixel_container  (IMG00)   — raw RGBA8 buffer
     ↓
image_point_ops  (IMG03)   — per-pixel transforms
     ↓
image_geometric_transforms (IMG04)  ← this package
```

## Quick Start

```ruby
$LOAD_PATH.unshift "path/to/pixel_container/lib"
require "coding_adventures/pixel_container"
require "coding_adventures/image_geometric_transforms"

PC = CodingAdventures::PixelContainer
GT = CodingAdventures::ImageGeometricTransforms

src = PC.create(100, 100)
# ... fill src with pixels ...

# Flip left-to-right
flipped = GT.flip_horizontal(src)

# Rotate 90° clockwise (dimensions swap: 100×100 → 100×100)
rotated = GT.rotate_90_cw(src)

# Scale to 200×150 using bilinear interpolation (default)
scaled = GT.scale(src, 200, 150)

# Scale using nearest-neighbour (fast, pixel-art style)
scaled_nn = GT.scale(src, 200, 150, mode: :nearest)

# Rotate 30° with fit canvas (no clipping)
dst = GT.rotate(src, Math::PI / 6, bounds: :fit)

# Affine transform: 2D shear
matrix = [[1.0, 0.3, 0.0],   # x' = x + 0.3*y
          [0.0, 1.0, 0.0]]   # y' = y
sheared = GT.affine(src, matrix, src.width, src.height)

# Perspective warp
h = [[1.0, 0.0, 0.0],
     [0.0, 1.0, 0.0],
     [0.0, 0.0, 1.0]]  # identity
warped = GT.perspective_warp(src, h, src.width, src.height)
```

## API

### Lossless transforms (raw-byte copy, no colour-space conversion)

| Method | Description |
|---|---|
| `flip_horizontal(src)` | Mirror left-to-right |
| `flip_vertical(src)` | Mirror top-to-bottom |
| `rotate_90_cw(src)` | Rotate 90° clockwise; dimensions swap |
| `rotate_90_ccw(src)` | Rotate 90° counter-clockwise; dimensions swap |
| `rotate_180(src)` | Rotate 180°; same dimensions |
| `crop(src, x0, y0, w, h)` | Extract w×h rectangle at (x0, y0) |
| `pad(src, top, right, bottom, left, fill: [0,0,0,0])` | Add border pixels |

### Continuous transforms (backward mapping with interpolation)

| Method | Signature | Description |
|---|---|---|
| `scale` | `(src, out_w, out_h, mode: :bilinear)` | Resize to out_w×out_h |
| `rotate` | `(src, radians, mode: :bilinear, bounds: :fit)` | Rotate about centre |
| `affine` | `(src, matrix, out_w, out_h, mode: :bilinear, oob: :replicate)` | 2×3 affine |
| `perspective_warp` | `(src, h, out_w, out_h, mode: :bilinear, oob: :replicate)` | 3×3 homography |

### Interpolation modes

| Symbol | Description |
|---|---|
| `:nearest` | Nearest-neighbour — fast, blocky on scale-up |
| `:bilinear` | 2×2 weighted average — smooth, slight blur |
| `:bicubic` | 4×4 Catmull-Rom spline — sharper than bilinear |

### Out-of-bounds (OOB) strategies

| Symbol | Description |
|---|---|
| `:zero` | Outside pixels are transparent black `[0,0,0,0]` |
| `:replicate` | Clamp to the nearest edge pixel |
| `:reflect` | Mirror at each edge (period = 2 * dimension) |
| `:wrap` | Tile the image periodically |

## Design notes

### Pixel-centre model

All continuous transforms use the pixel-centre coordinate convention:
pixel `(x, y)` occupies the unit square centred at `(x + 0.5, y + 0.5)`.
The mapping for `scale(src, out_w, out_h)` is therefore:

```
u = (x + 0.5) * (W / out_w) - 0.5
v = (y + 0.5) * (H / out_h) - 0.5
```

This ensures the top-left output pixel maps to the top-left input pixel,
not to the gap between pixels, which prevents a systematic half-pixel offset
visible as a slight drift in repeated transforms.

### sRGB ↔ linear light

Blending (bilinear, bicubic) is performed in linear light. Doing it in the
non-linear sRGB encoding produces visually dark transitions — the classic
"too-dark midpoint" problem. The module pre-computes a 256-entry decode LUT
to avoid repeated calls to the gamma-expansion formula.

The alpha channel is always treated as linear (it is never gamma-encoded).

### Backward mapping

All continuous transforms iterate over output pixels and compute the
corresponding source coordinate. This avoids holes (empty output pixels)
that would occur in forward mapping when the warp contracts the image.

## Running the tests

```bash
ruby -I lib -I ../pixel_container/lib test/test_image_geometric_transforms.rb
```

37 tests, >113 assertions.
