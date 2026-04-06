# coding-adventures-image-codec-qoi

IC03: QOI (Quite OK Image) encoder and decoder for Python.

Part of the IC00–IC03 image codec series built on
`coding-adventures-pixel-container`.

## Usage

```python
from pixel_container import create_pixel_container, fill_pixels
from image_codec_qoi import QoiCodec, encode_qoi, decode_qoi

c = create_pixel_container(4, 4)
fill_pixels(c, 100, 150, 200, 255)

qoi_bytes = encode_qoi(c)
assert qoi_bytes[:4] == b"qoif"

c2 = decode_qoi(qoi_bytes)
```

## Format

14-byte header (`qoif` magic + width + height + channels + colorspace), followed
by a stream of operations:

| Op | Tag | Description |
|----|-----|-------------|
| OP_RGB | `0xFE` | Full RGB pixel (alpha unchanged) |
| OP_RGBA | `0xFF` | Full RGBA pixel |
| OP_INDEX | `00xxxxxx` | Reference previously seen pixel by hash |
| OP_DIFF | `01rrggbb` | Small RGB delta (±2 per channel) |
| OP_LUMA | `10gggggg` + byte | Medium green delta (±32), dr/db relative to dg |
| OP_RUN | `11rrrrrr` | Repeat previous pixel 1–62 times |

Hash: `(r*3 + g*5 + b*7 + a*11) % 64`

Ends with 8-byte marker: `00 00 00 00 00 00 00 01`.

## API

| Name | Description |
|------|-------------|
| `QoiCodec` | `ImageCodec` implementation for QOI |
| `encode_qoi(pixels)` | Encode `PixelContainer` → QOI bytes |
| `decode_qoi(data)` | Decode QOI bytes → `PixelContainer` |
