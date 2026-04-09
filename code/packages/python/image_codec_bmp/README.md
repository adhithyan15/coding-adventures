# coding-adventures-image-codec-bmp

IC01: BMP image encoder and decoder for Python.

Produces and reads 32-bit BGRA BMP files. Part of the IC00–IC03 image codec
series built on `coding-adventures-pixel-container`.

## Usage

```python
from pixel_container import create_pixel_container, set_pixel
from image_codec_bmp import BmpCodec, encode_bmp, decode_bmp

c = create_pixel_container(4, 4)
set_pixel(c, 0, 0, 255, 0, 0, 255)  # red dot at (0,0)

bmp_bytes = encode_bmp(c)
assert bmp_bytes[:2] == b"BM"

c2 = decode_bmp(bmp_bytes)
```

## Format

- 54-byte header (BITMAPFILEHEADER + BITMAPINFOHEADER)
- 32-bit BGRA pixel data, `biBitCount=32`, `biCompression=BI_RGB`
- Negative `biHeight` → top-down layout (row 0 in file = top of image)
- Decoder handles both top-down and bottom-up files

## API

| Name | Description |
|------|-------------|
| `BmpCodec` | `ImageCodec` implementation for BMP |
| `encode_bmp(pixels)` | Encode `PixelContainer` → BMP bytes |
| `decode_bmp(data)` | Decode BMP bytes → `PixelContainer` |
