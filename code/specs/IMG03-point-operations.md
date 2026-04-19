# IMG03 — Point Operations

## Overview

A **point operation** transforms each pixel independently: the output at
position (x, y) depends only on the input at (x, y). No neighbourhood, no
geometry, no knowledge of adjacent pixels.

```
O(x, y) = f( I(x, y) )     for every (x, y) in the image
```

Because each pixel is independent, point operations are embarrassingly
parallel — the ideal GPU workload — and the cheapest operations in the IMG
series.

This spec defines the standard library of point operations built on top of
`Image` (IC00). `Image` stores pixels as RGBA8 in sRGB
colour space. Point operations either work directly on u8 values (when the
mapping is perceptually uniform in sRGB, or when speed matters more than
accuracy) or lift to a linear-light f32 working buffer, operate there, and
pack back to RGBA8.

---

## 1. `Image` as the Base Type

`Image` (IC00) is the concrete image type for every IMG package:

```
Image {
    width:  u32
    height: u32
    data:   [u8]   // RGBA8, row-major, top-left, stride = width × 4
}

offset(x, y) = (y * width + x) * 4
R = data[offset + 0]
G = data[offset + 1]
B = data[offset + 2]
A = data[offset + 3]
```

The RGBA8 format is the canonical interchange format: every IC codec produces
and consumes it, and every paint back-end accepts it. The IMG operations in
this spec accept `Image` as both input and output.

### The Image ↔ f32 working buffer pattern

Many operations must work in **linear light** (see IMG00 §2). The standard
pattern for all such operations:

```
Step 1: decode sRGB u8 → linear f32
    for each pixel (r, g, b, a) in Image:
        R_lin = srgb_to_linear(r / 255.0)
        G_lin = srgb_to_linear(g / 255.0)
        B_lin = srgb_to_linear(b / 255.0)
        A_lin = a / 255.0          ← alpha is always linear; no gamma

Step 2: operate on [R_lin, G_lin, B_lin, A_lin] in f32

Step 3: encode linear f32 → sRGB u8
    for each pixel:
        r = round(linear_to_srgb(R_lin) × 255.0)
        g = round(linear_to_srgb(G_lin) × 255.0)
        b = round(linear_to_srgb(B_lin) × 255.0)
        a = round(A_lin × 255.0)
        clamp all to [0, 255]
```

The sRGB encode and decode functions are the piecewise functions from IMG00 §2.
Both directions should be precomputed as LUTs (IMG02 §2.1) and cached at
library initialisation — one `f32[256]` table for decode (u8 → f32 linear),
one `f32[1024]` table for encode (f32 linear → f32 sRGB, then multiply by 255
and round).

**Which operations need linear light, and which do not:**

| Operation                     | Needs linear light? | Reason                                        |
|-------------------------------|---------------------|-----------------------------------------------|
| Invert                        | No                  | 255−x is symmetric under sRGB gamma           |
| Threshold / posterize         | No                  | Comparison; no arithmetic                     |
| Channel swap / extract        | No                  | Structural, no arithmetic                     |
| Brightness (additive)         | No (close enough)   | Addition is approximately linear in sRGB      |
| Contrast (multiplicative)     | **Yes**             | Multiplication amplifies gamma non-linearity  |
| Gamma correction              | **Yes**             | By definition                                 |
| Colour matrix                 | **Yes**             | Matrix blend must be linear to be physically correct |
| HSV / HSL adjustment          | **Yes**             | Convert to linear first, adjust, convert back |
| Exposure (EV stops)           | **Yes**             | Multiplicative in linear light                |

---

## 2. Operation Reference

### 2.1 Invert

Negate each colour channel. Alpha is unaffected.

```
R_out = 255 - R_in
G_out = 255 - G_in
B_out = 255 - B_in
A_out = A_in
```

No colorspace conversion required. This is exact in both sRGB and linear
spaces because f(x) = 1−x is its own inverse and is a valid sRGB tone curve.

### 2.2 Threshold

Convert to a two-level (binary) image. Each channel is replaced by 0 or 255
based on whether it exceeds a threshold t ∈ [0, 255].

```
for each channel C:
    C_out = 255 if C_in >= t else 0
```

The threshold is applied per-channel to the RGBA8 values directly. A common
variant uses luminance: compute Y = 0.299R + 0.587G + 0.114B (BT.601
luma in sRGB), threshold Y, then set all channels to 0 or 255.

### 2.3 Posterize (quantise)

Reduce the number of distinct values per channel to `levels`:

```
step = 255 / (levels - 1)
C_out = round(C_in / step) * step    (clamped to [0, 255])
```

For `levels = 2` this is equivalent to threshold at 128.

### 2.4 Channel operations

**Swap channels** (e.g. BGR ↔ RGB):

```
R_out = B_in
G_out = G_in
B_out = R_in
```

**Extract single channel to greyscale**:

```
R_out = G_out = B_out = C_in    (where C is the chosen channel)
A_out = 255
```

**Set alpha from luminance** (used to create luminance masks):

```
Y = round(0.299*R + 0.587*G + 0.114*B)   ← BT.601 in sRGB
A_out = Y;  R_out = R_in;  G_out = G_in;  B_out = B_in
```

### 2.5 Brightness (additive shift)

```
brightness ∈ [−255, +255]   (u8 domain)

C_out = clamp(C_in + brightness, 0, 255)
```

Applied directly in sRGB u8. Strictly speaking, additive shifts are not
gamma-correct (the perceived brightness change is not uniform across the
tonal range), but for interactive adjustments and small values the
approximation is acceptable. For physically accurate exposure adjustment
use §2.8.

### 2.6 Contrast

Contrast stretches or compresses the tonal range around the midpoint 128.

```
contrast ∈ (−1.0, +∞)    (−1 = full compression; 0 = no change; positive = expansion)

factor = (259 * (contrast + 255)) / (255 * (259 - contrast))
C_out = clamp(factor * (C_in - 128) + 128, 0, 255)
```

This formula is the classic "contrast adjustment" from graphics textbooks.
The `factor` is derived so that `contrast = 0` gives `factor = 1.0` (no
change), and the formula preserves 128 as the pivot point.

For correct results the adjustment should be applied in **linear light**: the
sRGB midpoint 128 corresponds to approximately 0.216 in linear light — not
50% — so a contrast expansion in sRGB space darkens the lower half of the
tonal range more than the upper half. Applications that care about physical
accuracy should convert to linear, apply the contrast factor symmetrically
around 0.5, then re-encode.

### 2.7 Gamma correction

Apply a power-law tone curve with exponent γ:

```
// Work in linear f32.
C_lin = srgb_to_linear(C_in / 255.0)
C_lin_out = clamp(C_lin ^ γ, 0.0, 1.0)
C_out = round(linear_to_srgb(C_lin_out) * 255.0)
```

`γ < 1` → brightens midtones (camera underexposure correction).
`γ > 1` → darkens midtones.
`γ = 1` → identity.

Note: applying a γ curve on top of the sRGB transfer function means the
effective display gamma becomes 2.2 × γ. This is the correct model for
photographic "gamma adjustment" tools.

### 2.8 Exposure (EV stops)

Multiply linear-light values by 2^EV (each EV stop doubles or halves the
light energy):

```
C_lin = srgb_to_linear(C_in / 255.0)
C_lin_out = clamp(C_lin * pow(2.0, ev), 0.0, 1.0)
C_out = round(linear_to_srgb(C_lin_out) * 255.0)
```

`ev = +1` → one stop overexposure (double the light).
`ev = −1` → one stop underexposure (half the light).

Unlike the additive brightness shift (§2.5), EV adjustment is physically
correct: it models the effect of changing the camera's exposure time.

### 2.9 sRGB ↔ Linear conversion

Convert between sRGB-encoded and linear-light Images:

```
// sRGB → linear:
R_lin = round(srgb_to_linear(R_in / 255.0) * 255.0)
(same for G, B; A unchanged)

// linear → sRGB:
R_srgb = round(linear_to_srgb(R_in / 255.0) * 255.0)
```

The output is still RGBA8, but the meaning of the values has changed: values
now represent linear-light intensities (approximately) packed into u8.
Note that quantising linear values to u8 introduces visible banding in dark
regions (the sRGB gamma curve is specifically designed to give more u8
headroom to dark tones). Use f32 intermediates for any processing that
follows linear decode.

### 2.10 Colour matrix

Apply a 3×3 (or 4×4 with bias) matrix in linear light:

```
[R_out]   [m00 m01 m02 m03]   [R_in ]
[G_out] = [m10 m11 m12 m13] × [G_in ]
[B_out]   [m20 m21 m22 m23]   [B_in ]
[1    ]   [ 0   0   0   1 ]   [1    ]
```

The matrix encodes any linear colour transform: white balance correction,
colour grading with bias, channel mixing, desaturation, or conversion between
RGB primaries.

**Greyscale (luminance weights)**:

```
Rec. 709 (sRGB, HDR):
  Y = 0.2126 R + 0.7152 G + 0.0722 B    (linear light)

BT.601 (standard definition video):
  Y = 0.299  R + 0.587  G + 0.114  B

Simple average (fast, less accurate):
  Y = (R + G + B) / 3
```

To produce a greyscale Image using Rec.709:

```
matrix = [ 0.2126  0.7152  0.0722  0.0 ]
         [ 0.2126  0.7152  0.0722  0.0 ]
         [ 0.2126  0.7152  0.0722  0.0 ]
```

### 2.11 Sepia tone

A warm brownish greyscale effect. Applied after desaturation in linear light:

```
// After converting to greyscale Y (linear):
R_out = linear_to_srgb(clamp(Y * 1.351 + 0.0, 0, 1))   → sepia red
G_out = linear_to_srgb(clamp(Y * 1.203 + 0.0, 0, 1))   → sepia green
B_out = linear_to_srgb(clamp(Y * 0.937 + 0.0, 0, 1))   → sepia blue
```

The scale factors come from the average colour of aged photographic paper
(approximately R=112/G=66/B=20 in sRGB u8).

### 2.12 Hue rotation

Shift all hues by an angle θ ∈ [0°, 360°) while preserving luminance and
saturation. Applied in linear light via a rotation matrix. The exact matrix
for rotating hue by θ (Blythe, 2002):

```
// In linear RGB:
Ur = Ug = Ub = 0.2126, 0.7152, 0.0722   (Rec.709 luminance weights)

// 3D rotation around the grey axis (1,1,1)/√3 by angle θ:
// (Not reproduced in full — see the colour-matrix package for the
//  closed-form expanded version.)

The matrix is a standard rotation-around-axis formula applied to the
colour cube.
```

For practical implementation, convert to HSV/HSL in linear light, add θ to
H, convert back. Wrap H modulo 360°.

### 2.13 Saturation

Adjust colour saturation by a factor s:

```
s = 0.0 → fully desaturated (greyscale)
s = 1.0 → no change
s > 1.0 → oversaturated (hyper-vivid)
```

Using the colour matrix approach in linear light:

```
Yr = 0.2126;  Yg = 0.7152;  Yb = 0.0722   (Rec.709)

matrix = [ Yr + s*(1-Yr)   Yg - s*Yg     Yb - s*Yb   ]
         [ Yr - s*Yr       Yg + s*(1-Yg)  Yb - s*Yb   ]
         [ Yr - s*Yr       Yg - s*Yg     Yb + s*(1-Yb) ]
```

At s=0 the matrix reduces to the greyscale matrix. At s=1 the matrix is
the identity. For s>1, clamp outputs to [0,1] before encoding.

---

## 3. 1D LUT Application (from IMG02)

Point operations built from arbitrary tone curves use the 1D LUT machinery
defined in IMG02. The `apply_lut1d_u8` function from IMG02 §9 takes three
256-entry u8 tables and applies them channel-by-channel in a single pass:

```
for each pixel:
    R_out = r_lut[R_in]
    G_out = g_lut[G_in]
    B_out = b_lut[B_in]
    A_out = A_in           ← alpha unchanged unless an alpha LUT is provided
```

Any of the operations in §2 that accept simple scalar parameters can be
precomputed into a pair of LUTs (encode + decode) and then baked together
via LUT composition (IMG02 §6), reducing complex chains to a single
table lookup per pixel.

---

## 4. Alpha Channel Rules

Unless an operation explicitly documents alpha handling, the rule is:

- **Colour operations** (§2.5–§2.13) do not modify alpha. The alpha channel
  passes through unchanged.
- **Invert** (§2.1) does not invert alpha. Use a separate alpha-invert call
  if needed.
- **Threshold** (§2.2) applied to all channels does affect alpha.
- **Colour matrix** (§2.10) can include an alpha column if the 4×4 form is used.

When alpha is premultiplied (see IMG05), colour operations must account for
the scale: all channel values are ≤ alpha, and arithmetic should not treat
premultiplied values as straight values. Convert to straight alpha, operate,
and convert back.

---

## 5. Performance Notes

### SIMD layout

`Image` stores interleaved RGBA8 (R₀G₀B₀A₀ R₁G₁B₁A₁ …). For SIMD
processing, load 16 consecutive bytes into one 128-bit vector, which holds 4
complete RGBA8 pixels. Process 4 pixels per SIMD instruction.

With AVX2 (256-bit), 8 RGBA8 pixels fit per vector. On Apple Silicon (NEON,
128-bit, 4-wide), 4 pixels per vector.

For the f32 working-buffer path the layout expands to 16 bytes per pixel
(4 × f32). Load 2 pixels per 128-bit SIMD vector, or 4 per 256-bit AVX2.

### LUT throughput

Applying a 256-entry u8 LUT to RGBA8 data is a scatter-gather operation:
each byte is used as an index. On modern CPUs this is ~2–4 GB/s (limited by
L1 cache since the LUT fits in 256 bytes). For f32 LUTs, the lookup requires
a float multiply and an array index; throughput drops to ~1–2 GB/s.

### GPU dispatch (IMG06)

For images larger than ~2 MP (2 megapixels) the GPU accelerates all operations
in this spec via the colour-matrix shader (`shaders/point_ops.wgsl`). The
crossover depends on the GPU upload/download cost (typically ~0.7 ms for a
1920×1080 image at PCIe 3.0 bandwidth). An operation that takes >0.7 ms on
CPU benefits from GPU offload.

---

## 6. Interface

```
// u8-domain operations (no colorspace conversion):
fn invert(src: &Image) -> Image
fn threshold(src: &Image, t: u8) -> Image
fn threshold_luminance(src: &Image, t: u8) -> Image
fn posterize(src: &Image, levels: u8) -> Image
fn swap_channels_rgb_bgr(src: &Image) -> Image
fn extract_channel(src: &Image, channel: Channel) -> Image
fn brightness(src: &Image, delta: i16) -> Image

// linear-light operations (sRGB decode → f32 → process → sRGB encode):
fn contrast(src: &Image, factor: f32) -> Image
fn gamma(src: &Image, gamma: f32) -> Image
fn exposure(src: &Image, ev_stops: f32) -> Image
fn greyscale(src: &Image, weights: LuminanceWeights) -> Image
fn sepia(src: &Image) -> Image
fn hue_rotate(src: &Image, degrees: f32) -> Image
fn saturate(src: &Image, factor: f32) -> Image
fn colour_matrix(src: &Image, matrix: [[f32; 4]; 3]) -> Image

// colorspace conversion:
fn srgb_to_linear(src: &Image) -> Image
fn linear_to_srgb(src: &Image) -> Image

// LUT application (from IMG02):
fn apply_lut1d_u8(src: &Image, r: &[u8; 256], g: &[u8; 256], b: &[u8; 256]) -> Image
fn apply_lut1d_f32(src: &Image, lut: &Lut1dF32) -> Image
fn apply_lut3d(src: &Image, lut: &Lut3d) -> Image

// enum helpers:
enum Channel { R, G, B, A }
enum LuminanceWeights { Rec709, Bt601, Average }
```
