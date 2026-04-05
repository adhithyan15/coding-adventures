# coding_adventures_image_codec_bmp

**IC01** — BMP (Windows Bitmap) image encoder/decoder. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What It Does

Encodes and decodes BMP files using 32-bit BGRA format with BI_BITFIELDS
compression. Implements the `CodingAdventures.ImageCodec` behaviour.

```elixir
alias CodingAdventures.{PixelContainer, ImageCodecBmp}

# Encode a pixel container to BMP binary
c = PixelContainer.new(320, 240)
c = PixelContainer.set_pixel(c, 160, 120, 255, 0, 0, 255)
bmp_data = ImageCodecBmp.encode(c)
File.write!("output.bmp", bmp_data)

# Decode a BMP binary back to a pixel container
{:ok, loaded} = ImageCodecBmp.decode(File.read!("output.bmp"))
PixelContainer.pixel_at(loaded, 160, 120)   # => {255, 0, 0, 255}
```

## Where It Fits

```
IC01 BMP codec  ← this package
  └── IC00 PixelContainer
```

## BMP Format Details

- **File header**: 14 bytes — signature "BM", file size, reserved, pixel offset
- **DIB header**: 40 bytes — BITMAPINFOHEADER with negative height (top-to-bottom)
- **Channel masks**: 12 bytes — BI_BITFIELDS for 32-bit BGRA layout
- **Pixel data**: 4 bytes/pixel, BGRA order (BMP stores Blue first)

Total fixed header: 66 bytes.

## API

| Function | Description |
|---|---|
| `mime_type/0` | Returns `"image/bmp"` |
| `encode/1` | Encodes a `PixelContainer` to a BMP binary |
| `decode/1` | Parses a BMP binary into a `PixelContainer` |
| `encode_bmp/1` | Same as `encode/1`, public for testing |
| `decode_bmp/1` | Same as `decode/1`, public for testing |

## Running Tests

```bash
mix deps.get && mix test
```

## Version

0.1.0 — IC01 spec compliant.
