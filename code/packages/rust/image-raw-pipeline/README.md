# image-raw-pipeline

Shared RAW colour development pipeline for all camera RAW image codecs.

Extracts the sRGB gamma function, 3×3 matrix operations, and the four-stage
RAW colour development pipeline that were previously duplicated across
`image-codec-tiff`, `image-codec-raf`, and `image-codec-rw2`.

## API

```rust
use image_raw_pipeline::{
    srgb_gamma, srgb_decode,
    mat3x3_mul, invert_3x3,
    apply_color_pipeline,
};

// sRGB gamma (IEC 61966-2-1): linear light → display encoding.
let display = srgb_gamma(0.5); // ≈ 0.735

// Inverse: display encoding → linear light.
let linear = srgb_decode(display); // ≈ 0.5

// 3×3 × column-vector (per-pixel, no heap allocation).
let cam_to_srgb = [[1.392, -0.418, 0.026],
                   [-0.254, 1.614, -0.360],
                   [0.068, -0.584, 1.516]];
let rgb = mat3x3_mul(&cam_to_srgb, [0.5, 0.5, 0.5]);

// 3×3 inversion (per-file, used by DNG ForwardMatrix path).
let inv = invert_3x3(&cam_to_srgb).expect("matrix should be invertible");

// Full RAW pipeline: normalize → white balance → colour matrix → gamma.
let raw_pixels: Vec<(u16, u16, u16)> = vec![(40000, 20000, 18000)];
let srgb = apply_color_pipeline(
    &raw_pixels,
    512,        // black level
    4095,       // white level (12-bit sensor)
    [2.1, 1.0, 1.7], // daylight white balance
    cam_to_srgb,
);
// → Vec<(u8, u8, u8)> in sRGB
```

## Four-stage pipeline

```
raw u16 sensor values
  │
  ▼ 1. Black-level subtraction + normalization  → [0.0, 1.0]
  │
  ▼ 2. White balance (multiply per channel)
  │
  ▼ 3. Camera-to-sRGB colour matrix (3×3 f64)
  │
  ▼ 4. sRGB gamma (IEC 61966-2-1)
  │
  ▼ u8 sRGB output
```

## Zero dependencies

All math is elementary scalar arithmetic on stack-allocated arrays.
No heap allocation per pixel in `mat3x3_mul` or `apply_color_pipeline`.

## Spec

See [`code/specs/IMG07-image-raw-pipeline.md`](../../../../specs/IMG07-image-raw-pipeline.md).

## Consumers

| Crate | Uses |
|-------|------|
| `image-codec-tiff` | `apply_color_pipeline`, `srgb_gamma` |
| `image-codec-raf`  | `apply_color_pipeline`, `srgb_gamma` |
| `image-codec-rw2`  | `apply_color_pipeline`, `srgb_gamma` |
| `image-codec-dng`  | `invert_3x3`, `mat3x3_mul` |
