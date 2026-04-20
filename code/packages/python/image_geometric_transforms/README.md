# image_geometric_transforms

**IMG04** — Geometric transformations on `PixelContainer`.

Part of the [coding-adventures](https://github.com/adhithyan/coding-adventures) image pipeline stack.

## What it does

This package implements 2-D spatial transforms on `PixelContainer` objects — operations that reposition pixels rather than changing their values in-place.  It sits one layer above `pixel_container` (IC00) and alongside `image_point_ops` (IMG03).

### Where it fits in the stack

```
IC00  pixel_container          — RGBA8 buffer + codec interface
IMG01 image_codec_bmp          — BMP encode/decode
IMG02 image_codec_qoi          — QOI encode/decode
IMG03 image_point_ops          — per-pixel colour operations
IMG04 image_geometric_transforms  ← you are here
```

## Transforms

### Lossless (exact byte copy, no interpolation)

| Function            | Description                                      |
|---------------------|--------------------------------------------------|
| `flip_horizontal`   | Mirror left-to-right                             |
| `flip_vertical`     | Mirror top-to-bottom                             |
| `rotate_90_cw`      | 90° clockwise rotation (swaps dimensions)        |
| `rotate_90_ccw`     | 90° counter-clockwise rotation (swaps dimensions)|
| `rotate_180`        | 180° rotation                                    |
| `crop`              | Extract a rectangular sub-region                 |
| `pad`               | Add a border of configurable fill colour         |

### Continuous (inverse-warp with interpolation)

| Function            | Description                                      |
|---------------------|--------------------------------------------------|
| `scale`             | Resize to arbitrary dimensions                   |
| `rotate`            | Rotate by arbitrary angle in radians             |
| `affine`            | Apply a 2×3 affine inverse-warp matrix           |
| `perspective_warp`  | Apply a 3×3 homographic (perspective) transform  |

## Design principles

### Inverse warp

Every continuous transform works by *inverse warp*: for each output pixel we
compute where to fetch from the source, rather than pushing source pixels
forward.  This guarantees exactly one write per output pixel with no holes.

### Pixel-centre model

A pixel at integer coordinates `(x, y)` is modelled as a 1×1 cell *centred*
at `(x, y)`.  Scaling uses the formula:

```
u = (x' + 0.5) * (W / W') - 0.5
```

This ensures pixel centres map to pixel centres at 1:1 scale and distributes
border pixels symmetrically.

### Colour-correct blending

Bilinear and bicubic sampling decode sRGB bytes to linear light before
blending, then re-encode.  This avoids the darkening artifact that occurs when
averaging gamma-compressed values directly.

## Enums

```python
class Interpolation(Enum):
    NEAREST  = "nearest"   # snap; fast, blocky on upscale
    BILINEAR = "bilinear"  # 2×2 blend; smooth, default
    BICUBIC  = "bicubic"   # 4×4 Catmull-Rom; sharpest

class RotateBounds(Enum):
    FIT  = "fit"   # expand canvas to avoid clipping
    CROP = "crop"  # keep original size, clip corners

class OutOfBounds(Enum):
    ZERO      = "zero"      # transparent black
    REPLICATE = "replicate" # clamp to nearest edge
    REFLECT   = "reflect"   # mirror at edges
    WRAP      = "wrap"      # tile periodically
```

## Usage

```python
from pixel_container import create_pixel_container, set_pixel, pixel_at
from image_geometric_transforms import (
    flip_horizontal, scale, rotate, affine, Interpolation, RotateBounds
)
import math

# Load an image (here we create one manually)
src = create_pixel_container(320, 240)
# ... populate src ...

# Flip
flipped = flip_horizontal(src)

# Scale to 640×480
big = scale(src, 640, 480, mode=Interpolation.BILINEAR)

# Rotate 30° clockwise (= -30° radians) with FIT canvas
rotated = rotate(src, -math.radians(30), bounds=RotateBounds.FIT)

# Arbitrary affine: shear by 0.2 in x
shear = [[1.0, 0.2, 0.0],
         [0.0, 1.0, 0.0]]
sheared = affine(src, shear, src.width, src.height)
```

## Installation

```bash
pip install coding-adventures-image_geometric_transforms
```

Or in development mode:

```bash
uv venv
uv pip install -e ../pixel_container
uv pip install -e ".[dev]"
```

## Testing

```bash
.venv/bin/python -m pytest tests/ -v --cov=image_geometric_transforms
```

## Dependencies

- `coding-adventures-pixel_container >= 0.1.0`
- Python >= 3.11
- Standard library only (`math`, `enum`)

## License

MIT
