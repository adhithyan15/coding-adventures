# image-codec-arw

Sony ARW (Alpha RAW) RAW image codec for Rust.

## What is ARW?

ARW is Sony's proprietary RAW image format used in all Sony Alpha DSLRs and
mirrorless cameras since 2006. It stores the unprocessed sensor data —
before white balance, noise reduction, or colour processing — giving
photographers maximum control in post-production.

ARW is a TIFF 6.0 container extended with Sony-specific tags:
- **Make** tag = "SONY"
- **Model** tag = "DSLR-xxxx" or "ILCE-xxxx"
- **Raw data**: sub-IFD with `PhotometricInterpretation = 32803` (CFA/Bayer)
- **Compression**: 32767 (Sony-specific)
- **Bit depth**: 12-bit (ARW 1.0) or 14-bit (ARW 2.x+)

## Where it fits

```text
pixel-container          (RGBA8 pixel buffer)
paint-instructions       (ImageCodec trait)
image-codec-tiff         (TIFF IFD parser + colour pipeline)
image-codec-arw          (this crate — Sony-specific wrapper)
```

## Usage

```rust
use image_codec_arw::{decode_arw, encode_arw, ArwCodec};
use paint_instructions::ImageCodec;

// Decode
let arw_bytes = std::fs::read("photo.ARW").unwrap();
let pixels = decode_arw(&arw_bytes)?;
println!("Decoded {}×{} image", pixels.width, pixels.height);

// Encode (minimal, for round-trip testing)
let arw_out = encode_arw(&pixels);

// Via codec trait
let pixels2 = ArwCodec.decode(&arw_bytes)?;
let bytes2 = ArwCodec.encode(&pixels2);
```

## Version 0.1 scope

| Feature | Status |
|---|---|
| Uncompressed ARW (Compression=1) | Supported |
| Sony compressed (Compression=32767) | Err if TIFF decoder rejects it |
| Make tag validation | Rejects non-Sony files |
| White balance | D65 default (no MakerNote parse) |
| Colour matrix | Hardcoded Sony A7R II generic |
| Bayer demosaic | Bilinear RGGB (via image-codec-tiff) |
| sRGB gamma curve | Applied (via image-codec-tiff) |
| ARW 3.0 | Err if unsupported by TIFF decoder |

## Colour pipeline

```
Raw CFA pixels (12/14-bit)
  → black level subtraction (200 default, ARW 2.x)
  → white level normalisation (16383 = 2^14 - 1)
  → bilinear Bayer demosaic (RGGB)
  → white balance [1.0, 1.0, 1.0] (D65 neutral)
  → 3×3 colour matrix (Sony A7R II generic)
  → sRGB gamma curve
  → RGBA8 PixelContainer (A=255)
```

## Dependencies

- `pixel-container` — RGBA8 pixel buffer
- `paint-instructions` — `ImageCodec` trait
- `image-codec-tiff` — TIFF IFD parser, colour pipeline, strip decoder

## References

- dcraw.c by Dave Coffin — `sony_arw2_load_raw()` function
- LibRaw Sony decoders — https://github.com/LibRaw/LibRaw
- Exiv2 Sony tag database — https://exiv2.org/tags-sony.html
- rawspeed Sony decoders — https://github.com/darktable-org/rawspeed
