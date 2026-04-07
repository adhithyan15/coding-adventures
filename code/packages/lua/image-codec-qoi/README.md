# coding-adventures-image-codec-qoi (Lua)

IC03: QOI (Quite OK Image format) encoder and decoder. All six ops implemented
with a 64-entry hash table, run-length encoding, and big-endian 14-byte header.

## What It Does

Encodes a `PixelContainer` into a QOI binary string, and decodes a QOI binary
string back into a `PixelContainer`.

- All six QOI operations: RUN, INDEX, DIFF, LUMA, RGB, RGBA
- 64-entry seen-pixels hash table: `(r*3 + g*5 + b*7 + a*11) % 64`
- Correct 8-bit signed delta wrapping for DIFF and LUMA
- 14-byte big-endian header with "qoif" magic
- 8-byte end-of-stream marker (00 00 00 00 00 00 00 01)
- No external dependencies

## Stack Position

```
IC03 — image-codec-qoi  ← you are here
IC02 — image-codec-ppm
IC01 — image-codec-bmp
IC00 — pixel-container
```

## Quick Start

```lua
local pc  = require("coding_adventures.pixel_container")
local qoi = require("coding_adventures.image_codec_qoi")

local img = pc.new(640, 480)
pc.fill_pixels(img, 0, 128, 255, 255)

local bytes = qoi.encode_qoi(img)

local f = io.open("output.qoi", "wb")
f:write(bytes)
f:close()

local f2 = io.open("output.qoi", "rb")
local img2 = qoi.decode_qoi(f2:read("*a"))
f2:close()
-- img2 is pixel-identical to img
```

## API Reference

| Function | Description |
|----------|-------------|
| `encode_qoi(c)` | Encode container → QOI binary string |
| `decode_qoi(data)` | Decode QOI string → container |
| `codec` | Table: `{ mime_type, encode, decode }` |

## QOI Operations

| Op | Tag | Condition |
|----|-----|-----------|
| RUN   | `11xxxxxx` | Same pixel as previous (up to 62 repeats) |
| INDEX | `00xxxxxx` | Pixel found in 64-entry seen table |
| DIFF  | `01xxxxxx` | dr,dg,db in [-2,1]; alpha unchanged |
| LUMA  | `10xxxxxx` | dg in [-32,31]; dr-dg,db-dg in [-8,7]; alpha unchanged |
| RGB   | `0xFE` + 3 bytes | RGB changed; alpha unchanged |
| RGBA  | `0xFF` + 4 bytes | RGBA all changed |

## Hash Formula

```lua
index = (r * 3 + g * 5 + b * 7 + a * 11) % 64
```

## Running Tests

```bash
cd tests && busted . --verbose --pattern=test_
```

Requires [busted](https://olivinelabs.com/busted/) and Lua 5.4+.

## Dependencies

- Lua ≥ 5.4 (uses `&`, `>>`, `<<`, `//` bitwise/integer operators)
- `coding-adventures-pixel-container` ≥ 0.1.0

## License

MIT
