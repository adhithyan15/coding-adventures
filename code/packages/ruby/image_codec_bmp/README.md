# CodingAdventures::ImageCodecBmp

BMP (Windows Bitmap) encoder and decoder. Ruby implementation of [IC01](../../../specs/IC01-image-codec-bmp.md).

Encodes RGBA8 `PixelContainer` buffers to 32-bit BMP files, and decodes BMP files back to `PixelContainer`.

## Quick Start

```ruby
$LOAD_PATH.unshift "path/to/pixel_container/lib"
require "coding_adventures/pixel_container"
require "coding_adventures/image_codec_bmp"

PC  = CodingAdventures::PixelContainer
BMP = CodingAdventures::ImageCodecBmp

# Create a 4×4 image
canvas = PC.create(4, 4)
PC.set_pixel(canvas, 0, 0, 255, 0,   0, 255)  # red at top-left
PC.set_pixel(canvas, 3, 3, 0,   0, 255, 255)  # blue at bottom-right

# Encode to BMP bytes
bmp_bytes = BMP.encode_bmp(canvas)
File.binwrite("output.bmp", bmp_bytes)

# Decode back
loaded = BMP.decode_bmp(File.binread("output.bmp"))
PC.pixel_at(loaded, 0, 0)  # => [255, 0, 0, 255]
```

## API

| Method | Description |
|---|---|
| `BMP.encode_bmp(container)` | Encode RGBA8 container to BMP binary string |
| `BMP.decode_bmp(data)` | Decode BMP binary string to RGBA8 container |
| `BMP::BmpCodec#encode(container)` | Same via codec object (implements `ImageCodec`) |
| `BMP::BmpCodec#decode(data)` | Same via codec object |
| `BMP::BmpCodec#mime_type` | Returns `"image/bmp"` |

## BMP Format

- Header: 54 bytes (14-byte BITMAPFILEHEADER + 40-byte BITMAPINFOHEADER)
- Pixel format: 32-bit BGRA (Blue, Green, Red, Alpha)
- Encoding produces top-down images (negative `biHeight`)
- Decoding handles both top-down and bottom-up orientations
- No compression (BI_RGB = 0); 32-bit only

## Errors

`decode_bmp` raises `ArgumentError` for:
- Data shorter than 54 bytes
- Bad `"BM"` magic
- Bit depth other than 32
- Non-zero compression flag
