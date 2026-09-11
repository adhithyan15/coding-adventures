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
| Adaptive (IMG09)| `adaptive_threshold_mean` — the one function here that looks at a neighbourhood, not just its own pixel; see below |

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

**`adaptive_threshold_mean` (IMG09)**: `threshold`/`threshold_luminance`
apply one scalar cutoff to every pixel — exact for a clean, evenly-lit
source, unusable on a real photo where a shadow across half the image
pushes that half below any single cutoff regardless of where it's set.
`adaptive_threshold_mean(src, window, t)` compares each pixel to the mean
luminance of its own local neighbourhood instead, computed in O(1) per
pixel via a summed-area table (Bradley & Roth, 2007) so the whole
operation stays O(width × height). See `IMG09-adaptive-threshold.md` for
the full design, including why this is the one function in the crate
that isn't a pure per-pixel-independent point operation.

## Testing

```
cargo test -p image-point-ops -- --nocapture
```
