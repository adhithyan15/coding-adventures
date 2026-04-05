# image-codec-bmp

BMP image encoder and decoder. Implements `ImageCodec` from `pixel-container`.

Encodes and decodes 32-bit BGRA BMP files with a 54-byte fixed header.
No compression. No external dependencies beyond `pixel-container`.

## Usage

```rust
use pixel_container::PixelContainer;
use image_codec_bmp::{encode_bmp, decode_bmp};

// Encode
let mut buf = PixelContainer::new(320, 240);
buf.fill(200, 100, 50, 255);
let bmp_bytes = encode_bmp(&buf);
std::fs::write("output.bmp", &bmp_bytes).unwrap();

// Decode
let bmp_bytes = std::fs::read("input.bmp").unwrap();
let pixels = decode_bmp(&bmp_bytes).unwrap();
println!("{}×{}", pixels.width, pixels.height);
```

## Format Details

- Header: 54 bytes (14-byte BITMAPFILEHEADER + 40-byte BITMAPINFOHEADER)
- Pixel data: BGRA order (R↔B swap from RGBA), top-down layout
- Negative `biHeight` signals top-down scanlines — no row reversal needed
- All integers: little-endian

See `code/specs/IC01-image-codec-bmp.md` for the full specification.
