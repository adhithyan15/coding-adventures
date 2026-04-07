# CodingAdventures::ImageCodecQoi

QOI (Quite OK Image Format) encoder and decoder. Ruby implementation of [IC03](../../../specs/IC03-image-codec-qoi.md).

QOI is a lossless image format that is extremely simple to implement — the entire spec fits on one page — while achieving 20–50% compression over raw pixels.

## Quick Start

```ruby
$LOAD_PATH.unshift "path/to/pixel_container/lib"
require "coding_adventures/pixel_container"
require "coding_adventures/image_codec_qoi"

PC  = CodingAdventures::PixelContainer
QOI = CodingAdventures::ImageCodecQoi

canvas = PC.create(256, 256)
PC.fill_pixels(canvas, 100, 149, 237, 255)  # cornflower blue

qoi_bytes = QOI.encode_qoi(canvas)
File.binwrite("output.qoi", qoi_bytes)

loaded = QOI.decode_qoi(File.binread("output.qoi"))
PC.pixel_at(loaded, 0, 0)  # => [100, 149, 237, 255]
```

## API

| Method | Description |
|---|---|
| `QOI.encode_qoi(container)` | Encode RGBA8 container to QOI binary string |
| `QOI.decode_qoi(data)` | Decode QOI binary string to RGBA8 container |
| `QOI.pixel_hash(r, g, b, a)` | Running-array index: `(r*3 + g*5 + b*7 + a*11) % 64` |
| `QOI.wrap(delta)` | Signed byte delta: `((delta & 0xFF) + 128) & 0xFF - 128` |
| `QOI::QoiCodec#encode(container)` | Same via codec object |
| `QOI::QoiCodec#decode(data)` | Same via codec object |
| `QOI::QoiCodec#mime_type` | Returns `"image/qoi"` |

## Chunk Types

| Op | Tag bits | Size | Description |
|---|---|---|---|
| QOI_OP_RUN | `11xxxxxx` | 1 byte | Run of 1–62 identical pixels (6-bit length, bias -1) |
| QOI_OP_INDEX | `00xxxxxx` | 1 byte | Reference to running array slot 0–63 |
| QOI_OP_DIFF | `01rdgdbd` | 1 byte | Per-channel delta -2..1 (2 bits each, bias -2) |
| QOI_OP_LUMA | `10gggggg` + byte | 2 bytes | Green delta -32..31; dr-dg and db-dg in -8..7 |
| QOI_OP_RGB | `11111110` + 3 bytes | 4 bytes | Full RGB, alpha unchanged |
| QOI_OP_RGBA | `11111111` + 4 bytes | 5 bytes | Full RGBA |

## File Structure

```
"qoif"          4 bytes  magic
width           4 bytes  uint32 big-endian
height          4 bytes  uint32 big-endian
channels        1 byte   4 (RGBA)
colorspace      1 byte   0 (sRGB)
<chunk stream>  variable
[0,0,0,0,0,0,0,1]  8 bytes  end marker
```
