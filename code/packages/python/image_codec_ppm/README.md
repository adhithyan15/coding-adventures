# coding-adventures-image-codec-ppm

IC02: PPM P6 image encoder and decoder for Python.

Part of the IC00–IC03 image codec series built on
`coding-adventures-pixel-container`.

## Usage

```python
from pixel_container import create_pixel_container, set_pixel
from image_codec_ppm import PpmCodec, encode_ppm, decode_ppm

c = create_pixel_container(2, 1)
set_pixel(c, 0, 0, 255, 0, 0, 255)
set_pixel(c, 1, 0, 0, 255, 0, 255)

ppm_bytes = encode_ppm(c)
assert ppm_bytes.startswith(b"P6\n")

c2 = decode_ppm(ppm_bytes)
```

## Format

`P6\n<width> <height>\n255\n<RGB bytes>` — binary PPM, no alpha channel.

Alpha is dropped on encode; set to 255 on decode.

## API

| Name | Description |
|------|-------------|
| `PpmCodec` | `ImageCodec` implementation for PPM |
| `encode_ppm(pixels)` | Encode `PixelContainer` → PPM bytes |
| `decode_ppm(data)` | Decode PPM bytes → `PixelContainer` |
