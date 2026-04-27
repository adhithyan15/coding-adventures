# IMG02 — LUTs: Look-Up Tables for Colour Remapping

## Overview

A **Look-Up Table** (LUT) replaces a computation with a table lookup. For image
processing, LUTs store precomputed colour mappings. At runtime, each pixel's
colour is used as an index into the table to retrieve the output colour — an
array access is cheaper than re-evaluating the original formula.

LUTs appear across the entire image-processing pipeline:

| Use case                     | LUT type      | Typical size       |
|------------------------------|---------------|--------------------|
| Per-channel tone curves       | 1D LUT        | 256 entries (u8)   |
| Gamma encode / decode        | 1D LUT        | 1024 entries (f32) |
| Film look ("colour grade")   | 3D LUT        | 17³ or 33³         |
| ICC colour profile           | 3D LUT        | 17³                |
| Night-vision / thermal       | 3D LUT        | 33³                |
| Hue-saturation curves        | 1D LUT × 3   | 360 entries (hue)  |

This spec covers:

- **1D LUTs**: per-channel tone curves, discrete and floating-point variants
- **3D LUTs**: the full (R,G,B) → (R′,G′,B′) colour cube, the `.cube` format,
  trilinear interpolation, tetrahedral interpolation
- **Identity LUTs** and LUT composition
- GPU LUTs: 3D LUTs as 3D textures for hardware-accelerated lookup

---

## 1. The Core Idea: Precompute Once, Lookup Many Times

Any pure function f : P → P applied to every pixel of an N-megapixel image
requires N evaluations of f. If f is cheap (one multiply) the cost is trivial.
If f is expensive (trigonometric functions, matrix inversion, ICC profile
interpolation), it dominates the processing budget.

A LUT solves this by precomputing f at a discrete set of input values:

```
Build phase (once):
  for i in 0..LUT_SIZE:
      lut[i] = f(i / (LUT_SIZE - 1))    // f evaluated at uniform samples

Apply phase (once per pixel):
  output = lut[input]                    // O(1) array access
```

The trade-off: the LUT is an approximation if f is evaluated at input values
between sample points. The approximation error is controlled by LUT size and
interpolation method.

---

## 2. 1D LUTs

A 1D LUT maps a single channel value to a new single channel value:

```
lut1d : C → C
```

For 8-bit images the table has exactly 256 entries — one per possible input
value — and no interpolation is needed:

```
u8 lut[256];
for x in 0..W:
    for y in 0..H:
        out.R(x,y) = lut[in.R(x,y)]
        out.G(x,y) = lut[in.G(x,y)]   // same or different LUT per channel
        out.B(x,y) = lut[in.B(x,y)]
```

For floating-point images the input domain [0.0, 1.0] is continuous, so
the LUT stores n+1 breakpoints and the lookup interpolates between adjacent
entries (§2.2).

### 2.1 Building a 1D LUT

Any monotone or non-monotone tone curve can be baked into a 1D LUT. Common
examples:

**Brightness and contrast** (linear transformation):

```
f(x) = contrast × x + brightness
lut[i] = clamp(contrast × (i/255) + brightness, 0, 1) × 255
```

**Gamma correction** (power law):

```
f(x) = x^γ
lut[i] = round( (i/255)^γ × 255 )

γ < 1.0 → brightens midtones (used in display calibration)
γ > 1.0 → darkens midtones
γ = 1.0 → identity (no change)
```

**sRGB ↔ linear conversion** (the piecewise function from IMG00 §2):

```
// sRGB → linear:
for i in 0..=255:
    c = i / 255.0
    if c <= 0.04045:
        linear_lut[i] = c / 12.92
    else:
        linear_lut[i] = ((c + 0.055) / 1.055) ^ 2.4

// linear → sRGB:
for i in 0..=255:
    c = i / 255.0
    if c <= 0.0031308:
        srgb_lut[i] = c * 12.92
    else:
        srgb_lut[i] = 1.055 * c^(1/2.4) - 0.055
```

Both conversion directions can be baked once at library initialisation and
reused for every image. The piecewise function, evaluated naively on every
pixel, takes ~10 ns per pixel on a modern CPU core; the LUT lookup takes
~1 ns.

**Inversion**:

```
lut[i] = 255 - i
```

**Sigmoid contrast enhancement** (S-curve):

```
f(x) = 1 / (1 + exp(−k × (x − 0.5)))   (logistic, centred at 0.5)
```

Controls: k governs the steepness of the S. k=4 is subtle; k=12 is aggressive.

**Equalization** (histogram equalisation):

```
Build the CDF of the image's histogram:
  hist[v] = count of pixels with value v
  cdf[v]  = Σ_{u=0}^{v} hist[u]
  cdf_min = first non-zero cdf entry

Equalisation mapping:
  lut[v] = round( (cdf[v] − cdf_min) / (W*H − cdf_min) × 255 )
```

This stretches the histogram to fill [0, 255], enhancing low-contrast images.

### 2.2 Floating-Point 1D LUT with Interpolation

For f32 images (linear-light pipeline), the input is in [0.0, 1.0]. A 1D LUT
with n+1 entries stores:

```
lut_f32[k] = f(k / n)    for k in 0..=n
```

Lookup for input x ∈ [0.0, 1.0]:

```
t = x * n
k = floor(t)                          // lower entry index
frac = t - k                          // fractional part ∈ [0, 1)
k = clamp(k, 0, n-1)
output = lerp(lut_f32[k], lut_f32[k+1], frac)
       = lut_f32[k] * (1 - frac) + lut_f32[k+1] * frac
```

Error analysis: for a function with second derivative bounded by |f''| ≤ M,
the linear interpolation error is:

```
|error| ≤ M / (8n²)
```

For sRGB gamma (M ≈ 60 near zero), n = 1024 gives error < 60 / (8×1024²) ≈
7×10⁻⁶ — well below the precision of f32 storage.

### 2.3 Channel Independence

A 1D LUT can be the **same** for all channels (e.g., overall gamma correction)
or **different** per channel (e.g., a colour grade that brightens reds while
leaving blues flat). The implementation stores up to three separate tables
(R_lut, G_lut, B_lut).

---

## 3. 3D LUTs

A 1D LUT maps each channel independently. A **3D LUT** maps an (R, G, B)
triple as a whole to a new (R′, G′, B′) triple:

```
lut3d : (R, G, B) → (R′, G′, B′)
```

This allows any colour remapping that cannot be expressed as three independent
channel curves — for example:

- Skin tones need to be warmer (shift orange hue) without affecting other colours
- Teal and orange film look: push blues/greens toward teal, reds/yellows toward
  orange
- Convert between device colour gamuts (sRGB → DCI-P3) where the mapping
  depends on all three channels jointly

### 3.1 The Lattice

A 3D LUT stores output values at a regularly-spaced **N×N×N lattice** of input
(R, G, B) points:

```
lattice[r][g][b] = (R′, G′, B′)

where r, g, b ∈ {0, 1, …, N−1}
input coordinates: R = r/(N−1),  G = g/(N−1),  B = b/(N−1)
```

Common lattice sizes:

```
N = 2  →   8 entries     (identity LUT, testing only)
N = 17 → 4913 entries    (cinema standard; 2017 ACES interchange)
N = 33 → 35937 entries   (high-precision colour grading)
N = 65 → 274625 entries  (reference; used in ICC workflows)
```

Total storage: N³ × 3 × 4 bytes (three f32 per lattice point).

```
N=17: 4913 × 12 bytes ≈ 58 KB     (fits in L1 cache)
N=33: 35937 × 12 bytes ≈ 431 KB   (fits in L2 cache)
N=65: 274625 × 12 bytes ≈ 3.2 MB  (L3 cache or main memory)
```

### 3.2 Trilinear Interpolation

To look up an arbitrary input (r, g, b) ∈ [0,1]³, locate the surrounding
lattice cube and interpolate:

```
Step 1: find the lattice cell

r_scaled = r * (N-1)
g_scaled = g * (N-1)
b_scaled = b * (N-1)

r0 = floor(r_scaled);  r1 = r0 + 1;  fr = r_scaled - r0
g0 = floor(g_scaled);  g1 = g0 + 1;  fg = g_scaled - g0
b0 = floor(b_scaled);  b1 = b0 + 1;  fb = b_scaled - b0

(clamp r0,g0,b0 to [0, N-2])

Step 2: fetch the 8 corners of the cube

c000 = lattice[r0][g0][b0]
c001 = lattice[r0][g0][b1]
c010 = lattice[r0][g1][b0]
c011 = lattice[r0][g1][b1]
c100 = lattice[r1][g0][b0]
c101 = lattice[r1][g0][b1]
c110 = lattice[r1][g1][b0]
c111 = lattice[r1][g1][b1]

Step 3: trilinear interpolation (blend along each axis in turn)

c00 = lerp(c000, c001, fb)    // blend along B axis
c01 = lerp(c010, c011, fb)
c10 = lerp(c100, c101, fb)
c11 = lerp(c110, c111, fb)

c0  = lerp(c00, c01, fg)      // blend along G axis
c1  = lerp(c10, c11, fg)

out = lerp(c0, c1, fr)        // blend along R axis
```

Each `lerp` is applied independently to the three output components (R′, G′, B′).

**Error of trilinear interpolation**: For a smooth colour grade, the
interpolation error between adjacent lattice points is proportional to the
second derivative of the mapping function and inversely proportional to N².
For N=33 the error is below perceptual threshold (ΔE < 0.5) for typical
photographic colour grades.

### 3.3 Tetrahedral Interpolation

Trilinear divides the unit cube into 8 regions (one per octant) and uses
trilinear blending. **Tetrahedral interpolation** (Sakamoto, 1996) divides
the cube into 6 tetrahedra and uses barycentric coordinates within the
tetrahedron containing the input point.

The 6 tetrahedra partition the cube based on the sorted order of (fr, fg, fb):

```
If fr ≥ fg ≥ fb:  vertices: (0,0,0) (1,0,0) (1,1,0) (1,1,1)
If fr ≥ fb ≥ fg:  vertices: (0,0,0) (1,0,0) (1,0,1) (1,1,1)
If fg ≥ fr ≥ fb:  vertices: (0,0,0) (0,1,0) (1,1,0) (1,1,1)
If fg ≥ fb ≥ fr:  vertices: (0,0,0) (0,1,0) (0,1,1) (1,1,1)
If fb ≥ fr ≥ fg:  vertices: (0,0,0) (0,0,1) (1,0,1) (1,1,1)
If fb ≥ fg ≥ fr:  vertices: (0,0,0) (0,0,1) (0,1,1) (1,1,1)
```

Within the matching tetrahedron, the output is a weighted sum of the 4 vertex
lattice values. The weights are the barycentric coordinates, which are simple
differences of the fractional parts.

Example (fr ≥ fg ≥ fb case):

```
w0 = 1 - fr
w1 = fr - fg
w2 = fg - fb
w3 = fb

out = w0 * c000 + w1 * c100 + w2 * c110 + w3 * c111
```

**Advantages over trilinear**:

1. **More accurate**: tetrahedral interpolation exactly reproduces affine
   (matrix) colour transforms because an affine transform is linear in 3D, and
   barycentric coordinates within a tetrahedron preserve linearity. Trilinear
   does not reproduce affine transforms exactly (the trilinear blend is a
   trilinear polynomial, not a linear one).

2. **Fewer memory accesses**: 4 lattice reads vs 8 for trilinear.

3. **Industry standard**: DaVinci Resolve, Nuke, and After Effects all use
   tetrahedral interpolation for 3D LUTs.

---

## 4. The .cube Format

The `.cube` format (originally from Iridas/Adobe, now de facto standard) is the
most widely supported LUT interchange format. It is a plain-text file.

### 4.1 1D .cube (CUBE_1D_SIZE)

```
# Optional comments start with #
TITLE "Example 1D LUT"
LUT_1D_SIZE 4           # number of entries per channel
# One line per entry: R G B  (0.0 to 1.0)
0.0 0.0 0.0
0.3 0.2 0.1
0.8 0.7 0.6
1.0 1.0 1.0
```

Each line gives the output (R′, G′, B′) for the corresponding input value.
Input values are implicitly `i / (N-1)` for i in 0..N.

### 4.2 3D .cube (LUT_3D_SIZE)

```
# 3D LUT example
TITLE "Film look"
LUT_3D_SIZE 4           # lattice dimension N; this gives a 4×4×4 = 64-entry LUT
DOMAIN_MIN 0.0 0.0 0.0  # optional: min input value per channel
DOMAIN_MAX 1.0 1.0 1.0  # optional: max input value per channel

# Entries listed B-major, G-medium, R-slow:
# i.e., first iterate b from 0 to N-1, then g, then r
# r=0,g=0,b=0:
0.000000 0.000000 0.000000
# r=0,g=0,b=1:
0.000000 0.000000 0.250000
# ...
1.000000 1.000000 1.000000
```

**Critical detail — axis ordering**: in the `.cube` format the entries are
ordered with the **B axis varying fastest**, then G, then R:

```
for r in 0..N:
    for g in 0..N:
        for b in 0..N:
            write entry for (R = r/(N-1), G = g/(N-1), B = b/(N-1))
```

When reading a `.cube` file, populate the lattice with this ordering. Reversing
the axis order is a common bug that produces a transposed LUT (the colour grade
is still consistent at lattice corners but interpolates incorrectly elsewhere).

### 4.3 DOMAIN_MIN and DOMAIN_MAX

These optional fields extend the input domain beyond [0, 1]. For example, a
LUT that operates in scene-linear light (HDR) might specify:

```
DOMAIN_MIN 0.0 0.0 0.0
DOMAIN_MAX 16.0 16.0 16.0
```

Before looking up, remap the input:

```
r_normalised = (r_input - domain_min.R) / (domain_max.R - domain_min.R)
```

then proceed with the standard [0,1] lookup. Clamp normalised values to [0,1].

---

## 5. The Identity LUT

An identity LUT maps every colour to itself. It is the baseline and the
starting point for LUT editing tools.

### 1D identity LUT

```
lut[i] = i / (N - 1)    // output equals input for every entry
```

### 3D identity LUT

```
lattice[r][g][b] = (r/(N-1), g/(N-1), b/(N-1))    // output equals input
```

A useful debugging tool: generate an identity 3D LUT, write it to a `.cube`
file, import it into the application under test, and verify that the output
image matches the input pixel-for-pixel. Any deviation indicates a bug in the
LUT parsing or application code.

---

## 6. LUT Composition

Two LUTs can be **composed** (baked together) into a single LUT:

```
(A after B)(x) = A(B(x))
```

To compose a 3D LUT A and a 3D LUT B (both size N³) into a new LUT C:

```
for r in 0..N:
    for g in 0..N:
        for b in 0..N:
            input = (r/(N-1), g/(N-1), b/(N-1))
            intermediate = B.lookup(input)            // apply B to lattice point
            C.lattice[r][g][b] = A.lookup(intermediate)  // then apply A
```

Composition avoids a double lookup at runtime: instead of applying LUT B then
LUT A to every pixel, apply the composed LUT C once.

**Precision note**: composition introduces interpolation errors twice (once
when evaluating B at the lattice point, once when evaluating A). For N=33 this
is acceptable; for N=17 use N=33 or 65 for the composed LUT to keep errors low.

---

## 7. GPU LUTs

On modern GPUs, a 3D LUT maps directly to a **3D texture**:

```
CPU:
  Upload lattice data as a 3D texture of size N×N×N, format RGB32F.

GPU shader (WGSL):
  @group(0) @binding(0) var lut_texture : texture_3d<f32>;
  @group(0) @binding(1) var lut_sampler : sampler;

  fn apply_lut(colour: vec3<f32>) -> vec3<f32> {
      // Scale from [0,1] to [0.5/N, 1 - 0.5/N] to avoid edge clamping
      let n = f32(textureDimensions(lut_texture).x);
      let uv = colour * (n - 1.0) / n + 0.5 / n;
      return textureSample(lut_texture, lut_sampler, uv).rgb;
  }
```

The GPU's texture hardware performs **trilinear interpolation in hardware** at
effectively zero cost — it is the same operation as trilinear texture mapping,
which all GPUs are optimised for.

Performance: applying a 33³ 3D LUT to a 4K image (3840×2160 ≈ 8 million pixels)
takes < 1 ms on a mid-range GPU (2023), compared to ~100 ms on a single CPU
core. The GPU turns LUT application into a free operation in a real-time
pipeline.

### Sampler configuration

For trilinear interpolation on the GPU, configure the sampler with:

```
WGSL sampler descriptor:
  addressModeU = clamp-to-edge
  addressModeV = clamp-to-edge
  addressModeW = clamp-to-edge
  magFilter    = linear
  minFilter    = linear
```

The `linear` filter enables the hardware trilinear interpolation. `nearest`
would give nearest-lattice-point lookup (incorrect for smooth LUTs).

**Note on WebGPU**: `texture_3d` with `sampler` and linear filtering is
supported since WebGPU spec 1.0. The same WGSL code works in both the browser
(WebGPU) and in Rust via wgpu (targeting Metal / Vulkan / DX12).

---

## 8. Precision and Colour Accuracy

### 8.1 Floating-point LUT storage

Store lattice values as f32 (or f16 if memory is constrained). The f32 format
has 24-bit mantissa ≈ 7 decimal digits of precision. For a normalised [0,1]
output this is far more than required.

Avoid u8 LUT storage for 3D LUTs: u8 has only 8-bit per channel precision
(256 levels), introducing quantisation error up to 1/255 ≈ 0.004 before
interpolation. For creative colour grades this is typically invisible, but for
technical workflows (ICC profile conversion) it fails the accuracy requirements.

### 8.2 Input domain clamping

Always clamp the input to [domain_min, domain_max] before lookup. Out-of-domain
input values (from HDR images or negative values in a linear-light pipeline)
must be clamped, not wrapped, to the nearest domain boundary.

### 8.3 Applying LUTs to sRGB images

A 3D LUT designed for a **linear-light** pipeline should only be applied to
linear-light image data. Applying it to an sRGB-encoded image without prior
gamma decode produces incorrect results (the LUT "sees" the gamma-encoded
values, not the physical colours).

Correct workflow for film look on an sRGB JPEG:

```
1. Decode JPEG → sRGB u8 image
2. sRGB → linear f32 (apply linear_lut from §2.1)
3. Apply 3D LUT (in linear light)
4. linear → sRGB f32 (apply sRGB encode)
5. Clamp to [0,1], convert to u8, encode JPEG
```

Some `.cube` files are designed for sRGB input (they have the gamma decode
baked into the lattice). Such LUTs are typically labelled with "sRGB input" or
"log input" in their TITLE line.

---

## 9. Interface

Every implementation package for IMG02 must provide:

```
// 1D LUT — 256 entry (u8 domain)
type Lut1dU8 = [u8; 256]

fn build_gamma_lut(gamma: f32) -> Lut1dU8
fn build_brightness_contrast_lut(brightness: f32, contrast: f32) -> Lut1dU8
fn build_srgb_to_linear_lut() -> [f32; 256]
fn build_linear_to_srgb_lut() -> [f32; 256]
fn apply_lut1d_u8(src: &Image<RGB8>, r: &Lut1dU8, g: &Lut1dU8, b: &Lut1dU8) -> Image<RGB8>

// 1D LUT — floating-point domain
type Lut1dF32 = Vec<f32>   // n+1 entries; input domain [0,1]

fn build_lut1d_f32(n: usize, f: impl Fn(f32) -> f32) -> Lut1dF32
fn lookup_lut1d_f32(lut: &Lut1dF32, x: f32) -> f32   // linear interpolation
fn apply_lut1d_f32(src: &Image<RGB32F>, lut: &Lut1dF32) -> Image<RGB32F>

// 3D LUT
struct Lut3d {
    size:         usize,          // N (lattice dimension)
    domain_min:   [f32; 3],
    domain_max:   [f32; 3],
    lattice:      Vec<[f32; 3]>,  // N^3 entries, B-major ordering
}

fn parse_cube(data: &str) -> Result<Lut3d, ParseError>
fn write_cube(lut: &Lut3d) -> String
fn identity_lut3d(size: usize) -> Lut3d
fn compose_luts(outer: &Lut3d, inner: &Lut3d) -> Lut3d
fn lookup_trilinear(lut: &Lut3d, r: f32, g: f32, b: f32) -> [f32; 3]
fn lookup_tetrahedral(lut: &Lut3d, r: f32, g: f32, b: f32) -> [f32; 3]
fn apply_lut3d(src: &Image<RGB32F>, lut: &Lut3d, method: Interpolation) -> Image<RGB32F>
```

---

## Appendix A: LUT File Format Comparison

| Format   | Ext      | Dim  | Max Size | Organisation                | Notes                        |
|----------|----------|------|----------|-----------------------------|------------------------------|
| .cube    | .cube    | 1D,3D| 65³      | Text, B-major               | Adobe/Iridas; industry std   |
| .3dl     | .3dl     | 3D   | 64³      | Text, float or u16          | Autodesk Lustre               |
| .csp     | .csp     | 1D,3D| —        | Text, Adobe/Cineform variant | Pre-shaper + 3D cube         |
| .lut     | .lut     | 1D   | —        | Text, old After Effects      | Rarely used                   |
| CLF      | .xml     | 1D,3D| —        | XML (ACES Common LUT Format) | The eventual open standard    |

This series implements `.cube` only (the universal de facto standard). CLF
(Academy S-2014-006) is the eventual open replacement and may be added later.

---

## Appendix B: Example .cube — Teal-and-Orange Film Look

The "teal and orange" look is the most recognisable film grade of the 2010s
blockbuster era: push midtone blues/greens toward teal, push midtone
reds/yellows toward warm orange. Skin tones (which are orange-adjacent) are
accentuated; the complementary teal shadows create visual contrast.

A 4×4×4 identity LUT with manual teal-and-orange adjustments at a few
key lattice points illustrates the `.cube` structure:

```
TITLE "Teal and Orange (pedagogical 4^3)"
LUT_3D_SIZE 4
# axes: R slow, G medium, B fast
# Lattice at (0/3, 0/3, 0/3) = (0.0, 0.0, 0.0): black → black
0.000 0.000 0.000
# (0/3, 0/3, 1/3): dark blue → pushed toward teal (add green)
0.000 0.050 0.333
# (0/3, 0/3, 2/3): medium blue → teal
0.000 0.200 0.667
# (0/3, 0/3, 3/3): (0, 0, 1) pure blue → cooler teal
0.000 0.333 0.900
# …
# (3/3, 1/3, 0/3): red-orange → pushed warmer
1.080 0.250 0.000
# (3/3, 3/3, 3/3): (1, 1, 1) white → white (no cast in highlights)
1.000 1.000 1.000
```

A real creative LUT would have entries for all N³ lattice points; this
abbreviated example is for illustration only.
