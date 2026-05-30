# image-codec-nef

Nikon NEF (Nikon Electronic Format) RAW image codec for Rust.

## What is NEF?

NEF is Nikon's proprietary RAW image format used in all Nikon DSLRs and
mirrorless cameras. It stores the unprocessed sensor data — before white
balance, noise reduction, or any colour processing — giving photographers
maximum control in post-production.

NEF is a TIFF 6.0 container extended with Nikon-specific tags:
- **Make** tag = "NIKON" or "NIKON CORPORATION"
- **Raw data**: sub-IFD with `PhotometricInterpretation = 32803` (CFA/Bayer)
- **Bit depth**: 12-bit (older bodies) or 14-bit (D300 and newer)
- **Compression**: Uncompressed (1) or Nikon compressed (34713)

## Where it fits

```text
pixel-container          (RGBA8 pixel buffer)
paint-instructions       (ImageCodec trait)
image-codec-tiff         (TIFF IFD parser + colour pipeline)
image-codec-nef          (this crate — Nikon-specific wrapper)
```

## Usage

```rust
use image_codec_nef::{decode_nef, encode_nef, NefCodec};
use paint_instructions::ImageCodec;

// Decode
let nef_bytes = std::fs::read("photo.NEF").unwrap();
let pixels = decode_nef(&nef_bytes)?;
println!("Decoded {}×{} image", pixels.width, pixels.height);

// Encode (minimal, for round-trip testing)
let nef_out = encode_nef(&pixels);

// Via codec trait
let pixels2 = NefCodec.decode(&nef_bytes)?;
let bytes2 = NefCodec.encode(&pixels2);
```

## Version 0.1 scope

| Feature | Status |
|---|---|
| Uncompressed 12-bit NEF | Supported |
| Uncompressed 14-bit NEF | Supported |
| Nikon compressed (34713) | Returns descriptive Err |
| Make tag validation | Rejects non-Nikon files |
| White balance | D65 default (no MakerNote decrypt) |
| Colour matrix | Hardcoded Nikon D70 generic |
| Bayer demosaic | Bilinear RGGB (via image-codec-tiff) |
| sRGB gamma curve | Applied (via image-codec-tiff) |

## Colour pipeline

```
Raw CFA pixels (12-bit or 14-bit packed)
  → black level subtraction (0 default)
  → white level normalisation (4095 or 16383)
  → bilinear Bayer demosaic (RGGB)
  → white balance [1.0, 1.0, 1.0] (D65 neutral)
  → 3×3 colour matrix (Nikon D70 generic)
  → sRGB gamma curve
  → RGBA8 PixelContainer (A=255)
```

## Dependencies

- `pixel-container` — RGBA8 pixel buffer
- `paint-instructions` — `ImageCodec` trait
- `image-codec-tiff` — TIFF IFD parser, colour pipeline, strip decoder

## References

- dcraw.c by Dave Coffin — canonical NEF reference implementation
- LibRaw — https://github.com/LibRaw/LibRaw
- Exiv2 Nikon tag database — https://exiv2.org/tags-nikon.html
