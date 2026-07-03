# image-codec-orf

Olympus ORF (Olympus RAW Format) image codec for the coding-adventures monorepo.

## What is ORF?

ORF is the proprietary RAW image format used by Olympus (now OM System)
interchangeable-lens cameras since the E-1 (2003). It is a TIFF 6.0
container with Olympus-specific MakerNote extensions.

This crate builds on `image-codec-tiff` (IC09) and adds ORF-specific handling:

- **IIRO magic variant** — Olympus cameras sometimes write non-standard TIFF
  magic bytes (`IIRO` instead of `II`+42). This crate normalises them.
- **Make tag validation** — Only files from Olympus / OM Digital are accepted.
- **CFA IFD selection** — Finds the full-resolution Bayer image IFD.
- **Olympus colour pipeline** — Black level 256, white level 4095, and an
  E-M1 Mark II colour matrix are applied during decode.

## How it fits in the stack

```
pixel-container  ← raw pixel storage
paint-instructions ← ImageCodec trait
image-codec-tiff ← TIFF parsing / colour pipeline
image-codec-orf  ← ORF-specific wrapping (this crate)
```

## Usage

```rust
use image_codec_orf::{decode_orf, encode_orf, OrfCodec};
use paint_instructions::ImageCodec;

// Decode
let bytes = std::fs::read("DSC00001.ORF").unwrap();
let pixels = decode_orf(&bytes).unwrap();
println!("{}×{} image", pixels.width, pixels.height);

// Encode (uncompressed TIFF, suitable for testing)
let encoded = encode_orf(&pixels);

// Via codec trait
let codec = OrfCodec;
let pixels2 = codec.decode(&bytes).unwrap();
assert_eq!(codec.mime_type(), "image/x-olympus-orf");
```

## Compression support

| Compression  | Code  | Status  |
|--------------|-------|---------|
| Uncompressed | 1     | Full    |
| Olympus RLE  | 32767 | v0.2+   |

The Olympus proprietary 12-bit RLE (Compression=32767) is not yet implemented.
Files that use it will return a clear error message. The test encoder always
writes Compression=1.

## Colour constants

```rust
pub const OLYMPUS_COLOR_MATRIX: [[f64; 3]; 3] = [
    [ 1.476, -0.490,  0.014],
    [-0.254,  1.619, -0.365],
    [ 0.069, -0.497,  1.428],
];
pub const OLYMPUS_BLACK_LEVEL: u32 = 256;
pub const OLYMPUS_WHITE_LEVEL: u32 = 4095;
```

## Version

0.1.0 — initial release, uncompressed ORF decode/encode.
