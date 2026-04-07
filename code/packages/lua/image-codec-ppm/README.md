# coding-adventures-image-codec-ppm (Lua)

IC02: PPM (P6 Portable Pixmap) image encoder and decoder. RGB-only format —
alpha is dropped on encode and set to 255 on decode.

## What It Does

Encodes a `PixelContainer` into a P6 PPM binary string, and decodes a P6 PPM
binary string back into a `PixelContainer`.

- Plain-text ASCII header (`P6\n<width> <height>\n255\n`)
- Raw binary RGB pixels (3 bytes per pixel, no padding)
- Comment lines (`#`) in the header are silently skipped on decode
- Alpha is NOT stored: encode drops it, decode always sets A = 255
- No external dependencies

## Stack Position

```
IC03 — image-codec-qoi
IC02 — image-codec-ppm  ← you are here
IC01 — image-codec-bmp
IC00 — pixel-container
```

## Quick Start

```lua
local pc  = require("coding_adventures.pixel_container")
local ppm = require("coding_adventures.image_codec_ppm")

local img = pc.new(320, 240)
pc.fill_pixels(img, 0, 128, 255, 255)  -- solid cyan-ish

local bytes = ppm.encode_ppm(img)

local f = io.open("output.ppm", "wb")
f:write(bytes)
f:close()

local f2  = io.open("output.ppm", "rb")
local img2 = ppm.decode_ppm(f2:read("*a"))
f2:close()
-- img2.width == 320, img2.height == 240
-- every pixel has A = 255
```

## API Reference

| Function | Description |
|----------|-------------|
| `encode_ppm(c)` | Encode container → PPM binary string (no alpha) |
| `decode_ppm(data)` | Decode PPM string → container (A=255) |
| `codec` | Table: `{ mime_type, encode, decode }` |

## PPM P6 Format

```
P6\n
<width> <height>\n
255\n
<raw RGB bytes, width*height*3 bytes>
```

One byte per channel, three channels per pixel, no padding, no compression.

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) and Lua 5.4+.

## Dependencies

- Lua ≥ 5.4
- `coding-adventures-pixel-container` ≥ 0.1.0

## License

MIT
