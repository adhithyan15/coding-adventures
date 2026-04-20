# image-geometric-transforms

**IMG04** — Geometric (spatial) transforms on the `PixelContainer` image type.

A *geometric transform* repositions pixels in 2-D space without changing their
colour values. This crate covers every operation defined in the IMG04
specification: exact-integer lossless transforms (flip, rotate-90, crop, pad)
and continuous resampled transforms (scale, free-angle rotate, affine warp,
perspective warp).

## Stack position

```
IMG04 image-geometric-transforms
 └── IC00 pixel-container   (PixelContainer — RGBA8 raster store)
```

This crate does **not** depend on `image` (the popular crate) — it works
directly with `PixelContainer` from IC00.  IMG03 (`image-point-ops`) is a
sibling; there is no dependency between them.

## Operations

### Lossless integer transforms

These map each output pixel to exactly one input pixel via an integer formula.
No colour-space conversion or interpolation is performed; raw RGBA bytes are
simply shuffled.

| Function          | Description                                               |
|-------------------|-----------------------------------------------------------|
| `flip_horizontal` | Mirror left ↔ right                                       |
| `flip_vertical`   | Mirror top ↔ bottom                                       |
| `rotate_90_cw`    | Rotate 90° clockwise (dimensions swap)                    |
| `rotate_90_ccw`   | Rotate 90° counter-clockwise (dimensions swap)            |
| `rotate_180`      | Rotate 180° (half-turn)                                   |
| `crop`            | Extract a rectangular sub-region                          |
| `pad`             | Add a solid-colour border                                 |

### Continuous resampled transforms

These use the **inverse warp** model: for each output pixel, compute the
fractional source coordinate and sample the input image at that location.
All interpolation operates in **linear light** (sRGB decoded to f32) to
avoid the systematic darkening that blending gamma-compressed values produces.

| Function            | Description                                             |
|---------------------|---------------------------------------------------------|
| `scale`             | Resize to arbitrary dimensions                          |
| `rotate`            | Rotate by any angle (radians); Fit or Crop canvas       |
| `affine`            | Apply a 2×3 inverse-warp affine matrix                  |
| `perspective_warp`  | Apply a 3×3 inverse-warp homogeneous matrix             |

### Interpolation modes

| Variant              | Quality         | Speed   | Notes                     |
|----------------------|-----------------|---------|---------------------------|
| `Interpolation::Nearest`  | Low (blocky)    | Fastest | Exact pixel values        |
| `Interpolation::Bilinear` | Medium (smooth) | Fast    | Slight blur at magnification |
| `Interpolation::Bicubic`  | High (sharp)    | Medium  | Catmull-Rom, minimal ringing |

### Out-of-bounds policies

| Variant                   | Behaviour                                     |
|---------------------------|-----------------------------------------------|
| `OutOfBounds::Zero`       | Transparent black outside the image            |
| `OutOfBounds::Replicate`  | Clamp to nearest edge pixel                   |
| `OutOfBounds::Reflect`    | Mirror-reflect around each edge               |
| `OutOfBounds::Wrap`       | Tile (modular wrap)                           |

## Usage

```rust
use pixel_container::PixelContainer;
use image_geometric_transforms::{
    flip_horizontal, flip_vertical, rotate_90_cw, crop, pad,
    scale, rotate, affine, perspective_warp,
    Interpolation, RotateBounds, OutOfBounds,
};

// Lossless: flip left-right
let flipped = flip_horizontal(&img);

// Scale up 2× with bilinear interpolation
let bigger = scale(&img, img.width * 2, img.height * 2, Interpolation::Bilinear);

// Rotate 45° clockwise, expanding canvas to fit the full image
let rotated = rotate(
    &img,
    std::f32::consts::FRAC_PI_4,
    Interpolation::Bicubic,
    RotateBounds::Fit,
);

// Affine identity (pass-through)
let identity: [[f32; 3]; 2] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
let warped = affine(&img, identity, img.width, img.height,
                   Interpolation::Bilinear, OutOfBounds::Replicate);

// Perspective warp (identity homography)
let h_identity: [[f32; 3]; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];
let perspective = perspective_warp(&img, h_identity, img.width, img.height,
                                   Interpolation::Bilinear, OutOfBounds::Zero);
```

## Design notes

**Inverse warp**: All continuous transforms iterate over output pixels and
compute the corresponding input coordinate (pull-based). The alternative
forward warp (push) leaves holes for non-injective mappings and is harder
to parallelise. Inverse warp fills every output pixel exactly once.

**Pixel-centre convention**: Following the OpenGL/Metal convention, pixel
(x, y) has its centre at (x+0.5, y+0.5). The scale function applies the
half-pixel correction `u = (x'+0.5)/sx - 0.5` so that the first and last
output pixels align with the first and last input pixels, not with the edges
of the array.

**Linear-light interpolation**: Blending sRGB-encoded values produces
systematically dark results because the gamma curve is non-linear. All
`Bilinear` and `Bicubic` operations decode pixel values to linear f32 first,
blend, then re-encode. The `Nearest` mode copies exact bytes (no decoding
needed since no averaging occurs).

**Catmull-Rom cubic kernel**: The bicubic mode uses the Keys (α=0.5)
variant of Catmull-Rom, which satisfies the partition-of-unity property
(weights sum to 1 for any fractional offset) and passes through data points
(interpolating, not approximating).

## Testing

```
cargo test -p image-geometric-transforms -- --nocapture
```
