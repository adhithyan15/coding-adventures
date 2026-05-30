# image-codec-dng

Adobe DNG (Digital Negative) image codec for the coding-adventures monorepo.
Decodes DNG RAW camera files to `PixelContainer` (RGBA8) and provides minimal
DNG encoding for round-trip tests.

## Overview

DNG is an open RAW image format created by Adobe in 2004, now at version 1.6.
It is a strict superset of TIFF 6.0: every DNG file is a valid TIFF file,
extended with private tags (IDs 50700–51100+) that carry camera colour
calibration data.

**Why DNG?**
- Open specification — no reverse engineering required
- Many cameras output DNG natively (Google Pixel, Leica, Hasselblad, Pentax)
- Adobe Lightroom, ACR, and darktable can convert proprietary RAWs to DNG
- DNG embeds its own colour calibration data, making correct colour easy

## Architecture

This crate is a thin shim over `image-codec-tiff`. The TIFF decoder already
handles IFD parsing, strip decompression, Bayer demosaicing, and the colour
pipeline. The DNG layer extracts calibration tags and feeds them to
`decode_tiff_with_opts`.

```text
image-codec-dng/
  src/
    lib.rs       Public API, DngCodec, VERSION
    tags.rs      DNG private tag constants (50706–50880)
    color.rs     WB from AsShotNeutral, matrix math, XYZ D50 → sRGB
    decoder.rs   Find raw IFD, extract tags, call decode_tiff_with_opts
    encoder.rs   Minimal synthetic DNG writer (encode as TIFF)
```

## How it fits in the stack

```
IC00: pixel-container  ← holds decoded RGBA8 pixels
IC09: image-codec-tiff ← TIFF IFD parser + colour pipeline (dependency)
IC10: image-codec-dng  ← DNG calibration tag extractor (this crate)
```

## Usage

```rust
use image_codec_dng::{decode_dng, encode_dng, DngCodec, VERSION};
use paint_instructions::ImageCodec;
use pixel_container::PixelContainer;

// Decode a DNG file
let dng_bytes = std::fs::read("photo.dng").unwrap();
let pixels = decode_dng(&dng_bytes)?;
println!("{}×{} image decoded", pixels.width, pixels.height);

// Use the codec trait (for plug-in pipeline use)
let codec: &dyn ImageCodec = &DngCodec;
println!("MIME type: {}", codec.mime_type()); // "image/x-adobe-dng"

// Round-trip (encode → decode)
let mut pc = PixelContainer::new(4, 4);
pc.fill(200, 150, 100, 255);
let encoded = encode_dng(&pc);
let decoded = decode_dng(&encoded).unwrap();
assert_eq!(decoded.width, 4);

println!("Version: {}", VERSION); // "0.1.0"
```

## Colour pipeline

1. Parse IFD chain, find RAW IFD (NewSubfileType=0, photometric=CFA/LinearRaw)
2. Extract AsShotNeutral → white balance multipliers
3. Extract ForwardMatrix1 (preferred) or ColorMatrix1 (fallback, inverted)
4. Call `decode_tiff_with_opts` with black level, white level, WB, and matrix
5. The TIFF decoder applies: black-level subtraction → white normalisation →
   Bayer demosaicing → WB → colour matrix → sRGB gamma → u8 clamp → RGBA8

## DNG private tags

| Tag   | Name                    | Type         | Purpose                         |
|-------|-------------------------|--------------|---------------------------------|
| 50706 | DNGVersion              | BYTE[4]      | DNG spec version (e.g. 1.6.0.0)|
| 50708 | UniqueCameraModel       | ASCII        | Camera identifier               |
| 50714 | BlackLevel              | RATIONAL+    | Sensor black floor              |
| 50717 | WhiteLevel              | SHORT/LONG   | Sensor saturation ceiling       |
| 50721 | ColorMatrix1            | SRATIONAL[9] | XYZ D50 → camera (illuminant 1)|
| 50728 | AsShotNeutral           | RATIONAL[3]  | White balance neutrals          |
| 50829 | ActiveArea              | LONG[4]      | Valid sensor region             |
| 50879 | ForwardMatrix1          | SRATIONAL[9] | Camera → XYZ D50 (preferred)   |

## Version

0.1.0

## Depends on

- `pixel-container` (IC00) — RGBA8 pixel buffer
- `paint-instructions` (IC01) — `ImageCodec` trait
- `image-codec-tiff` (IC09) — TIFF decoding engine
