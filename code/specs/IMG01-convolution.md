# IMG01 — Convolution and Spatial Filters

## Overview

A **spatial filter** transforms each pixel of an image by computing a weighted
combination of its neighbours. The filter is described by a small 2D matrix of
weights called a **kernel** (or *convolution kernel*, *filter mask*). The kernel
is slid across every pixel of the image; at each position the weights multiply
the neighbourhood pixel values and the products are summed to produce one output
value.

```
Input image I  ──────────────────────────────────► Output image O
                 kernel K slides over every (x,y)
                 O(x,y) = weighted sum of neighbourhood
```

This operation is called **discrete 2D convolution** (in practice, most
libraries implement cross-correlation, which differs only in the sign convention
for the kernel indices — discussed in §3).

Spatial convolution underlies a huge fraction of image processing:

| Goal                         | Kernel type              |
|------------------------------|--------------------------|
| Smooth / denoise             | Gaussian, box blur       |
| Detect edges                 | Sobel, Prewitt, Canny    |
| Measure curvature            | Laplacian, LoG           |
| Sharpen                      | Unsharp mask             |
| Emboss / stylise             | Emboss kernel            |
| Feature extraction (CNNs)    | Learned kernels           |

This spec covers: the mathematical definition, padding modes, the separable
kernel optimisation, a library of standard kernels with examples, precision
rules, and the computational complexity analysis that motivates GPU offloading
(IMG06).

---

## 1. Motivation: Why Weighted Neighbourhoods?

Before diving into the formula, consider what we want to achieve.

### Smoothing (noise reduction)

Raw sensor output from a camera contains random per-pixel noise: each pixel
deviates from the "true" colour by a small random amount. If we replace each
pixel with the **average** of its neighbours, random high-frequency fluctuations
cancel out while the underlying image — which changes slowly — is preserved.

```
Before (noisy):   10  12  200  11  9   ← one noisy pixel (200)
Box average (3):  (10+12+200)/3 ≈ 74   ← still high, but attenuated
```

A plain average (box blur) is the simplest kernel. A **Gaussian-weighted**
average gives more weight to the centre pixel and less to the periphery,
preserving the image better while still smoothing.

### Edge detection

An edge is a location where the image value changes rapidly. The **gradient**
of the image — the rate of change in x and y — is high at edges and near-zero
in flat regions. A kernel that approximates the partial derivative ∂I/∂x
highlights vertical edges; ∂I/∂y highlights horizontal edges.

```
Smooth region:   100 100 100 101 100   gradient ≈ 0
Edge:            10  10  10  90  90    gradient ≈ 80  ← large change
```

### Sharpening

Sharpening inverts the blur idea: add a scaled version of the high-frequency
content back to the image. The Laplacian measures high-frequency content;
subtracting it from the image enhances edges.

```
Sharpened = I − λ × Laplacian(I)   (λ > 0)
```

All three goals follow from the same kernel machinery.

---

## 2. The Kernel

A kernel K is a 2D array of real-valued weights. We index it with coordinates
centred at (0, 0):

```
K has size (2r+1) × (2r+1), radius r.

   col:  -r  ...  0  ...  +r
row -r: [ K(-r,-r) ... K(0,-r) ... K(r,-r) ]
   ...
row  0: [ K(-r, 0) ... K( 0, 0) ... K(r, 0) ]
   ...
row +r: [ K(-r,+r) ... K(0,+r) ... K(r,+r) ]
```

Common kernel sizes: 3×3 (r=1), 5×5 (r=2), 7×7 (r=3). For separable
Gaussian blurs, larger sizes (11×11, 15×15) are common.

### Kernel visualisation conventions

When kernels are written as boxes in code or diagrams, the top-left cell is
(−r, −r) and the bottom-right is (+r, +r). The centre cell (0, 0) is the
kernel element that aligns with the current pixel (x, y).

```
3×3 kernel stored as a flat array [k0, k1, …, k8]:

  k0  k1  k2       K(-1,-1)  K(0,-1)  K(1,-1)
  k3  k4  k5   =   K(-1, 0)  K(0, 0)  K(1, 0)
  k6  k7  k8       K(-1, 1)  K(0, 1)  K(1, 1)
```

In memory the kernel is typically stored row-by-row (row-major), top to bottom.

---

## 3. The Discrete Convolution Formula

For a kernel K of radius r, the **2D cross-correlation** of image I at pixel
(x, y) is:

```
O(x, y) = Σᵢ₌₋ᵣ^r  Σⱼ₌₋ᵣ^r  K(i, j) × I(x+i, y+j)
```

The output pixel is a weighted sum of the (2r+1)² pixels in the neighbourhood
centred at (x, y).

### Cross-correlation vs. true convolution

True mathematical 2D convolution flips the kernel:

```
(K ★ I)(x, y) = Σᵢ Σⱼ  K(i, j) × I(x−i, y−j)
```

The only difference is the sign of the offsets into I. For **symmetric**
kernels (K(i,j) = K(−i,−j)), which includes all the common blur and edge
kernels in §7, the two definitions produce identical output. For asymmetric
kernels (e.g., Sobel with explicit directionality), the distinction matters.

In this series we implement **cross-correlation** (matching the convention of
most frameworks: PyTorch, TensorFlow, PIL, OpenCV) and call it convolution
throughout. If a future package requires true convolution with an asymmetric
kernel, flip the kernel indices before calling the standard routine.

### Worked example: 3×3 box blur on a grayscale patch

```
Input patch (5×5 u8):
   80  82  85  83  80
   81  90 200  88  79    ← single "hot" pixel at (2,1)
   82  85  87  84  81
   80  83  86  85  82
   79  81  84  82  80

Box blur kernel (3×3), all weights = 1/9:
   1/9  1/9  1/9
   1/9  1/9  1/9
   1/9  1/9  1/9

Output at centre pixel (2,2):
  O(2,2) = (90 + 200 + 88 + 85 + 87 + 84 + 85 + 87 + 84) / 9
          = 790 / 9 ≈ 87.8  → 88

The hot pixel (200) has been strongly attenuated. The surrounding pixels
are barely changed.
```

---

## 4. Padding Modes

The kernel extends r pixels beyond each edge. For the pixels in the r-wide
border of the image, some kernel positions would fall outside the image
boundaries. The **padding mode** specifies what value to use for those
out-of-bounds positions.

Let W, H be image dimensions. For pixel (x, y) and offset (i, j), the
out-of-bounds case occurs when x+i < 0, x+i ≥ W, y+j < 0, or y+j ≥ H.

### Zero / constant padding

```
I(x, y) = C  for (x, y) outside image bounds  (C = 0 by default)
```

The image is treated as if surrounded by a constant-colour border.

```
Image row: [A, B, C, D, E]  with r=1

Extended:  [0, A, B, C, D, E, 0]
```

Pros: simple. Cons: creates a dark (or coloured) halo around the image edge.
Best for: edge detection (the artificial border produces a strong edge
response, which is usually acceptable since image boundaries are not subject
to the same processing rules as interior content).

### Replicate (clamp-to-edge)

```
I(x, y) clamps:  x → clamp(x, 0, W-1),  y → clamp(y, 0, H-1)
```

The edge pixel is repeated indefinitely outside the boundary.

```
Extended:  [A, A, B, C, D, E, E]
```

Pros: no border artefact in smooth regions. Best for: blur (avoids the
dark halo from zero-padding).

### Reflect

```
I(x, y) reflects:  x → reflect(x, 0, W-1)
where reflect(v, lo, hi):
    period = 2*(hi - lo)
    v = ((v - lo) mod period + period) mod period
    if v > hi - lo: v = period - v
    return v + lo
```

The image tiles by mirroring at its boundary (without repeating the edge pixel).

```
Original:       [A, B, C, D, E]
Extended:  [C, B, A, B, C, D, E, D, C]
                    ─── reflect at left ───
```

Pros: smooth continuation; no artificial edge. Best for: textures, seamless
patterns, and any filter where you need the derivative at the boundary to match
the interior.

### Wrap (periodic / torus)

```
I(x, y) wraps:  x → x mod W,  y → y mod H  (with correct handling of negatives)
```

The image tiles to fill the plane.

```
Original:      [A, B, C, D, E]
Extended: [D, E, A, B, C, D, E, A, B]
```

Pros: mathematically exact for FFT-based filtering (where the DFT assumes
periodic input). Best for: frequency-domain operations, tileable textures.

### Choosing a padding mode

```
Use case                         Recommended mode
──────────────────────────────────────────────────
General blur / smoothing         Replicate (no edge halo)
Edge detection                   Zero (boundary gives strong response)
Seamless texture processing      Reflect
FFT-paired spatial filtering     Wrap
Neural network convolutions      Zero (PyTorch / TensorFlow default)
```

---

## 5. Separable Kernels

A 2D kernel K is **separable** if it can be written as the outer product of two
1D vectors:

```
K(i, j) = h(i) × v(j)

where h is a 1D horizontal filter (length 2r+1)
      v is a 1D vertical filter   (length 2r+1)
```

If K is separable, the 2D convolution decomposes into two sequential 1D passes:

```
Step 1 (horizontal):  T(x, y) = Σᵢ h(i) × I(x+i, y)
Step 2 (vertical):    O(x, y) = Σⱼ v(j) × T(x, y+j)
```

### Why this matters

The cost of a direct 2D convolution on an W×H image with a (2r+1)×(2r+1)
kernel is:

```
ops_2d = W × H × (2r+1)²
```

The cost of two 1D passes is:

```
ops_1d = W × H × (2r+1)  [horizontal]
       + W × H × (2r+1)  [vertical]
       = 2 × W × H × (2r+1)
```

The ratio:

```
ops_2d / ops_1d = (2r+1)² / (2 × (2r+1)) = (2r+1) / 2

r = 1  (3×3  kernel):  1.5× speedup
r = 2  (5×5  kernel):  2.5× speedup
r = 4  (9×9  kernel):  4.5× speedup
r = 7  (15×15 kernel): 7.5× speedup
```

For large blur kernels the speedup is substantial.

### Proof of separability for the Gaussian kernel

The 2D Gaussian with standard deviation σ is:

```
G(i, j) = (1 / (2πσ²)) × exp(−(i² + j²) / (2σ²))
```

By the rules of exponentials:

```
G(i, j) = (1 / √(2πσ²)) × exp(−i² / (2σ²))
         × (1 / √(2πσ²)) × exp(−j² / (2σ²))
         = g(i) × g(j)
```

where g(x) = (1/√(2πσ²)) × exp(−x²/(2σ²)) is the 1D Gaussian. The 2D
Gaussian is the outer product of two identical 1D Gaussians — it is separable.

### Generating Gaussian 1D kernel coefficients

For a kernel of half-width r (total width 2r+1), sample the 1D Gaussian at
integer positions and normalise:

```
k[i] = exp(−i² / (2σ²))    for i ∈ {−r, …, +r}
k[i] /= Σ k[i]             (normalise so weights sum to 1)
```

Choosing σ given r: a common heuristic is σ = (2r+1) / 6 so that ±3σ ≈ ±r
(the kernel captures ~99.7% of the Gaussian mass). Another heuristic used by
OpenCV: σ = 0.3 × (r − 1) + 0.8.

Example for r=1 (3-element kernel), σ = 1.0:

```
k[-1] = exp(−1/2) ≈ 0.6065
k[ 0] = exp(0)   = 1.0000
k[+1] = exp(−1/2) ≈ 0.6065

sum = 2.2130
normalised: [0.274, 0.452, 0.274]  (matches the [1, 2, 1]/4 Pascal approximation)
```

---

## 6. Padding and Border Handling in the Separable Pass

When applying the horizontal pass first, the intermediate image T has the same
dimensions as I. The border handling is applied independently in each pass:

- Horizontal pass: only the left and right edges require clamping/padding.
- Vertical pass: only the top and bottom edges require clamping/padding.

This keeps the implementation simple: the 1D convolution routine takes a
padding mode parameter and handles it uniformly.

---

## 7. Standard Kernel Library

### 7.1 Box blur

Uniform average over a (2r+1)×(2r+1) neighbourhood:

```
K(i, j) = 1 / (2r+1)²    for all (i, j)

3×3 box blur:
  1/9  1/9  1/9
  1/9  1/9  1/9
  1/9  1/9  1/9
```

Not separable in the Gaussian sense, but separable as [1,1,1]/3 × [1,1,1]/3
(horizontal pass of uniform 1/3, then vertical). Fast in integer arithmetic:
use a sliding-window sum that subtracts the leaving column and adds the
entering column for O(1) amortised cost per pixel.

Effect: strong blur, equal-weight average of the neighbourhood. Box blurs
are not ideal because they have significant high-frequency ringing (their
frequency response has large sidelobes). Gaussian blurs are preferable for
anything that will be viewed by a human; box blurs are fine for preprocessing
pipelines where exact visual quality is less important.

### 7.2 Gaussian blur

Described in §5. The canonical smoothing kernel.

```
3×3, σ≈0.85 (normalised):
  0.0625  0.125  0.0625
  0.125   0.25   0.125
  0.0625  0.125  0.0625

Often written as integer approximation [1 2 1 ; 2 4 2 ; 1 2 1] / 16
```

Effect: smooth, isotropic blur. The frequency-domain interpretation: a Gaussian
kernel is a **low-pass filter** that multiplies each frequency component by
exp(−2π²σ²f²) — it perfectly attenuates high frequencies (sharp detail) while
leaving low frequencies (gradual colour changes) intact.

Increasing σ increases the blur radius. The bandwidth of the resulting
low-pass filter decreases with σ.

### 7.3 Sobel edge detector

Approximates the image gradient ∂I/∂x (horizontal) and ∂I/∂y (vertical).

```
Sobel horizontal Kₓ (detects vertical edges, gradient in x direction):
  -1   0  +1
  -2   0  +2
  -1   0  +1

Sobel vertical Kᵧ (detects horizontal edges, gradient in y direction):
  -1  -2  -1
   0   0   0
  +1  +2  +1
```

Note: Kₓ and Kᵧ are separable:

```
Kₓ = [+1, 0, −1]ᵀ × [1, 2, 1]     (outer product: vertical smoothing × horizontal differentiation)
Kᵧ = [1, 2, 1]ᵀ  × [+1, 0, −1]   (outer product: vertical differentiation × horizontal smoothing)
```

The magnitude of the gradient (edge strength) and its direction (edge angle):

```
Gₓ = Kₓ ★ I
Gᵧ = Kᵧ ★ I

magnitude  M(x,y) = √(Gₓ² + Gᵧ²)   (or |Gₓ| + |Gᵧ| for speed)
direction  θ(x,y) = atan2(Gᵧ, Gₓ)   in radians
```

Example: vertical edge between left dark region and right bright region:

```
Input (grayscale):
  10  10  10   90  90
  10  10  10   90  90
  10  10  10   90  90

Kₓ response at the centre column (x=2):
  −1×10 + 0×10 + 1×90  = 80   ← large response: vertical edge
  −1×10 + 0×10 + 1×90  = 80
  −1×10 + 0×10 + 1×90  = 80

Kᵧ response at the centre row:
  all rows are identical, so vertical difference = 0   ← no horizontal edge
```

### 7.4 Prewitt edge detector

A simpler alternative to Sobel (no centre weighting):

```
Prewitt Kₓ:          Prewitt Kᵧ:
  -1  0  +1            -1  -1  -1
  -1  0  +1             0   0   0
  -1  0  +1            +1  +1  +1
```

Slightly noisier than Sobel (no smoothing in the perpendicular direction) but
marginally faster (all non-zero weights are ±1 — no multiplication needed,
only addition/subtraction).

### 7.5 Laplacian

The Laplacian ∇²I = ∂²I/∂x² + ∂²I/∂y² measures the **curvature** of the
image: it is zero in flat regions, large in magnitude at edges, and changes
sign across an edge. Because the Laplacian is positive inside a bright blob
and negative outside (or vice versa), zero-crossings of the Laplacian are
precise edge locations.

```
4-connected Laplacian:        8-connected Laplacian:
   0   1   0                   1   1   1
   1  -4   1                   1  -8   1
   0   1   0                   1   1   1
```

The 4-connected version approximates ∂²I/∂x² + ∂²I/∂y² using only axial
neighbours. The 8-connected version also includes diagonal neighbours,
making it more isotropic (less orientation-dependent).

Effect: highlights regions of rapid change. Flat regions → 0. Edges →
large positive or negative values. The output must be treated as a signed
quantity (use i16 or f32, not u8).

### 7.6 Laplacian of Gaussian (LoG)

The LoG combines Gaussian smoothing (to suppress noise) with the Laplacian
(to detect edges). Apply Gaussian first, then Laplacian. The result is
equivalent to a single kernel:

```
LoG(x,y) = −(1/(πσ⁴)) × (1 − (x²+y²)/(2σ²)) × exp(−(x²+y²)/(2σ²))
```

Commonly approximated as a 5×5 or 9×9 kernel. The kernel has a characteristic
"Mexican hat" profile: negative in the centre, positive ring around it, then
zero beyond.

```
LoG approximation (5×5, σ≈1.4):
   0   0  −1   0   0
   0  −1  −2  −1   0
  −1  −2  16  −2  −1
   0  −1  −2  −1   0
   0   0  −1   0   0
```

Zero-crossings of the LoG response mark edge positions.

### 7.7 Sharpen (Laplacian subtraction)

Sharpening adds back high-frequency content that was attenuated by blur or
camera optics. The standard approach: compute the Laplacian, then subtract it
scaled by a strength parameter λ.

```
O = I − λ × ∇²I

Equivalent single kernel (λ=1, 4-connected Laplacian):
   0  -1   0
  -1   5  -1
   0  -1   0

= identity − Laplacian(4-connected)
= [0,0,0; 0,1,0; 0,0,0] − [0,1,0; 1,-4,1; 0,1,0]
= [0,-1,0; -1,5,-1; 0,-1,0]  ✓
```

Higher λ → more aggressive sharpening → more haloing around edges.
λ = 0.5 is a gentle sharpen; λ = 2–3 is noticeable. Beyond λ ≈ 4 on typical
images, the output begins to look over-sharpened with strong ringing.

### 7.8 Unsharp mask (USM)

The "unsharp mask" name is historical (it originated in darkroom photography
where a blurred — "unsharp" — copy of the negative was used as a mask). The
algorithm:

```
O = I + k × (I − blur(I, σ))   for some strength k > 0
```

`I − blur(I, σ)` is the **high-pass residual**: the detail the blur removed.
Adding it back, scaled by k, enhances fine detail without the ringing of the
direct Laplacian subtraction.

Unsharp mask is separable (blur is separable; subtraction and addition are
point operations). Typical parameters: σ = 1.0–2.0, k = 0.5–2.0.

### 7.9 Emboss

Highlights edges in a single diagonal direction, creating the appearance of
a raised relief carved into metal:

```
Emboss (top-left light source):
  -2  -1   0
  -1   1   1
   0   1   2
```

After applying, add 128 (or 0.5 for float) to bias the zero-centred output
into a displayable [0, 255] range. Flat regions → 128 (grey). Edges facing
the light source → brighter; edges facing away → darker.

---

## 8. Multi-Channel Images

For RGB or RGBA images, apply the kernel **independently to each channel**:

```
O.R(x,y) = Σᵢ Σⱼ K(i,j) × I.R(x+i, y+j)
O.G(x,y) = Σᵢ Σⱼ K(i,j) × I.G(x+i, y+j)
O.B(x,y) = Σᵢ Σⱼ K(i,j) × I.B(x+i, y+j)
```

Exception: **alpha-premultiplied** images require special treatment. If an
image uses pre-multiplied alpha, blur the RGB channels *as-is* (they already
encode alpha-weighted colour). Blurring straight-alpha RGB separately then
re-compositing produces colour fringing at semi-transparent edges.

For edge detection on colour images, a common approach: convert to
luminance first (Y = 0.2126R + 0.7152G + 0.0722B in linear light), detect
edges on Y, then optionally combine with colour-channel gradient magnitudes.

---

## 9. Precision Rules

Building on IMG00 §7:

1. **Convert to linear light** before filtering. Gaussian blur in sRGB space
   darkens edges because the gamma curve is non-linear. The error is subtle but
   visually measurable on strong gradients.

2. **Accumulate in f32**. Even for u8 input images, load each channel as f32
   before the kernel multiply-accumulate. Store back to u8 at the end.

3. **Clamp, not wrap**. After accumulation, clamp to [0, 255] before casting
   to u8. Edge detection outputs (signed) may intentionally exceed [0, 255];
   those should stay in f32/i16 until visualised.

4. **Normalise kernels** for blurs. A blur kernel whose weights sum to 1.0
   preserves the overall image brightness. Unnormalised kernels (like the raw
   integer Sobel) should not be normalised — their outputs are gradient
   measurements, not brightness.

---

## 10. Computational Complexity

For an W×H image and a kernel of radius r:

| Algorithm              | Operations per pixel | Total                    | Notes                     |
|------------------------|---------------------|--------------------------|---------------------------|
| Direct 2D              | (2r+1)²             | WH(2r+1)²               | Baseline                  |
| Separable 1D (2 pass)  | 2(2r+1)             | 2WH(2r+1)               | Requires separable kernel |
| Box blur (sliding sum) | O(1)                | O(WH)                   | Only for uniform weights  |
| FFT-based              | —                   | O(WH log(WH))           | Good for r > ~30          |

Breakeven for FFT vs separable: roughly when 2r+1 > log₂(WH)/2. For a 1024×1024
image (log₂ ≈ 20), breakeven is around r ≈ 5 (11×11 kernel). In practice FFT
overhead (forward and inverse transforms, complex multiply) means the breakeven
is closer to r ≈ 15 on modern hardware with SIMD.

---

## 11. GPU Acceleration (Forward Reference)

A spatial convolution is embarrassingly parallel at the pixel level: each output
pixel is independent. The GPU acceleration layer (IMG06) provides WGSL compute
shaders for:

- Generic 2D kernel convolution up to 15×15
- Separable Gaussian blur (two 1D passes)
- Sobel edge detection (returns two output textures: Gₓ and Gᵧ)
- LoG

The GPU versions share the same precision rules (f32 intermediate) and padding
mode interface as the CPU implementations. The CPU and GPU implementations are
tested against each other on a suite of reference images to guarantee identical
numerical output (within f32 rounding).

---

## 12. Interface

Every implementation package for IMG01 must provide the following operations
(pseudocode):

```
// Apply a 2D kernel to a single-channel or multi-channel image.
// kernel: flat array of (2r+1)*(2r+1) f32 weights, row-major, indexed [−r..r, −r..r]
// padding: Zero | Replicate | Reflect | Wrap
fn convolve2d(src: &Image<P>, kernel: &[f32], radius: u32, padding: PaddingMode) -> Image<P>

// Separable convolution (two 1D passes). h_kernel and v_kernel have length 2*radius+1.
fn convolve_separable(src: &Image<P>, h: &[f32], v: &[f32], radius: u32, padding: PaddingMode) -> Image<P>

// Convenience wrappers built on convolve_separable or convolve2d:
fn gaussian_blur(src: &Image<P>, sigma: f32, padding: PaddingMode) -> Image<P>
fn box_blur(src: &Image<P>, radius: u32, padding: PaddingMode) -> Image<P>
fn sobel(src: &Image<Luma32F>) -> (Image<Luma32F>, Image<Luma32F>)   // returns (Gx, Gy)
fn laplacian(src: &Image<Luma32F>, connected: Connectivity) -> Image<Luma32F>
fn sharpen(src: &Image<P>, lambda: f32) -> Image<P>
fn unsharp_mask(src: &Image<P>, sigma: f32, strength: f32) -> Image<P>
```

---

## Appendix A: Relationship to CNN Convolutions

Neural-network libraries (PyTorch, TensorFlow, JAX) expose a `conv2d` operation
that is almost identical to what this spec defines, with these differences:

| Dimension     | This spec             | CNN conv2d                                      |
|---------------|-----------------------|-------------------------------------------------|
| Input shape   | H × W × C            | N × C_in × H × W (batch-first, channels-first) |
| Kernel shape  | (2r+1) × (2r+1)       | C_out × C_in × kH × kW (learned, cross-channel) |
| Weights       | Fixed, hand-designed  | Learned during training                         |
| Bias          | None (or baked in)    | Separate per-output-channel bias term           |
| Stride        | Always 1              | Configurable stride for downsampling            |
| Dilation      | Always 1              | Configurable dilation for atrous convolution    |

The math is the same; the machinery around it differs. The kernels defined in
§7 can be loaded directly into a CNN's first fixed-weight convolutional layer
as **feature extractors**, and the output activations will be exactly the
images described in this spec.

---

## Appendix B: Frequency Domain Interpretation

Every kernel has a **frequency response** — the factor by which it multiplies
each spatial frequency when applied to the image. This is the magnitude of the
kernel's 2D DFT.

```
Kernel type        Frequency response
─────────────────────────────────────────────────────────
Box blur           Sinc function — good suppression of high freq,
                   but significant ringing (sidelobes)
Gaussian blur      Gaussian — monotone rolloff, no ringing;
                   low-pass filter with bandwidth ∝ 1/σ
Laplacian          ∝ f² — high-pass filter; zero DC response
Sobel (Kₓ)        ∝ sin(2πfₓ) — differentiator in x, smoother in y
Identity           Flat (1 everywhere) — all-pass, no change
```

The frequency-domain view explains why Gaussian blur is preferred over box blur:
the Gaussian has no sidelobes in the frequency domain, so it does not introduce
any ringing artefacts. Box blur's sinc frequency response has sidelobes that
cause "ringing" — faint copies of edges — visible as concentric bright/dark bands
around hard boundaries when the blur radius is large.
