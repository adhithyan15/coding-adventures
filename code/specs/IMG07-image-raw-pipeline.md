# IMG07 — Shared RAW Colour Pipeline

## Overview

Camera RAW formats (TIFF, DNG, CR2, NEF, ARW, RAF, ORF, RW2) all require
the same four-stage colour pipeline to convert raw sensor values into a
displayable sRGB image:

```
raw u16 sensor values
  │
  ▼ 1. Black-level subtraction + normalization  [0, white_level] → [0.0, 1.0]
  │
  ▼ 2. White balance                            multiply per channel
  │
  ▼ 3. Camera-to-sRGB colour matrix             3×3 f64 multiply
  │
  ▼ 4. sRGB gamma (IEC 61966-2-1)              linear light → display encoding
  │
  ▼ u8 sRGB output (clamp, scale, round)
```

Before this crate existed, the sRGB gamma formula and full pipeline were
duplicated in `image-codec-tiff/src/color.rs`, `image-codec-raf/src/color.rs`,
and `image-codec-rw2/src/color.rs`. IMG07 extracts them into one canonical
implementation that all RAW codec crates depend on.

Additionally, `image-codec-dng` needed to invert a 3×3 matrix to convert
between DNG's ForwardMatrix (camera→XYZ) and the ColorMatrix (XYZ→camera)
paths. That `invert_3x3` and the underlying `mat3x3_mul` primitive now live
here too.

## Motivation: why a shared crate?

The IEC 61966-2-1 transfer function has specific threshold constants
(0.0031308, 0.04045, 12.92, 1.055, 0.055, 2.4). Any of these could be
slightly wrong in a hand-typed duplicate. Extracting to one place:

- eliminates three copies of the same arithmetic
- gives a single test harness for sRGB correctness
- makes it trivial to add future RAW codecs without re-implementing gamma

## Function specifications

### `srgb_gamma(linear: f64) -> f64`

Apply the sRGB EOTF (electro-optical transfer function) — converts a
linear-light scalar in [0, 1] to a display-encoded scalar in [0, 1].

The IEC 61966-2-1 standard (sRGB) specifies:

```
V = 12.92 × L                     if L ≤ 0.0031308  (linear segment)
V = 1.055 × L^(1/2.4) − 0.055    if L > 0.0031308  (power segment)
```

Both endpoints are fixed points: `srgb_gamma(0.0) == 0.0` and
`srgb_gamma(1.0) == 1.0`.

### `srgb_decode(encoded: f64) -> f64`

Inverse of `srgb_gamma` — converts a display-encoded value back to
linear light. Defined by IEC 61966-2-1 as:

```
L = V / 12.92                     if V ≤ 0.04045
L = ((V + 0.055) / 1.055)^2.4    if V > 0.04045
```

Round-trip property: `srgb_decode(srgb_gamma(x)) ≈ x` within float
rounding error for `x ∈ [0, 1]`.

### `mat3x3_mul(m: &[[f64;3];3], v: [f64;3]) -> [f64;3]`

Multiply a 3×3 row-major matrix by a column vector.

```
[out[0]]   [m[0][0]  m[0][1]  m[0][2]]   [v[0]]
[out[1]] = [m[1][0]  m[1][1]  m[1][2]] × [v[1]]
[out[2]]   [m[2][0]  m[2][1]  m[2][2]]   [v[2]]
```

This is 9 multiplications and 6 additions — fast enough to run per pixel
without any heap allocation. Used in `apply_color_pipeline` for the
camera-to-sRGB matrix step, and available for DNG's ForwardMatrix path.

### `invert_3x3(m: &[[f64;3];3]) -> Option<[[f64;3];3]>`

Analytically invert a 3×3 matrix via Cramer's rule (cofactor expansion).

Returns `None` if `|det(M)| < 1e-12` (singular or near-singular matrix).

The inverse satisfies `mat3x3_mul(&inv, mat3x3_mul(&m, v)) ≈ v` for any
column vector `v`, up to floating-point rounding.

Used by `image-codec-dng` to convert between the ForwardMatrix (camera →
XYZ D50) and ColorMatrix (XYZ → camera) representations when only one is
available in the DNG metadata.

### `apply_color_pipeline`

```rust
pub fn apply_color_pipeline(
    pixels:       &[(u16, u16, u16)],
    black_level:  u32,
    white_level:  u32,
    wb:           [f64; 3],
    color_matrix: [[f64; 3]; 3],
) -> Vec<(u8, u8, u8)>
```

Apply the full four-stage RAW development pipeline to a slice of 16-bit
linear RGB triples (one per pixel, from demosaicing or direct multi-channel
reads):

**Stage 1 — Normalize.**

```
r_norm = (r_raw.saturating_sub(black_level) as f64) / effective_white
```

`effective_white = white_level as f64 - black_level as f64` (clamped to 1.0
to avoid division by zero). After subtraction, values are clamped to [0.0, 1.0].

**Stage 2 — White balance.**

```
r_wb = r_norm * wb[0]
g_wb = g_norm * wb[1]
b_wb = b_norm * wb[2]
```

White balance multipliers compensate for the scene illuminant. Daylight WB
typically has `wb[0] > 1` (boost red) and `wb[2] > 1` (boost blue).

**Stage 3 — Colour matrix.**

```
[r', g', b'] = color_matrix × [r_wb, g_wb, b_wb]
```

Converts camera-native linear RGB to standard linear sRGB primaries.
Each RAW codec supplies a camera-specific matrix (or the identity matrix
for RGB sensors already calibrated to sRGB).

After the matrix multiply, each channel is clamped to [0.0, 1.0].

**Stage 4 — sRGB gamma.**

```
r_out = srgb_gamma(r')
```

Applied independently to each channel. Converts linear light to the
perceptual encoding that monitors display correctly.

**Output.**

Each channel is scaled by 255, rounded, and clamped to [0, 255] as u8.

## Crate layout

```
image-raw-pipeline/
  Cargo.toml       (no dependencies — zero-dep)
  BUILD            (cargo test -p image-raw-pipeline -- --nocapture)
  README.md
  CHANGELOG.md
  src/
    lib.rs         (re-exports + module-level documentation)
    gamma.rs       (srgb_gamma, srgb_decode)
    matrix.rs      (mat3x3_mul, invert_3x3)
    pipeline.rs    (apply_color_pipeline)
```

## Dependencies

**None.** All math is elementary scalar arithmetic. The `matrix` crate
was considered for `invert_3x3`, but its `Vec<Vec<f64>>` representation
heap-allocates for each 3×3 operation. Since inversion happens once per
RAW file decode (not per pixel), either approach is fine — but zero deps
is simpler and avoids adding to the workspace's dep graph.

## Test requirements

≥ 30 unit tests, targeting ≥ 95% line coverage:

- `srgb_gamma`: output at 0.0, 1.0, linear segment boundary (0.0031308),
  below/above boundary, midpoint (~0.5 → ~0.735)
- `srgb_decode`: output at 0.0, 1.0, linear segment boundary (0.04045),
  round-trip with srgb_gamma
- `mat3x3_mul`: identity matrix, zero matrix, channel-swap matrix, known
  vector example
- `invert_3x3`: identity matrix self-inverse, diagonal matrix, singular
  matrix returns None, `M × inv(M) = I` check
- `apply_color_pipeline`: empty input, identity pipeline (black and white
  inputs), WB multiplier boosts a channel to saturation, colour matrix
  swaps R and B, black level subtraction, white level normalization

## Consumers after this crate lands

| Crate | What changes |
|-------|-------------|
| `image-codec-tiff` | `color.rs` delegates `apply_color_pipeline` and removes private `apply_srgb_gamma` |
| `image-codec-raf`  | `color.rs` delegates `apply_color_pipeline` and removes private `srgb_gamma` |
| `image-codec-rw2`  | `color.rs` delegates `apply_color_pipeline` and removes private `srgb_gamma` |
| `image-codec-dng`  | `color.rs` uses `invert_3x3` and `mat3x3_mul` instead of inline copies |
