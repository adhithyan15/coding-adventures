# image-codec-jpeg

Baseline JPEG (JFIF SOF0) encoder and decoder in pure Rust. IC04 implementation.

## What it does

Encodes a `PixelContainer` (RGBA8 pixel buffer) into a complete JFIF/JPEG byte
stream, and decodes JFIF/JPEG bytes back into a `PixelContainer`.

## How it fits in the stack

```
paint-instructions  →  image-codec-jpeg  →  JPEG files
pixel-container     ↗
dsp-dct             ↗  (2-D DCT/IDCT)
```

`image-codec-jpeg` depends only on:
- `pixel-container` — for `PixelContainer` and `ImageCodec`
- `dsp-dct` — for the 2-D forward and inverse DCT

## Usage

```rust
use pixel_container::PixelContainer;
use image_codec_jpeg::{encode_jpeg, decode_jpeg, JpegCodec};
use pixel_container::ImageCodec;

// Encode at default quality (75)
let mut image = PixelContainer::new(640, 480);
// ... fill pixels ...
let jpeg_bytes = encode_jpeg(&image);

// Encode at a custom quality level
let codec = JpegCodec::new(90); // quality 1–100
let jpeg_bytes = codec.encode(&image);

// Decode
let decoded = decode_jpeg(&jpeg_bytes).expect("invalid JPEG");
println!("{}×{}", decoded.width, decoded.height);
```

## JPEG basics

JPEG compression has five stages:

1. **Colour transform** — RGB → YCbCr (separates brightness from colour)
2. **Block splitting** — 8×8 pixel blocks
3. **DCT** — converts pixel values to spatial frequencies
4. **Quantization** — divides frequencies by step sizes (lossy!)
5. **Huffman coding** — lossless entropy compression

Decoding reverses these steps exactly.

## Quality parameter

| Quality | Description                          |
|---------|--------------------------------------|
| 100     | Near-lossless (large files)          |
| 75      | Default — good balance (recommended) |
| 50      | Annex K base tables unchanged        |
| 1       | Maximum compression (visible artefacts) |

## Format support

- Encoder: always produces JFIF 1.1 with embedded Annex K Huffman tables
- Decoder: baseline sequential DCT (SOF0), 8-bit samples, 3 components, 4:4:4
- Does NOT support: progressive JPEG, arithmetic coding, 12-bit, CMYK, EXIF

## Round-trip accuracy

| Quality | Tolerance per channel |
|---------|----------------------|
| 75      | ±5                   |
| 100     | ±2–3                 |

Solid-colour images round-trip with minimal error. Gradient or textured images
may show larger per-pixel differences due to the block DCT approximation, but
the overall image quality is visually indistinguishable at quality ≥ 75.
