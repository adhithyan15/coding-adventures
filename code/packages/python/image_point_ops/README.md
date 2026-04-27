# coding-adventures-image-point-ops

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

## Stack position

```
IMG03 image-point-ops
 └── IC00 pixel-container   (PixelContainer — RGBA8 raster store)
```

## Operations

| Category     | Function(s) |
|--------------|-------------|
| u8-domain    | `invert`, `threshold`, `threshold_luminance`, `posterize`, `swap_rgb_bgr`, `extract_channel`, `brightness` |
| Linear-light | `contrast`, `gamma`, `exposure`, `greyscale`, `sepia`, `colour_matrix`, `saturate`, `hue_rotate` |
| Colorspace   | `srgb_to_linear_image`, `linear_to_srgb_image` |
| LUT          | `apply_lut1d_u8`, `build_lut1d_u8`, `build_gamma_lut` |

## Usage

```python
from pixel_container import create_pixel_container
from image_point_ops import invert, gamma, greyscale, GreyscaleMethod

img = create_pixel_container(640, 480)
# … fill with pixels …

inverted = invert(img)
darkened  = gamma(img, 2.0)   # γ > 1 → darker
bw        = greyscale(img, GreyscaleMethod.REC709)
```

## Testing

```
uv venv
uv pip install -e ../pixel_container
uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
