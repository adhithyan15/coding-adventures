# paint-codec-png

PNG image codec for the paint-instructions pixel pipeline.

Takes a `PixelContainer` and encodes it as a PNG file. Implements the
`ImageCodec` trait from `paint-instructions`. Uses the workspace `png` crate —
no external supply-chain dependencies.

## Usage

```rust
use paint_instructions::{PixelContainer, ImageCodec};
use paint_codec_png::PngCodec;

// Encode
let pixels: PixelContainer = paint_metal::render(&scene);
let png_bytes: Vec<u8> = PngCodec.encode(&pixels);
std::fs::write("output.png", &png_bytes).unwrap();

// Convenience function
let png_bytes = paint_codec_png::encode_png(&pixels);
paint_codec_png::write_png(&pixels, "output.png").unwrap();
```

## Current status

| Operation | Status |
|---|---|
| Encode (`PixelContainer → Vec<u8>`) | Fully implemented |
| Decode (`Vec<u8> → PixelContainer`) | Returns `Err` — requires inflate, which is not yet in the workspace `deflate` crate |

Encoding is complete and used in production (barcode → Metal render → PNG pipeline).
Decoding will be added when the `deflate` crate gains inflate support.

## Pipeline position

```text
paint-metal::render(&scene)          → PixelContainer
paint_codec_png::encode_png(&pixels) → Vec<u8> (PNG file)
std::fs::write("qr.png", bytes)
```

## Spec

P2D00 — `code/specs/P2D00-paint-instructions.md` (codec layer description)
