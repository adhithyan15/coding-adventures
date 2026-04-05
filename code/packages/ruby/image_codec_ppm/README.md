# CodingAdventures::ImageCodecPpm

PPM (Portable Pixmap, P6 binary format) encoder and decoder. Ruby implementation of [IC02](../../../specs/IC02-image-codec-ppm.md).

PPM is the simplest raster format: a plain-text header followed by raw RGB bytes. Part of the [Netpbm](https://netpbm.sourceforge.net/) family.

## Quick Start

```ruby
$LOAD_PATH.unshift "path/to/pixel_container/lib"
require "coding_adventures/pixel_container"
require "coding_adventures/image_codec_ppm"

PC  = CodingAdventures::PixelContainer
PPM = CodingAdventures::ImageCodecPpm

canvas = PC.create(3, 3)
PC.set_pixel(canvas, 1, 1, 255, 255, 0, 255)  # yellow centre

ppm_bytes = PPM.encode_ppm(canvas)
File.binwrite("output.ppm", ppm_bytes)

loaded = PPM.decode_ppm(File.binread("output.ppm"))
PC.pixel_at(loaded, 1, 1)  # => [255, 255, 0, 255]
```

## API

| Method | Description |
|---|---|
| `PPM.encode_ppm(container)` | Encode RGBA8 container to P6 PPM binary string |
| `PPM.decode_ppm(data)` | Decode P6 PPM binary string to RGBA8 container |
| `PPM::PpmCodec#encode(container)` | Same via codec object |
| `PPM::PpmCodec#decode(data)` | Same via codec object |
| `PPM::PpmCodec#mime_type` | Returns `"image/x-portable-pixmap"` |

## PPM Format

```
P6
<width> <height>
255
<raw RGB bytes>
```

- Magic: `P6` (binary PPM)
- Pixels: 3 bytes per pixel (R, G, B), no alpha
- Max value: 255 (only supported value)
- Comments: lines starting with `#` in the header are skipped

## Alpha channel

- **Encode**: alpha is dropped; only R, G, B are written.
- **Decode**: all decoded pixels have A = 255 (fully opaque).

## Errors

`decode_ppm` raises `ArgumentError` for wrong magic (`P6` required), maxval other than 255, or truncated pixel data.
