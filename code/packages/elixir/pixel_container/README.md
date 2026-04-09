# coding_adventures_pixel_container

**IC00** — Fixed RGBA8 pixel buffer. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures) monorepo.

## What It Does

Provides the foundational in-memory pixel buffer used by all image codecs in
the IC series. Stores pixels in row-major RGBA8 format: each pixel is 4 bytes
(Red, Green, Blue, Alpha), laid out left-to-right, top-to-bottom.

Also defines the `CodingAdventures.ImageCodec` behaviour — the interface that
BMP, PPM, and QOI codecs implement.

```elixir
alias CodingAdventures.PixelContainer, as: PC

# Create a 320×240 buffer, all transparent black
c = PC.new(320, 240)

# Set a red opaque pixel at (10, 20)
c = PC.set_pixel(c, 10, 20, 255, 0, 0, 255)

# Read it back
PC.pixel_at(c, 10, 20)   # => {255, 0, 0, 255}

# Fill the whole canvas with a blue-grey
c = PC.fill_pixels(c, 100, 149, 237, 255)
```

## Where It Fits

```
IC03 QOI codec  ──┐
IC02 PPM codec  ──┤──▶  IC00 PixelContainer  ← this package
IC01 BMP codec  ──┘          └── ImageCodec behaviour
```

## API

| Function | Description |
|---|---|
| `new(width, height)` | Create a zeroed RGBA8 buffer |
| `pixel_at(c, x, y)` | Read `{r, g, b, a}` at `(x, y)`; OOB returns `{0,0,0,0}` |
| `set_pixel(c, x, y, r, g, b, a)` | Write a pixel; OOB is a no-op |
| `fill_pixels(c, r, g, b, a)` | Fill entire buffer with one color |
| `byte_size(c)` | Total bytes (`width * height * 4`) |

## Byte Layout

```
offset = (y * width + x) * 4

[byte 0] R
[byte 1] G
[byte 2] B
[byte 3] A
```

## Running Tests

```bash
mix test
```

## Version

0.1.0 — IC00 spec compliant.
