# CodingAdventures::PixelContainer

A fixed RGBA8 pixel buffer for image codec packages. Ruby implementation of [IC00](../../../specs/IC00-pixel-container.md).

This is the shared data structure used by all image codec packages (BMP, PPM, QOI). A `Container` holds a rectangular grid of pixels stored as raw RGBA bytes in a binary String.

## Quick Start

```ruby
require_relative "lib/coding_adventures/pixel_container"

PC = CodingAdventures::PixelContainer

# Create a 640×480 zeroed (transparent black) image
canvas = PC.create(640, 480)

# Write a red pixel at (10, 20)
PC.set_pixel(canvas, 10, 20, 255, 0, 0, 255)

# Read it back
r, g, b, a = PC.pixel_at(canvas, 10, 20)
# => [255, 0, 0, 255]

# Fill the entire canvas with opaque white
PC.fill_pixels(canvas, 255, 255, 255, 255)
```

## API

| Method | Description |
|---|---|
| `PC.create(width, height)` | Allocate a zeroed RGBA8 buffer |
| `PC.pixel_at(container, x, y)` | Returns `[r, g, b, a]`; `[0,0,0,0]` if OOB |
| `PC.set_pixel(container, x, y, r, g, b, a)` | Write one pixel; no-op if OOB |
| `PC.fill_pixels(container, r, g, b, a)` | Set every pixel to the given colour |

### Container struct

| Field | Type | Description |
|---|---|---|
| `width` | Integer | Number of columns |
| `height` | Integer | Number of rows |
| `data` | String (BINARY) | Raw pixel bytes, `width * height * 4` bytes |

Helper methods on the struct: `pixel_count`, `byte_count`, `to_s`.

## Memory Layout

```
offset(x, y) = (y * width + x) * 4
```

Bytes at that offset: `[R, G, B, A]`.

## Design Notes

- `data` uses `Encoding::ASCII_8BIT` (binary) to avoid Ruby encoding overhead.
- `String#getbyte` / `String#setbyte` provide O(1) byte access with no allocations.
- Out-of-bounds coordinates are silently ignored (read returns `[0,0,0,0]`, write is a no-op).
- Channel values passed to `set_pixel` are masked with `& 0xFF` to prevent buffer corruption.
