# image-point-ops

**IMG03** — Per-pixel point operations for the `PixelContainer` image type.

A *point operation* transforms each pixel independently using only that
pixel's value — no neighbourhood, no frequency domain, no geometry.  This
crate covers every operation from the IMG03 specification:

| Category        | Operations |
|-----------------|------------|
| u8-domain       | `invert`, `threshold`, `threshold_luminance`, `posterize`, `swap_rgb_bgr`, `extract_channel`, `brightness` |
| Linear-light    | `contrast`, `gamma`, `exposure`, `greyscale`, `sepia`, `colour_matrix`, `saturate`, `hue_rotate` |
| Colorspace      | `srgb_to_linear_image`, `linear_to_srgb_image` |
| LUT             | `apply_lut1d_u8`, `build_lut1d_u8`, `build_gamma_lut` |

## Stack position

```
IMG03 image-point-ops
 └── IC00 pixel-container   (PixelContainer — RGBA8 raster store)
```

This crate does **not** depend on `image` (the popular crate) — it works
directly with `PixelContainer` from IC00.

## Usage

```rust
use pixel_container::PixelContainer;
use image_point_ops::{invert, gamma, greyscale, GreyscaleMethod};

let mut img: PixelContainer = load_png("photo.png");

// Flip all RGB values (u8 — no colour-space conversion needed)
let inverted = invert(&img);

// Darken by γ = 0.5 (linear-light: decode → pow → re-encode)
let darkened = gamma(&img, 0.5);

// Convert to black-and-white using Rec. 709 luma weights
let bw = greyscale(&img, GreyscaleMethod::Rec709);
```

## Design notes

**sRGB ↔ linear round-trip**: operations that compute weighted averages
(contrast, gamma, exposure, colour matrix, greyscale, sepia, saturation,
hue rotation) must work in *linear light* to be physically correct.  The
crate maintains a lazy 256-entry `SRGB_TO_LINEAR` decode LUT; encoding
uses the analytic formula.  See IMG00 §2 and IMG03 §1 for the full
rationale.

**u8-domain operations**: `invert`, `threshold`, `posterize`, and
channel-manipulation ops are exactly correct in sRGB because they are
monotone remappings that do not mix channel values.  These skip the
decode/encode round-trip entirely.

## Testing

```
cargo test -p image-point-ops -- --nocapture
```
