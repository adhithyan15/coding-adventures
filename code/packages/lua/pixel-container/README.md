# coding-adventures-pixel-container (Lua)

IC00: Fixed RGBA8 pixel buffer — the foundational in-memory image representation
for the image-codec stack.

## What It Does

A `PixelContainer` is a flat array of bytes representing a raster image:

- **4 bytes per pixel**: Red, Green, Blue, Alpha (each 0–255)
- **Row-major order**: pixels laid out left-to-right, then top-to-bottom
- **0-indexed coordinates**: pixel (0, 0) is top-left; (width-1, height-1) is bottom-right
- **Bounds-safe reads/writes**: `pixel_at` returns `0,0,0,0` outside bounds; `set_pixel` is a no-op

## Stack Position

```
IC03 — image-codec-qoi  (encodes/decodes QOI files)
IC02 — image-codec-ppm  (encodes/decodes PPM files)
IC01 — image-codec-bmp  (encodes/decodes BMP files)
IC00 — pixel-container  ← you are here (data model)
```

## Quick Start

```lua
local pc = require("coding_adventures.pixel_container")

-- Create a 320x240 RGBA image (all transparent black)
local img = pc.new(320, 240)

-- Fill with solid red
pc.fill_pixels(img, 255, 0, 0, 255)

-- Draw a white pixel at (100, 50)
pc.set_pixel(img, 100, 50, 255, 255, 255, 255)

-- Read a pixel back
local r, g, b, a = pc.pixel_at(img, 100, 50)
-- r=255, g=255, b=255, a=255

-- Out-of-bounds reads are safe
local r2, g2, b2, a2 = pc.pixel_at(img, 9999, 9999)
-- r2=0, g2=0, b2=0, a2=0

-- Clone and compare
local copy = pc.clone(img)
print(pc.equals(img, copy))  -- true
```

## API Reference

| Function | Description |
|----------|-------------|
| `new(width, height)` | Create blank RGBA8 container (all zeros) |
| `pixel_at(c, x, y)` | Read pixel → `r, g, b, a`; returns 0s if OOB |
| `set_pixel(c, x, y, r, g, b, a)` | Write pixel; no-op if OOB |
| `fill_pixels(c, r, g, b, a)` | Fill every pixel with one RGBA value |
| `clone(c)` | Deep copy of the container |
| `equals(a, b)` | True if same dimensions and identical pixels |

## Memory Layout

```
Pixel (x, y) starts at data index:  (y * width + x) * 4 + 1
  data[i+0] = Red
  data[i+1] = Green
  data[i+2] = Blue
  data[i+3] = Alpha
```

The `+1` is the Lua 1-indexing adjustment. All codecs (BMP, PPM, QOI) build
on this layout.

## ImageCodec Interface Convention

Codecs built on top of this container follow this shape:

```lua
codec.mime_type         -- string, e.g. "image/bmp"
codec.encode(container) -- returns binary string
codec.decode(string)    -- returns container or error(msg)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) and Lua 5.4+.

## Dependencies

- Lua ≥ 5.4
- No external Lua dependencies

## License

MIT
