# coding-adventures-pixel-container

IC00: Universal RGBA8 pixel buffer and image codec interface. Zero dependencies.

## Usage

```python
from pixel_container import create_pixel_container, set_pixel, pixel_at, fill_pixels

c = create_pixel_container(4, 4)
fill_pixels(c, 255, 255, 255, 255)   # solid white
set_pixel(c, 1, 1, 255, 0, 0, 255)  # red dot at (1,1)

r, g, b, a = pixel_at(c, 1, 1)  # (255, 0, 0, 255)
```

## API

| Name | Description |
|------|-------------|
| `PixelContainer` | Dataclass: `width`, `height`, `data: bytearray` |
| `ImageCodec` | ABC: `mime_type`, `encode`, `decode` |
| `create_pixel_container(w, h)` | Factory — zeroed RGBA8 buffer |
| `pixel_at(c, x, y)` | Read pixel → `(r, g, b, a)`; `(0,0,0,0)` if OOB |
| `set_pixel(c, x, y, r, g, b, a)` | Write pixel; no-op if OOB |
| `fill_pixels(c, r, g, b, a)` | Flood fill entire buffer |
