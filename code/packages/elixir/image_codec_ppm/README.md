# coding_adventures_image_codec_ppm

**IC02** — PPM (Portable PixMap) image encoder/decoder. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What It Does

Encodes and decodes P6 (binary PPM) image files. PPM is the simplest practical
image format: an ASCII header followed by raw RGB bytes. It has no alpha
channel, no compression, and no padding.

```elixir
alias CodingAdventures.{PixelContainer, ImageCodecPpm}

# Encode to PPM
c = PixelContainer.new(640, 480)
c = PixelContainer.fill_pixels(c, 135, 206, 235, 255)  # sky blue
ppm_data = ImageCodecPpm.encode(c)
File.write!("sky.ppm", ppm_data)

# Decode from PPM (alpha set to 255 on load)
{:ok, loaded} = ImageCodecPpm.decode(File.read!("sky.ppm"))
PixelContainer.pixel_at(loaded, 0, 0)   # => {135, 206, 235, 255}
```

## PPM Format

```
P6
<width> <height>
255
<raw RGB bytes, 3 bytes per pixel, no padding>
```

Comment lines (starting with `#`) can appear anywhere in the header and are
skipped during parsing.

## Alpha Handling

- **Encode**: alpha is dropped. PPM stores only R, G, B.
- **Decode**: alpha is set to 255 (fully opaque) for all pixels.

## Where It Fits

```
IC02 PPM codec  ← this package
  └── IC00 PixelContainer
```

## API

| Function | Description |
|---|---|
| `mime_type/0` | Returns `"image/x-portable-pixmap"` |
| `encode/1` | Encodes a `PixelContainer` to a P6 PPM binary |
| `decode/1` | Parses a P6 PPM binary into a `PixelContainer` |

## Running Tests

```bash
mix deps.get && mix test
```

## Version

0.1.0 — IC02 spec compliant.
