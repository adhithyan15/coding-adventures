# image-codec-ppm

PPM P6 image encoder and decoder. Implements `ImageCodec` from `pixel-container`.

The simplest possible image format: ASCII header + raw RGB bytes. No compression,
no metadata, no external dependencies.

## Usage

```rust
use pixel_container::PixelContainer;
use image_codec_ppm::{encode_ppm, decode_ppm};

// Encode
let mut buf = PixelContainer::new(320, 240);
buf.fill(200, 100, 50, 255);
let ppm_bytes = encode_ppm(&buf);
std::fs::write("output.ppm", &ppm_bytes).unwrap();

// Decode
let ppm_bytes = std::fs::read("input.ppm").unwrap();
let pixels = decode_ppm(&ppm_bytes).unwrap();
println!("{}×{}", pixels.width, pixels.height);
```

## Alpha Handling

PPM has no alpha channel.

- **Encode**: alpha is dropped. Only RGB is written.
- **Decode**: all pixels get A = 255.

## Interoperability

```sh
# Convert to PNG with ImageMagick
convert output.ppm output.png

# Use as ffmpeg input
ffmpeg -i output.ppm output.mp4

# Netpbm
pnmtopng output.ppm > output.png
```

See `code/specs/IC02-image-codec-ppm.md` for the full specification.
