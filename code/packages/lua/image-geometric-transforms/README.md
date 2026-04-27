# coding-adventures-image-geometric-transforms (Lua)

**IMG04** — Geometric transforms for the `pixel_container` image type.

## Stack position

```
IMG04 coding_adventures.image_geometric_transforms
 └── IC00 coding_adventures.pixel_container
```

## Overview

A geometric transform repositions pixels in 2-D space.  This package provides
two families of operations:

**Lossless (exact byte copy):** flip, rotate-by-90°, crop, pad.
No colour arithmetic — raw bytes are moved, not modified.  Applying the inverse
recovers the original image exactly.

**Continuous (sampled):** scale, arbitrary-angle rotate, affine warp, perspective
warp.  Each output pixel is computed by inverse-mapping back into source space
and blending nearby source pixels with a chosen interpolation filter.

## Interpolation modes

| Mode       | Description                                          |
|------------|------------------------------------------------------|
| `nearest`  | Snap to closest pixel.  Fast; aliased.               |
| `bilinear` | Weighted average of 2×2 neighbourhood. Default.      |
| `bicubic`  | Catmull-Rom 4×4 cubic filter.  Sharpest edges.       |

Bilinear and bicubic blending is performed in **linear light** (sRGB decoded
before blending, re-encoded after) to avoid the "dark edge" artefact.

## Out-of-bounds modes

| Mode        | Behaviour when source coord is outside image         |
|-------------|------------------------------------------------------|
| `zero`      | Return transparent black (0,0,0,0).  Default for rotate/perspective. |
| `replicate` | Clamp to nearest edge pixel.  Default for scale.     |
| `reflect`   | Mirror at the boundary.                              |
| `wrap`      | Tile the image.                                      |

## Operations

### Lossless

| Function              | Description                                  |
|-----------------------|----------------------------------------------|
| `flip_horizontal(src)` | Mirror each row left-to-right                |
| `flip_vertical(src)`   | Mirror each column top-to-bottom             |
| `rotate_90_cw(src)`    | Rotate 90° clockwise                         |
| `rotate_90_ccw(src)`   | Rotate 90° counter-clockwise                 |
| `rotate_180(src)`      | Rotate 180°                                  |
| `crop(src, x0, y0, w, h)` | Extract a rectangle (0-indexed)          |
| `pad(src, top, right, bottom, left, fill)` | Add a border           |

### Continuous

| Function                                     | Description                   |
|----------------------------------------------|-------------------------------|
| `scale(src, out_w, out_h, mode)`             | Resize to new dimensions      |
| `rotate(src, radians, mode, bounds)`         | Arbitrary-angle rotation      |
| `affine(src, matrix, out_w, out_h, mode, oob)` | 2×3 affine warp             |
| `perspective_warp(src, h, out_w, out_h, mode, oob)` | 3×3 homography warp  |

## Usage

```lua
local M  = require("coding_adventures.image_geometric_transforms")
local pc = require("coding_adventures.pixel_container")

local src = pc.new(640, 480)
-- ... fill src ...

-- Lossless ops
local flipped  = M.flip_horizontal(src)
local rotated  = M.rotate_90_cw(src)
local cropped  = M.crop(src, 100, 50, 200, 150)
local padded   = M.pad(src, 10, 10, 10, 10, {255, 255, 255, 255})

-- Continuous ops (bilinear by default)
local thumb    = M.scale(src, 160, 120)
local turned   = M.rotate(src, math.pi / 6, "bicubic", "fit")

-- Affine: 2x scale + 45° rotation
local cos45 = math.cos(math.pi / 4)
local sin45 = math.sin(math.pi / 4)
local matrix = {
    {2 * cos45, -2 * sin45, 0},
    {2 * sin45,  2 * cos45, 0},
}
local warped = M.affine(src, matrix, 640, 480, "bilinear", "zero")

-- Perspective: identity homography
local H = {{1,0,0},{0,1,0},{0,0,1}}
local persp = M.perspective_warp(src, H, 640, 480)
```

## Testing

```
cd tests && busted . --verbose --pattern=test_
```
