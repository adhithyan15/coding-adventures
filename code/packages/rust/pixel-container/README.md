# pixel-container

Zero-dependency crate that owns `PixelContainer` and `ImageCodec` — the two
types every image encoder and decoder needs.

## What It Is

`PixelContainer` is a flat RGBA8 pixel buffer: width, height, and a `Vec<u8>`
of raw pixel data in row-major order with top-left origin.

`ImageCodec` is a trait with three methods: `mime_type`, `encode`, and
`decode`. Any image format (BMP, PPM, QOI, PNG, JPEG…) implements this trait.

## Why It Exists

Without this crate, `PixelContainer` lived in `paint-instructions`, which
forced every codec to depend on the full paint IR. A PPM encoder had no
business knowing about `PaintScene` or `PaintInstruction`.

With this crate, the dependency graph is clean:

```
pixel-container          (PixelContainer, ImageCodec — zero deps)
    ├── paint-instructions   (re-exports both; adds PaintScene etc.)
    ├── image-codec-bmp      (pixel-container only)
    ├── image-codec-ppm      (pixel-container only)
    ├── image-codec-qoi      (pixel-container only)
    └── image-codec-png      (pixel-container + png crate)
```

## Usage

```rust
use pixel_container::{PixelContainer, ImageCodec};

// Create a 4×4 black canvas.
let mut buf = PixelContainer::new(4, 4);

// Set a single pixel.
buf.set_pixel(2, 1, 255, 0, 0, 255); // opaque red at (2,1)

// Read it back.
assert_eq!(buf.pixel_at(2, 1), (255, 0, 0, 255));

// Fill the whole canvas.
buf.fill(255, 255, 255, 255); // white
```

### Codec Chaining

```rust
use image_codec_jpeg::JpegCodec;
use image_codec_png::PngCodec;

let jpg = std::fs::read("photo.jpg").unwrap();
let pixels = JpegCodec.decode(&jpg).unwrap();   // PixelContainer
let png    = PngCodec.encode(&pixels);           // Vec<u8>
std::fs::write("photo.png", png).unwrap();
```

## Pixel Layout

```
offset = (y * width + x) * 4

data[offset + 0] = R
data[offset + 1] = G
data[offset + 2] = B
data[offset + 3] = A
```

RGBA8 means 8 bits per channel, 4 channels, 4 bytes per pixel. Alpha = 255
is fully opaque; alpha = 0 is fully transparent.

## Part of the IC Series

| Spec | Package |
|------|---------|
| IC00 | `pixel-container` (this crate) |
| IC01 | `image-codec-bmp` |
| IC02 | `image-codec-ppm` |
| IC03 | `image-codec-qoi` |
| IC04 | `image-codec-png` |
| IC06 | `image-codec-jpeg` |

See `code/specs/IC00-pixel-container.md` for the full specification.
