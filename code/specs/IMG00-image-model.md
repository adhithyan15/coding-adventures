# IMG00 — The Raster Image Model

## Overview

A **raster image** is a finite, rectangular grid of discrete color samples called
**pixels** (picture elements). Every pixel lives at an integer coordinate (x, y)
and holds a color value. The full image is simply a 2D matrix:

```
Image(W, H) = { pixel(x, y) | 0 ≤ x < W, 0 ≤ y < H }
```

Mathematically an image is a function:

```
f : ℤ × ℤ → P
```

where **P** is the **pixel type** — the set of possible color values — and the
domain is finite: x ∈ [0, W), y ∈ [0, H).

This spec defines the data model shared by every package in the IMG series. It
answers three questions before any code is written:

1. What is the structure of a pixel? (pixel types and channel formats)
2. How are pixels arranged in memory? (layout, stride, planar vs. interleaved)
3. What coordinate conventions does this series follow?

It also provides a roadmap of the full operation taxonomy so that later specs
(IMG01–IMG07) can position themselves within the larger whole.

---

## Series Overview

```
IMG00  (this spec)   Image model — pixel types, colorspaces, memory layout, coordinates
IMG01               Convolution and spatial filters — Gaussian blur, edge detection, kernels
IMG02               LUTs — 1D tone curves, 3D LUT (.cube format), GPU LUT textures
IMG03               Point operations — brightness, contrast, gamma, sRGB↔linear
IMG04               Geometric transforms — affine, perspective, scale, rotate, sampling
IMG05               Image I/O — pure PNG, JPEG, and BMP codec implementations
IMG06               GPU acceleration bridge — Rust+wgpu core, TypeScript+WebGPU, C-ABI FFI
IMG07               Morphological operations — erosion, dilation, opening, closing
```

Specs IMG01–IMG05 define CPU-only implementations in every supported language
(Python, TypeScript, Rust, Go, Ruby, etc.). IMG06 then provides a Rust crate
that runs the same operations on the GPU and exposes a C-ABI so every language
can call it. TypeScript additionally calls the browser's native WebGPU API
directly.

---

## 1. Pixels and Channel Formats

A **channel** is a single numeric component of a pixel. The numeric type of a
channel determines its precision and the operations you can perform on it.

### Primitive channel types

| Type   | Width  | Range          | Typical use                          |
|--------|--------|----------------|--------------------------------------|
| u8     | 8-bit  | [0, 255]       | sRGB display, web images             |
| u16    | 16-bit | [0, 65535]     | 16-bit PNG, RAW camera files         |
| f16    | 16-bit | ≈[0.0, 65504]  | GPU textures, HDR intermediate       |
| f32    | 32-bit | ≈[−3.4×10³⁸, …]| Linear-light computation, HDR output |

The rule of thumb: **u8** for final display output (you have 8 bits of precision,
which is enough for the human eye at typical viewing conditions); **f32** for
anything involving arithmetic (blending, convolution, color-space conversions)
because rounding errors accumulate fast in repeated u8 operations.

### Pixel formats

A pixel format specifies the number of channels, their types, and their
semantics.

```
Format       Channels  Channel type  Bits/pixel  Notes
───────────────────────────────────────────────────────────────────
Luma8        1         u8            8           Grayscale
LumaA8       2         u8            16          Grayscale + alpha
RGB8         3         u8            24          sRGB — the web default
RGBA8        4         u8            32          sRGB + alpha
RGB16        3         u16           48          16-bit HDR
RGBA16       4         u16           64          16-bit HDR + alpha
RGB32F       3         f32           96          Linear-light HDR
RGBA32F      4         f32           128         Linear-light HDR + alpha
```

The **alpha** channel encodes opacity: 0 = fully transparent, 255 (or 1.0) =
fully opaque. When blending two images A and B:

```
out = A.rgb * A.alpha + B.rgb * (1 - A.alpha)   (alpha pre-multiplied: A.rgb already multiplied)
out = A.rgb * A.alpha + B.rgb * (1 - A.alpha)   (straight alpha: A.rgb is not pre-multiplied)
```

Pre-multiplied alpha is preferred for compositing because blending is a single
multiply-add; straight alpha requires an extra divide on output. Most render
pipelines use pre-multiplied internally and convert at the image boundary.

---

## 2. Colorspaces

A colorspace defines what the numbers in a pixel *mean*. Two images with
identical byte patterns but different colorspace tags look different on screen.

### sRGB — the default

sRGB (IEC 61966-2-1, 1999) is the colorspace of the web, JPEG, PNG, and most
consumer cameras. It has three properties that matter for implementation:

1. **Primaries**: the CIE xy chromaticity coordinates of the red, green, and
   blue primaries. These are device-dependent (sRGB's primaries are based on
   the 1990s CRT phosphors), but are the world's most common convention.

2. **White point**: D65 (6500 K daylight), the CIE standard illuminant used for
   TV, print, and the web.

3. **Transfer function (gamma)**: sRGB uses a piecewise function that
   approximates a power curve with exponent ≈ 2.2:

```
sRGB → linear  (decode):
  if C_srgb ≤ 0.04045:
      C_linear = C_srgb / 12.92
  else:
      C_linear = ((C_srgb + 0.055) / 1.055) ^ 2.4

linear → sRGB  (encode):
  if C_linear ≤ 0.0031308:
      C_srgb = C_linear × 12.92
  else:
      C_srgb = 1.055 × C_linear^(1/2.4) − 0.055
```

The gamma curve is the single most common source of subtle image-processing
bugs. The rule is absolute: **convert to linear light before any arithmetic
(blending, convolution, scaling), then convert back to sRGB for storage or
display.**

To see why, consider blending 50% black (0) and 50% white (255) in sRGB u8:

```
sRGB blend (wrong):  (0 + 255) / 2 = 127   → perceived as darker than 50% grey
Linear blend (right):
  linear_black = 0.0
  linear_white = 1.0
  linear_mid   = 0.5
  srgb_mid     = encode_srgb(0.5) ≈ 188    → perceptually correct 50% grey
```

The difference is 127 vs 188 — a visually significant error. Every Gaussian
blur, every scale, every image composite should happen in linear light.

### Linear RGB

Same primaries and white point as sRGB, but no gamma transfer: numbers are
proportional to physical luminance. This is the correct space for:

- Gaussian blur and convolution
- Image scaling / downsampling
- Alpha blending
- Lighting and shading calculations

### HSV and HSL

HSV (Hue–Saturation–Value) and HSL (Hue–Saturation–Lightness) are cylindrical
re-parameterisations of RGB, designed to be more intuitive for artists:

```
H (hue)        ∈ [0°, 360°)   — the "color wheel" position
S (saturation) ∈ [0, 1]       — 0 = grey, 1 = fully saturated
V (value)      ∈ [0, 1]       — 0 = black, 1 = as bright as the primary allows
```

```
Color wheel — hue H:
      60°   120°
   Yellow   Green
 30°              150°
Chartreuse       Cyan-green

0°  Red                Cyan  180°

300°            210°
Magenta       Blue-green
     240°  300°
      Blue  Violet
```

HSV is useful for color pickers and color-range selection (e.g., "select all
orange pixels"). It is not suitable for blending or convolution — convert to
linear RGB first.

Conversion (HSV → RGB, for H ∈ [0°, 360°), S,V ∈ [0,1]):

```
C = V × S                 (chroma)
X = C × (1 − |H/60° mod 2 − 1|)
m = V − C

       H in [0°,60°):  (R',G',B') = (C, X, 0)
       H in [60°,120°): (R',G',B') = (X, C, 0)
       H in [120°,180°):(R',G',B') = (0, C, X)
       H in [180°,240°):(R',G',B') = (0, X, C)
       H in [240°,300°):(R',G',B') = (X, 0, C)
       H in [300°,360°):(R',G',B') = (C, 0, X)

(R, G, B) = (R'+m, G'+m, B'+m)
```

### CIE L*a*b* (CIELAB)

CIELAB (CIE 1976) is a **perceptually uniform** colorspace: equal numeric
distances correspond to equal perceived color differences for an average human
observer under a specified viewing condition.

```
L* ∈ [0, 100]       — lightness: 0 = black, 100 = diffuse white
a* ∈ [−128, 127]    — green (−) to red (+) opponent axis
b* ∈ [−128, 127]    — blue (−) to yellow (+) opponent axis
```

The principal use of CIELAB is **color difference measurement**:

```
ΔE₇₆ = √(ΔL*² + Δa*² + Δb*²)
```

A ΔE of 1 is approximately the just-noticeable difference (JND) for an average
observer. ΔE < 2 is considered acceptable for print; ΔE < 1 for critical
colour-matching work.

Conversion goes through XYZ (not reproduced in full here; see IMG03).

### Oklab (Björn Ottosson, 2020)

Oklab is a modern perceptually uniform colorspace with two advantages over
CIELAB:

1. **Better hue linearity**: hues do not shift when you interpolate or increase
   saturation. A gradient from blue to red through Oklab stays vivid in the
   middle; through HSV or sRGB it often turns muddy grey.
2. **Simpler derivation**: a direct matrix + per-channel cube root, no
   discontinuous cases.

```
L ∈ [0, 1]     — lightness
a ∈ [−0.5, 0.5] — approx. green–red
b ∈ [−0.5, 0.5] — approx. blue–yellow
```

Preferred over CIELAB for color interpolation, gradient generation, and palette
manipulation.

### YCbCr

YCbCr separates **luma** (Y, the brightness signal) from two **chroma**
channels (Cb = blue-difference, Cr = red-difference). It is the native
colorspace of JPEG, H.264, and H.265.

```
Y  ∈ [0, 1]          — luma (weighted average of linear R, G, B)
Cb ∈ [−0.5, 0.5]     — blue-difference chroma
Cr ∈ [−0.5, 0.5]     — red-difference chroma
```

The reason YCbCr dominates video codecs: human vision is far more sensitive to
luma detail than to chroma detail. So chroma can be **subsampled** — stored at
half or quarter resolution — without visible quality loss. The most common
scheme is **4:2:0**: luma at full resolution, both chroma channels at half
horizontal and half vertical resolution. This saves roughly 50% bandwidth.

---

## 3. Memory Layout

The pixel type tells us what one pixel looks like. The memory layout tells us
how an entire image is arranged as a flat byte buffer.

### Dimensions and stride

```
Field      Symbol  Description
──────────────────────────────────────────────────────────────
width        W     number of columns (x direction)
height       H     number of rows (y direction)
channels     C     components per pixel (1 for Luma, 3 for RGB, 4 for RGBA)
bit depth    D     bits per channel (8, 16, or 32)
stride       S     bytes from start of row i to start of row i+1
```

The minimum possible stride (tightly packed) is:

```
S_min = W × C × (D / 8)
```

Most libraries pad rows to a multiple of 4 or 16 bytes for SIMD alignment. The
extra bytes are called **row padding** and must not be interpreted as pixel
data.

```
Row y = 0: [pixel(0,0)][pixel(1,0)]...[pixel(W-1,0)][padding]
Row y = 1: [pixel(0,1)][pixel(1,1)]...[pixel(W-1,1)][padding]
...
Row y = H-1: [pixel(0,H-1)]...[pixel(W-1,H-1)][padding]
```

Byte offset of pixel (x, y):

```
offset(x, y) = y × S + x × bytes_per_pixel
             = y × S + x × C × (D / 8)
```

### Interleaved layout (packed)

All channels of one pixel are stored together:

```
RGBA8 (interleaved):
  R₀ G₀ B₀ A₀  R₁ G₁ B₁ A₁  R₂ G₂ B₂ A₂  ...
  ◄── pixel 0 ──►◄── pixel 1 ──►◄── pixel 2 ──►
```

This is the default for this series. Advantages:

- Cache-friendly when processing one pixel at a time (all channels adjacent)
- The format expected by OpenGL, Metal, WebGPU textures
- The format produced by PNG, JPEG, and BMP decoders

### Planar layout

All values of one channel are grouped together:

```
RGB planar (3 separate planes):
  plane R: R₀ R₁ R₂ R₃ ... R_{W×H-1}
  plane G: G₀ G₁ G₂ G₃ ... G_{W×H-1}
  plane B: B₀ B₁ B₂ B₃ ... B_{W×H-1}
```

Advantages:

- Excellent for SIMD: load 16 consecutive red values in a single AVX2 load
- Used by many video codecs for YCbCr data
- Allows different subsampling per channel (4:2:0)

The GPU acceleration layer (IMG06) accepts both layouts via a layout descriptor
passed to the shader. CPU code in this series defaults to interleaved.

### Byte order for multi-byte channels

For u16 and f32 channels, the byte order of the host CPU matters:

- x86, ARM (LE mode): **little-endian** — least-significant byte at the lower address
- Network byte order, most file formats: **big-endian** — most-significant byte first

16-bit PNG uses big-endian u16. A little-endian CPU must byte-swap when reading:

```
u16 from big-endian bytes [hi, lo]:
  value = (uint16_t)hi << 8 | lo

u16 from little-endian bytes [lo, hi]:
  value = lo | (uint16_t)hi << 8
```

f32 values are almost always stored in the native endianness of the host
machine. File formats that store f32 (e.g., OpenEXR) specify endianness in
their header.

---

## 4. Coordinate System

This series uses the **raster coordinate system** throughout:

```
(0,0) ─────────────────── x ────────────────► (W-1, 0)
  │
  │    (0,0) is the TOP-LEFT corner of the top-left pixel.
  │
  y    x increases to the RIGHT  (column index).
  │    y increases DOWNWARD      (row index).
  │
  ▼
(0, H-1)                               (W-1, H-1)
```

This convention matches PNG, JPEG, BMP, HTML Canvas, Metal, DirectX, and
WebGPU. It is the opposite of the mathematical Cartesian convention (y-up) used
by OpenGL and PDF. A y-flip is one of the most common pitfalls when mixing
rendering APIs.

### Sub-pixel coordinates

Geometric transforms (IMG04) and image sampling operate at sub-pixel precision.
A sub-pixel coordinate (u, v) where u ∈ ℝ refers to a position *within* the
image. The conversion between sub-pixel coordinates and integer pixel indices
depends on the **sample model**:

- **Pixel-centre model** (used by this series): integer (i, j) refers to the
  centre of pixel (i, j). The pixel occupies the square [i−0.5, i+0.5) ×
  [j−0.5, j+0.5). To sample at (u, v) bilinearly, take the four nearest
  integer centres.

- **Pixel-corner model** (used by some OpenGL code): integer (i, j) refers to
  the top-left corner of pixel (i, j). This shifts all sample positions by
  (0.5, 0.5) relative to the pixel-centre model.

Mixing the two models is a common half-pixel alignment bug. This series always
uses the pixel-centre model.

---

## 5. Categories of Image Operations

Every image operation takes one or more images as input, applies a
transformation, and produces one or more images as output. The categories below
partition the operation space by **locality** — how far from the current pixel
each output sample depends.

### 5.1 Point operations (zero neighbourhood)

Each output pixel depends only on the corresponding input pixel. The operation
is a pure function P → P applied independently at every (x, y).

```
O(x, y) = f( I(x, y) )
```

The work is trivially parallelisable: no data dependencies between pixels.
Modern CPUs process 16–32 u8 pixels per clock with SIMD; GPUs process millions
per microsecond.

**Examples**:

| Operation           | Mapping                                                     |
|---------------------|-------------------------------------------------------------|
| Invert              | O = 255 − I  (u8 domain)                                   |
| Brightness +Δ       | O = clamp(I + Δ, 0, 255)                                   |
| Contrast ×k         | O = clamp((I − 128) × k + 128, 0, 255)                     |
| Gamma γ             | O = (I / 255)^γ × 255                                       |
| sRGB → linear       | piecewise function (see §2)                                 |
| Threshold at t      | O = 255 if I ≥ t else 0                                     |
| Channel swap        | O.R = I.B; O.G = I.G; O.B = I.R                            |
| Colour matrix       | [O.R,O.G,O.B]ᵀ = M × [I.R,I.G,I.B]ᵀ (3×3 or 4×4 matrix)  |

Point operations are specified fully in IMG03.

### 5.2 Geometric transforms (coordinate remapping)

The output image is formed by mapping each output coordinate (x', y') back to a
source coordinate (u, v) and sampling the input image there. Because (u, v) is
generally non-integer, **interpolation** is required.

```
(u, v) = T⁻¹(x', y')          (inverse warp)
O(x', y') = sample( I, u, v )  (bilinear, bicubic, …)
```

**Examples**:

| Transform          | T(x, y)                                            |
|--------------------|----------------------------------------------------|
| Flip horizontal    | (W−1−x, y)                                         |
| Flip vertical      | (x, H−1−y)                                         |
| 90° rotation (CW)  | (y, W−1−x)                                         |
| Scale by (sx, sy)  | (x/sx, y/sy)                                       |
| Affine warp        | M × [x, y, 1]ᵀ  where M is a 2×3 matrix           |
| Perspective warp   | homogeneous divide: M × [x, y, 1]ᵀ / w             |

Geometric transforms are specified fully in IMG04.

### 5.3 Spatial filters (neighbourhood operations)

Each output pixel depends on a rectangular **neighbourhood** of input pixels,
combined using a small weight matrix called the **kernel**. This is the costliest
operation family and the subject of IMG01.

```
O(x, y) = Σᵢ Σⱼ  K(i, j) × I(x+i, y+j)
```

**Examples**:

| Filter              | What the kernel measures / applies                          |
|---------------------|-------------------------------------------------------------|
| Box blur            | Uniform average of the neighbourhood                        |
| Gaussian blur       | Weighted average, weights fall off as a Gaussian            |
| Sobel horizontal    | Approximates ∂I/∂x — detects vertical edges                |
| Sobel vertical      | Approximates ∂I/∂y — detects horizontal edges              |
| Laplacian           | Approximates ∇²I — highlights regions of rapid change      |
| Sharpen             | Center minus Laplacian: accentuates edges                   |
| Unsharp mask        | I + k × (I − blur(I)) — selective sharpening               |
| Emboss              | Directional edge with lighting bias                         |

### 5.4 LUTs and colour transforms (structured point operations)

A special class of point operations where the mapping P → P is stored as a
precomputed table rather than evaluated algebraically. The table can encode
transformations too complex for a closed-form formula.

- **1D LUT**: one table per channel; maps each channel value independently.
  Stores arbitrary tone curves.
- **3D LUT**: one N×N×N colour cube; maps an (R,G,B) triple to a new (R',G',B').
  Stores arbitrary colour grades, ICC profile approximations, film looks.

LUTs are the subject of IMG02.

### 5.5 Frequency-domain operations (global operations)

These decompose the entire image into spatial-frequency components via the
2D Discrete Fourier Transform (DFT), operate in frequency space, then
reconstruct.

```
F = DFT(I)          — W×H complex values, each a spatial frequency component
F' = op(F)          — e.g., multiply by a Gaussian to low-pass filter
O = IDFT(F')        — inverse transform back to spatial domain
```

Because the DFT is global, a single operation touches every pixel in the output.
This makes frequency-domain operations expensive to compose but extremely
powerful: a Gaussian blur via the DFT has O(WH log(WH)) cost compared to
O(WHr²) for a direct spatial convolution — advantageous for very large kernels.

This series does not implement full DFT-based filtering in the initial set of
specs, but it is named here so that IMG01 (convolution) can explain when the
spatial vs frequency tradeoff tips.

---

## 6. The `Image<P>` Type

Every implementation package in this series provides an `Image<P>` container
parameterised by pixel type P. The container must satisfy the following
interface (pseudocode; language-specific syntax in each package):

```
type Image<P> {
    width  : uint32
    height : uint32
    stride : uint32        // bytes per row (≥ width × sizeof(P))
    data   : &mut [u8]     // raw byte buffer, length = stride × height
}

impl Image<P> {
    fn new(width, height) -> Image<P>                 // stride = width × sizeof(P) (tightly packed)
    fn new_with_stride(width, height, stride) -> Image<P>
    fn pixel(x, y) -> &P                              // read pixel at (x, y)
    fn pixel_mut(x, y) -> &mut P                      // write pixel at (x, y)
    fn row(y) -> &[P]                                 // slice of one row
    fn row_mut(y) -> &mut [P]
    fn as_bytes() -> &[u8]                            // raw byte view
    fn crop(x, y, w, h) -> Image<P>                  // new image backed by same buffer (zero-copy)
}
```

The `crop` operation returns a **view** into the original buffer, not a copy.
The cropped image has a stride that matches the original image's stride (the
full row width), not the cropped width. This is standard practice and allows
zero-copy region processing.

---

## 7. Precision and Overflow

Before implementing any operation, pin down the arithmetic precision rules:

### Rule 1: work in the precision of the output

If the output format is f32, do all intermediate arithmetic in f32. If the
output is u8, intermediate results may temporarily exceed [0, 255] (e.g., a
convolution sum) and must be **clamped** before storing.

### Rule 2: clamp, do not wrap

u8 overflow must clamp to [0, 255], not wrap around modulo 256. A brightness
increase that pushes 200 to 320 should produce 255, not 64 (320 mod 256).
Wrapping produces vivid but wrong colour artefacts. Language-specific notes:

- Rust: use `u8::saturating_add` / `saturating_sub`, or cast via f32.
- C/C++: unsigned arithmetic wraps (defined behaviour); add explicit clamp.
- Python: integers do not overflow; cast to float, clamp, then cast back.

### Rule 3: accumulate convolutions in f32 or f64

A 5×5 kernel times a u8 image accumulates up to 25 products. Each product ≤
255 × 1.0 (normalised kernel) ≤ 255. The sum ≤ 255. But intermediate values
during accumulation can reach 25 × 255 = 6375, which overflows u8 and i16.
Always accumulate convolution sums in f32 (or i32 for integer kernels), then
clamp to the output range.

---

## 8. Relationship to the Paint Stack

The image model defined here is independent of the PaintVM (P2D) stack used for
vector graphics and text rendering. The two meet at **raster compositing**:

```
PaintVM emits PaintScene instructions
    │
    ▼  PaintRasterizer (P2D05, P2D06)
    │  converts vector + text → pixel buffer
    │
    ▼  Image<RGBA8>          ← this series
    │
    ▼  Display / export pipeline
```

A `PaintGlyphRun` or `PaintFillPath` instruction eventually writes pixels into
an `Image<RGBA8>`. The image processing operations in this series (blur, LUTs,
transforms) can be applied to that buffer before final display.

---

## 9. Terminology Reference

| Term            | Definition                                                         |
|-----------------|--------------------------------------------------------------------|
| Pixel           | One sample in the image grid; a tuple of channel values            |
| Channel         | A single numeric component of a pixel (e.g. Red, Green, Blue)     |
| Pixel format    | The combination of channel count, type, and semantic               |
| Stride          | Bytes from the start of row y to the start of row y+1             |
| Interleaved     | Layout: all channels of one pixel stored together                  |
| Planar          | Layout: all values of one channel stored together                  |
| Colorspace      | The interpretation of pixel values as physical colours             |
| Gamma           | The non-linear transfer function between physical and stored light |
| Linear light    | Pixel values proportional to physical luminance; no gamma          |
| Premultiplied α | RGB channels already multiplied by alpha                           |
| Kernel          | A small weight matrix used in spatial convolution (IMG01)          |
| LUT             | Look-Up Table: precomputed colour mapping (IMG02)                  |
| Affine          | Linear transform + translation; preserves parallel lines (IMG04)   |
| Perspective     | Projective transform; preserves straight lines but not parallelism |
