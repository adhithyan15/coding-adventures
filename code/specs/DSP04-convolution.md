# DSP04 — Convolution / Image Filters

**Status**: V1 spec (this document is Phase 0).

**Scope**: a new `dsp-conv` crate providing same-size 1-D and
2-D convolution with the standard boundary modes plus the
classical image filter design helpers (Gaussian blur, Sobel
edge detection, box blur, Laplacian, sharpen).  Built on top
of `dsp-fft` for FFT-based long-kernel paths in later phases.

## Why a convolution layer?

`dsp-filters::fir` (DSP03) already does 1-D linear convolution
— but it's the "full" mode (length `N + K - 1` output, zero
padding implicit) which is wrong for image filtering: you want
the *same-size* output, and you need control over the boundary
extension.

DSP04 fills that gap with a separate, image-and-signal-friendly
API:

- **Same-size output.**  `conv1d(x, h) → Vec<f32>` of length
  `N` (not `N + K - 1`).
- **Explicit boundary modes.**  `Zero`, `Replicate`, `Reflect`,
  `Wrap` — match `scipy.ndimage.convolve(mode=...)` and
  `numpy.convolve(mode=...)` conventions.
- **2-D convolution.**  `conv2d(image, kernel, ...)` for image
  filters.  `sep_conv2d(image, h_kernel, v_kernel, ...)` for
  the row-then-column fast path on separable kernels (the most
  common case in practice).
- **Design helpers.**  `gaussian_blur_kernel`, `sobel_x_kernel`,
  `box_blur_kernel`, `laplacian_kernel`, `sharpen_kernel` —
  the canonical image-processing primitives.

This unlocks the OpenCV-equivalent surface for image
preprocessing: blur, sharpen, edge detection, denoising.

## V1 scope

**Phase 1**: `dsp-conv` crate skeleton.

**Phase 2**: scalar 1-D convolution with all four boundary
modes.

```rust
pub fn conv1d(signal: &[f32], kernel: &[f32], mode: BoundaryMode)
    -> Result<Vec<f32>, ConvError>;
```

Output length = signal length.  The kernel slides over the
extended signal (boundary-extended per `mode`).

**Phase 3**: scalar 2-D convolution on row-major `[H, W]`
real f32 images.

```rust
pub fn conv2d(
    image: &[f32],
    kernel: &[f32],
    image_height: u32,
    image_width: u32,
    kernel_height: u32,
    kernel_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError>;
```

Output is the same `[H, W]` row-major buffer.

**Phase 4**: separable 2-D convolution.

```rust
pub fn sep_conv2d(
    image: &[f32],
    horizontal_kernel: &[f32],
    vertical_kernel: &[f32],
    image_height: u32,
    image_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError>;
```

Two 1-D passes: horizontal across each row, then vertical down
each column.  For an `H · W` image and an `K · K` kernel:
direct 2-D is `O(H · W · K²)`; separable is `O(H · W · K)`.
Big speed win for blurs and Gaussians where the kernel
factors.

**Phase 5**: image filter design helpers.

```rust
pub fn gaussian_blur_kernel(sigma: f32, size: u32) -> Vec<f32>;
pub fn sobel_x_kernel() -> Vec<f32>;       // 3x3, returns 9 floats
pub fn sobel_y_kernel() -> Vec<f32>;
pub fn box_blur_kernel(size: u32) -> Vec<f32>;       // 1-D, length=size
pub fn laplacian_kernel() -> Vec<f32>;     // 3x3
pub fn sharpen_kernel(amount: f32) -> Vec<f32>;      // 3x3
```

Most return 1-D separable kernels (the user calls `sep_conv2d`
with the same kernel along both axes for symmetric blurs).
`sobel_x_kernel` / `sobel_y_kernel` / `laplacian_kernel` /
`sharpen_kernel` return full 3×3 kernels because they're
short and the 2-D form is the canonical representation.

**Phase 6**: matrix-ir-lowered `conv1d` / `conv2d` via
`dsp-fft`'s matrix-ir path (FFT convolution for long kernels).

**Out of V1 scope**:

- Non-linear filters (median, bilateral, morphological).  Lives
  in DSP05+.
- Image rotation, resampling, geometric warps.  DSP06 territory.
- Color-aware filters (need a colorspace primitive).  Future.
- Adaptive filters (LMS/RLS).  Already deferred from DSP03.
- Multi-channel `[B, H, W, C]` convolution.  Single-channel
  `[H, W]` only in V1.

## Boundary modes

For sliding-window convolution we need to define `signal[i]`
when `i < 0` or `i >= N`.  The four modes:

| Mode        | Description                                  | scipy equivalent     |
| ----------- | -------------------------------------------- | -------------------- |
| `Zero`      | Zero-padded.  Easy default.                  | `mode='constant'`    |
| `Replicate` | `signal[-k] = signal[0]`, `signal[N+k] = signal[N-1]`.  Pads with edge values. | `mode='nearest'`     |
| `Reflect`   | Mirror across the boundary, e.g. for `N=5`: `signal[-1] = signal[1]`, `signal[5] = signal[3]`. | `mode='reflect'` (or `mode='mirror'`) |
| `Wrap`      | Periodic: `signal[-1] = signal[N-1]`, `signal[N] = signal[0]`. | `mode='wrap'`        |

Choice depends on the application:

- `Zero` for FIR filtering of signals with known zero context.
- `Replicate` for image edges (prevents dark fringes from blurring).
- `Reflect` for image denoising (smoothest extension).
- `Wrap` for periodic signals or texture filtering.

## Public API

Lives in a new crate **`dsp-conv`** depending on `dsp-fft`
(for Phase 6).

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BoundaryMode {
    Zero,
    Replicate,
    Reflect,
    Wrap,
}

pub fn conv1d(signal: &[f32], kernel: &[f32], mode: BoundaryMode)
    -> Result<Vec<f32>, ConvError>;

pub fn conv2d(
    image: &[f32],
    kernel: &[f32],
    image_height: u32, image_width: u32,
    kernel_height: u32, kernel_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError>;

pub fn sep_conv2d(
    image: &[f32],
    horizontal_kernel: &[f32],
    vertical_kernel: &[f32],
    image_height: u32, image_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError>;

// Phase 5: design helpers
pub fn gaussian_blur_kernel(sigma: f32, size: u32) -> Vec<f32>;
pub fn sobel_x_kernel() -> Vec<f32>;
pub fn sobel_y_kernel() -> Vec<f32>;
pub fn box_blur_kernel(size: u32) -> Vec<f32>;
pub fn laplacian_kernel() -> Vec<f32>;
pub fn sharpen_kernel(amount: f32) -> Vec<f32>;

#[derive(Debug)]
pub enum ConvError {
    EmptySignal,
    EmptyKernel,
    ImageSizeMismatch(String),   // image.len() != H * W
    KernelTooLarge(String),       // kernel bigger than image
}
```

## Numerical accuracy contract

Per the DSP roadmap:

- `conv1d(x, h, Zero)` matches `fir(x, h)` truncated to length
  `N`, with the same kernel orientation.
- `sep_conv2d(image, h_kernel, v_kernel, mode)` matches
  `conv2d(image, h_kernel ⊗ v_kernel, mode)` (where ⊗ is the
  outer product) to within `1e-5` relative tolerance for
  `(H · W) ≤ 64K`.
- Closed-form filter responses (impulse response of an impulse,
  identity kernel, etc.) match analytic expressions to ULP.

## Testing strategy

**Phase 2 (1-D conv)**:

- Error paths: empty signal, empty kernel.
- Closed-form: identity kernel `[1.0]` is identity; centred
  delta kernel preserves signal; box kernel sums.
- All four boundary modes correctly extend a small signal
  with a hand-verified expected output.
- Output length contract: `output.len() == signal.len()`.

**Phase 3 (2-D conv)**:

- Error paths: image size mismatch, kernel bigger than image.
- Closed-form: identity 1×1 kernel = identity; centred delta
  3×3 = identity; 3×3 box blur sum-preserves.
- Boundary modes verified for a 3×3 image + 3×3 kernel
  (boundary samples differ across modes in known ways).

**Phase 4 (separable)**:

- `sep_conv2d` matches `conv2d` with the outer-product kernel
  for several test images.
- 5×5 Gaussian via separable matches 5×5 Gaussian via direct
  2-D to `1e-6` tolerance.
- Speed: anecdotal benchmark in CHANGELOG (not a test
  assertion).

**Phase 5 (design helpers)**:

- `gaussian_blur_kernel(σ=1.0, size=5)` matches known
  reference values to ULP.
- `sobel_x_kernel()` and `sobel_y_kernel()` return the
  canonical 3×3 matrices.
- Sobel applied to a vertical edge produces a strong horizontal
  response (visual sanity check, expressed as a magnitude
  threshold).

**Phase 6 (matrix-ir)**:

- `conv1d_via_runtime` and `conv2d_via_runtime` match the
  scalar paths within `1e-4` tolerance for several N, K
  combinations.

## Phase plan

| Phase  | Lands                                                | Risk |
| ------ | ---------------------------------------------------- | ---- |
| 0      | Spec (this document)                                 | Low. |
| 1+2    | Crate skeleton + scalar `conv1d` with all 4 boundary modes | Low. |
| 3      | Scalar `conv2d` for row-major `[H, W]` images        | Low. |
| 4      | `sep_conv2d` for separable kernels                   | Low — composition of 1-D conv. |
| 5      | Image filter design helpers (Gaussian / Sobel / box / Laplacian / sharpen) | Low. |
| 6      | Matrix-ir-lowered `conv1d` / `conv2d` via `dsp-fft`  | Medium. |

Phases 1+2 typically bundle.  Phases 3 and 4 may also bundle
since `sep_conv2d` is just two `conv1d` passes; the spec splits
them to keep changesets reviewable.

## Dependencies

- `dsp-fft` — reserved for Phase 6's FFT-based long-kernel
  path (`matrix-ir`-lowered conv2d will use the same
  `fft_via_runtime` substrate dsp-dct already calls).
- `dsp-filters` — declared but not strictly needed; the
  existing `fir()` is the "full" mode form and dsp-conv ships
  the "same" mode variants.

No FFI, no `unsafe`, no external crates.

## Open questions

1. **Strided convolution.**  CNN-style stride > 1 (downsampling
   while convolving) is useful for neural networks but out of
   V1's scope (no real consumer yet).  Add when neural network
   layer needs it.
2. **Dilated / atrous convolution.**  Same answer.
3. **Anchor / centering of even-sized kernels.**  Currently V1
   uses the convention `centre = kernel.len() / 2` (integer
   division — for even K, this picks the *upper* centre).
   Future may add an explicit `anchor` parameter.
4. **Per-axis boundary modes.**  V1 uses one `BoundaryMode` for
   both axes of 2-D conv.  Mixed modes (e.g. `Reflect` rows +
   `Zero` columns) are rare in practice; defer.
