# coding-adventures-image-point-ops (Lua)

**IMG03** — Per-pixel point operations for the `pixel_container` image type.

## Stack position

```
IMG03 coding_adventures.image_point_ops
 └── IC00 coding_adventures.pixel_container
```

## Operations

| Category     | Functions |
|--------------|-----------|
| u8-domain    | `invert`, `threshold`, `threshold_luminance`, `posterize`, `swap_rgb_bgr`, `extract_channel`, `brightness` |
| Linear-light | `contrast`, `gamma`, `exposure`, `greyscale`, `sepia`, `colour_matrix`, `saturate`, `hue_rotate` |
| Colorspace   | `srgb_to_linear_image`, `linear_to_srgb_image` |
| LUT          | `apply_lut1d_u8`, `build_lut1d_u8`, `build_gamma_lut` |

## Usage

```lua
local ipo = require("coding_adventures.image_point_ops")
local pc  = require("coding_adventures.pixel_container")

local img = pc.new(640, 480)

local inv = ipo.invert(img)
local bw  = ipo.greyscale(img, "rec709")
```

## Testing

```
cd tests && busted . --verbose --pattern=test_
```
