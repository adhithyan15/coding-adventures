# coding-adventures-image-codec-bmp (Lua)

IC01: BMP (Windows Bitmap) image encoder and decoder — 32-bit RGBA, top-down,
uncompressed. Part of the image-codec stack built on `pixel-container` (IC00).

## What It Does

Encodes a `PixelContainer` into a valid 32-bit BMP binary string, and decodes a
32-bit BMP binary string back into a `PixelContainer`.

- No external dependencies (uses Lua 5.3+ `string.pack` / `string.unpack`)
- Handles top-down and bottom-up BMP files on decode
- Preserves alpha channel (BGRA byte order in the file)
- Validates magic bytes, bit depth, and data length on decode

## Stack Position

```
IC03 — image-codec-qoi
IC02 — image-codec-ppm
IC01 — image-codec-bmp  ← you are here
IC00 — pixel-container
```

## Quick Start

```lua
local pc  = require("coding_adventures.pixel_container")
local bmp = require("coding_adventures.image_codec_bmp")

-- Create a 4×4 red image
local img = pc.new(4, 4)
pc.fill_pixels(img, 255, 0, 0, 255)

-- Encode to BMP bytes
local bytes = bmp.encode_bmp(img)

-- Write to file
local f = io.open("red.bmp", "wb")
f:write(bytes)
f:close()

-- Decode back
local f2 = io.open("red.bmp", "rb")
local loaded = bmp.decode_bmp(f2:read("*a"))
f2:close()
```

## API Reference

| Function | Description |
|----------|-------------|
| `encode_bmp(c)` | Encode container → BMP binary string |
| `decode_bmp(data)` | Decode BMP binary string → container |
| `codec` | Table: `{ mime_type, encode, decode }` |

## BMP Format Summary

```
Offset  Size  Field
------  ----  -----
0       2     'BM' magic
2       4     file size (bytes)
6       4     reserved zeros
10      4     pixel data offset = 54
14      4     DIB header size = 40
18      4     width (pixels)
22      4     height (negative = top-down)
26      2     planes = 1
28      2     bits per pixel = 32
30      4     compression = 0 (BI_RGB)
34      4     pixel data size
54      ...   pixel data (BGRA bytes, row-major)
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) and Lua 5.4+.

## Dependencies

- Lua ≥ 5.4 (uses `string.pack` / `string.unpack`)
- `coding-adventures-pixel-container` ≥ 0.1.0

## License

MIT
