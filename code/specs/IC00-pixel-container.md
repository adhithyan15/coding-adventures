# IC00 — `pixel-container`: The Universal Pixel Buffer

## Overview

This spec defines `pixel-container`, a zero-dependency crate that owns the
`PixelContainer` struct and the `ImageCodec` trait.

Every image encoder and decoder in this repo speaks `PixelContainer`. A GPU
renderer hands you a `PixelContainer`. A JPEG decoder hands you a
`PixelContainer`. You can then pass that buffer into a PNG encoder, a BMP
encoder, or directly to the OS display layer — without ever importing a single
paint type.

```
jpg_bytes  → image-codec-jpeg::decode() → PixelContainer
                                                │
                                                ▼
                                  image-codec-png::encode() → png_bytes
```

The key design rule: **codecs only know about pixels, not paint**. A codec has
no idea what a `PaintScene`, `PaintInstruction`, or coordinate space is. It
receives a flat RGBA8 buffer and serialises it, or deserialises bytes into one.

---

## Why a Separate Crate?

Before this crate existed, `PixelContainer` and `ImageCodec` lived in
`paint-instructions`. That forced every codec to depend on the full paint IR —
even a trivial PPM encoder had to import `PaintScene`, `PaintRect`, and all 10
instruction variants just to get the pixel buffer type.

The fix is straightforward:

```
pixel-container                (PixelContainer, ImageCodec — zero deps)
    ├── paint-instructions     (re-exports both; adds PaintScene etc.)
    │       └── paint-metal, paint-vm-svg, ...
    ├── image-codec-bmp        (pixel-container only)
    ├── image-codec-ppm        (pixel-container only)
    ├── image-codec-qoi        (pixel-container only)
    └── image-codec-png        (pixel-container + png crate)
```

Existing code that imports `paint_instructions::PixelContainer` keeps compiling
unchanged via the re-export. New codec crates can depend only on
`pixel-container` and remain completely isolated from rendering concepts.

---

## `PixelContainer`

### Fields

```rust
pub struct PixelContainer {
    pub width:  u32,      // image width in pixels
    pub height: u32,      // image height in pixels
    pub data:   Vec<u8>,  // RGBA8, row-major, top-left origin
}
```

### Pixel Layout

The buffer stores four bytes per pixel in **RGBA** order (red, green, blue,
alpha). Pixels are arranged left-to-right, top-to-bottom (row-major, top-left
origin):

```
offset = (y * width + x) * 4

data[offset + 0] = R
data[offset + 1] = G
data[offset + 2] = B
data[offset + 3] = A
```

A fully opaque red pixel at (x=2, y=1) in a 640×480 image:

```
offset = (1 * 640 + 2) * 4 = 2568
data[2568] = 0xFF  // R
data[2569] = 0x00  // G
data[2570] = 0x00  // B
data[2571] = 0xFF  // A
```

A fully transparent pixel has A=0; the RGB values are conventionally zero but
are not significant when A=0.

### Constructor API

```rust
impl PixelContainer {
    /// Create a blank (all-zero) buffer of the given dimensions.
    pub fn new(width: u32, height: u32) -> Self;

    /// Create from existing pixel data. Panics if data.len() != width * height * 4.
    pub fn from_data(width: u32, height: u32, data: Vec<u8>) -> Self;

    /// Read the RGBA components of the pixel at (x, y).
    /// Returns (0, 0, 0, 0) if (x, y) is out of bounds.
    pub fn pixel_at(&self, x: u32, y: u32) -> (u8, u8, u8, u8);

    /// Write the RGBA components of the pixel at (x, y).
    /// No-op if (x, y) is out of bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8);
}
```

---

## `ImageCodec` Trait

```rust
pub trait ImageCodec {
    /// The MIME type for this format, e.g. "image/png".
    fn mime_type(&self) -> &'static str;

    /// Encode a PixelContainer into raw bytes for this format.
    fn encode(&self, container: &PixelContainer) -> Vec<u8>;

    /// Decode raw bytes into a PixelContainer.
    /// Returns Err if the bytes are not valid for this format.
    fn decode(&self, bytes: &[u8]) -> Result<PixelContainer, String>;
}
```

All image codec crates implement this trait. This means you can store codecs
in a `Vec<Box<dyn ImageCodec>>`, swap encoders at runtime, or build a generic
format-conversion function:

```rust
fn transcode(
    input_bytes: &[u8],
    decoder: &dyn ImageCodec,
    encoder: &dyn ImageCodec,
) -> Result<Vec<u8>, String> {
    let pixels = decoder.decode(input_bytes)?;
    Ok(encoder.encode(&pixels))
}
```

---

## Codec Chaining Pattern

The canonical use case: convert between formats without a common dependency
on either codec:

```rust
use image_codec_jpeg::JpegCodec;
use image_codec_png::PngCodec;

let jpg_bytes = std::fs::read("photo.jpg").unwrap();
let pixel_container = JpegCodec.decode(&jpg_bytes).unwrap();
let png_bytes = PngCodec.encode(&pixel_container);
std::fs::write("photo.png", png_bytes).unwrap();
```

And from GPU rendering to a file:

```rust
use paint_metal::render;
use image_codec_png::PngCodec;

let scene = PaintScene::new(800, 600);
// ... add instructions ...
let pixels = render(&scene);           // PixelContainer from GPU
let png = PngCodec.encode(&pixels);   // serialise to PNG
std::fs::write("output.png", png).unwrap();
```

---

## IC Series Roadmap

| Spec | Package | Format | Codec | Notes |
|------|---------|--------|-------|-------|
| IC00 | `pixel-container` | — | Base types | This spec |
| IC01 | `image-codec-bmp` | BMP | Lossless | 54-byte header, BGRA swap |
| IC02 | `image-codec-ppm` | PPM P6 | Lossless | Simplest possible format |
| IC03 | `image-codec-qoi` | QOI | Lossless | Hash table + delta coding |
| IC04 | `image-codec-png` | PNG | Lossless | Deflate-compressed |
| IC05 | `image-codec-gif` | GIF | Lossless | LZW, palette, animation |
| IC06 | `image-codec-jpeg` | JPEG | Lossy | YCbCr, DCT, Huffman |
| IC07 | `image-codec-webp` | WebP | Both | VP8 (lossy) + VP8L (lossless) |
| IC08 | `image-codec-avif` | AVIF | Both | AV1-based, HDR, wide colour |

Each spec is self-contained. A codec only needs `pixel-container` — no shared
codec framework, no codec-to-codec dependencies.

---

## Crate Layout

```
code/packages/rust/pixel-container/
├── Cargo.toml           # name = "pixel-container", zero dependencies
├── src/
│   └── lib.rs           # PixelContainer + ImageCodec
├── BUILD
├── README.md
└── CHANGELOG.md
```

`Cargo.toml` declares no `[dependencies]`. The crate is intentionally minimal.
