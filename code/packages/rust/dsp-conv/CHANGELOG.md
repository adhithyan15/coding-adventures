# Changelog — dsp-conv

## 0.3.0 — 2026-05-16

### Added — DSP04 Phase 4 + 5 (separable conv + image filter design helpers)

The image-filter toolbox lands.  Adds:

- `sep_conv2d` — separable 2-D convolution (row-then-column).
  Asymptotically faster than `conv2d` when the 2-D kernel
  factors: `O(H·W·(KH+KW))` vs `O(H·W·KH·KW)`.
- Image filter design helpers — `gaussian_blur_kernel`,
  `box_blur_kernel`, `sobel_x_kernel`, `sobel_y_kernel`,
  `laplacian_kernel`, `sharpen_kernel`.  The canonical
  OpenCV / scipy.ndimage primitives.

This makes `dsp-conv` a one-stop-shop for the standard image
preprocessing pipeline: blur, sharpen, edge-detect, denoise.

#### New module — `dsp_conv::sep`

```rust
pub fn sep_conv2d(
    image: &[f32],
    horizontal_kernel: &[f32],
    vertical_kernel: &[f32],
    image_height: u32, image_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError>;
```

Two passes: `conv1d` along each row with `horizontal_kernel`,
then `conv1d` along each column with `vertical_kernel`.  Each
pass uses the same `BoundaryMode` along its corresponding
axis.

#### New module — `dsp_conv::kernels`

```rust
pub fn gaussian_blur_kernel(sigma: f32, size: u32) -> Vec<f32>;
pub fn box_blur_kernel(size: u32) -> Vec<f32>;
pub fn sobel_x_kernel() -> Vec<f32>;     // 3x3, 9 floats
pub fn sobel_y_kernel() -> Vec<f32>;     // 3x3
pub fn laplacian_kernel() -> Vec<f32>;   // 3x3
pub fn sharpen_kernel(amount: f32) -> Vec<f32>;  // 3x3
```

- `gaussian_blur_kernel(σ, size)` — 1-D Gaussian normalised to
  sum=1.  Pass to `sep_conv2d` as both horizontal + vertical
  for a separable 2-D Gaussian blur.  Requires odd `size > 0`
  and `σ > 0`.
- `box_blur_kernel(size)` — 1-D uniform, each tap = 1/size.
- `sobel_x_kernel()` / `sobel_y_kernel()` — directional 3×3
  edge detectors.  Sum to 0 (reject DC).  Pass to `conv2d`
  with `kernel_height = kernel_width = 3`.
- `laplacian_kernel()` — 4-connected discrete `∇²` (`[0,1,0; 1,-4,1; 0,1,0]`).
- `sharpen_kernel(amount)` — identity + `amount` × (-Laplacian).
  Sums to 1 regardless of `amount` (preserves average
  brightness).  Common values: `amount ∈ [0.5, 2.0]`.

All re-exported at the crate root.

#### New unit tests — 21

`sep` module (6):
- 4 error paths: zero dims, empty horizontal kernel, empty
  vertical kernel, image size mismatch.
- 1 matches conv2d with outer-product kernel for 3×3 under
  all 4 boundary modes.
- 1 matches conv2d for a 5×5 Gaussian-like
  `[1,4,6,4,1]/16 ⊗ [1,4,6,4,1]/16` under Replicate.
- 1 identity kernels pass image through unchanged.
- 1 constant image + normalised separable kernel passes
  through under Replicate.

`kernels` module (15):
- Gaussian: sums to 1 across σ ∈ {0.5, 1, 2, 3}; symmetric;
  peak at centre.
- Box: sums to 1; all taps uniform.
- Sobel x/y: sum to 0; Sobel-X applied to a vertical edge
  produces strong horizontal response (≥ 3× the flat-region
  response).
- Laplacian: sums to 0; applied to a constant image produces
  zero (∇² of constant = 0).
- Sharpen: amount=0 gives identity kernel; sums to 1 across
  amounts; constant image passes through.

All 47 unit tests + 1 doctest pass (15 conv1d + 11 conv2d +
6 sep + 15 kernels).

#### Validation conventions

- `gaussian_blur_kernel` and `box_blur_kernel` panic via
  `assert!` on invalid input (even/zero size, non-positive
  σ).  Matches `dsp-filters::design` convention.
- `sep_conv2d` returns `ConvError` for empty kernels, zero
  dims, size mismatch, or kernel-larger-than-image.

### What this phase does NOT include

- Phase 6: matrix-ir-lowered `conv1d` / `conv2d` via dsp-fft.
- Strided / dilated convolution.
- Multi-channel `[B, H, W, C]` images.
- Kaiser-window / Lanczos / bilateral / non-linear filters
  (median, morphological).  DSP05+ territory.

## 0.2.0 — 2026-05-16

### Added — DSP04 Phase 3 (scalar conv2d for [H, W] images)

Adds same-size 2-D convolution on row-major `[H, W]` real
`f32` images.  Uses the same four boundary modes as `conv1d`,
applied independently along each axis.

#### Public API

```rust
pub fn conv2d(
    image: &[f32],
    kernel: &[f32],
    image_height: u32, image_width: u32,
    kernel_height: u32, kernel_width: u32,
    mode: BoundaryMode,
) -> Result<Vec<f32>, ConvError>;
```

Re-exported at the crate root.

#### Algorithm

Direct 2-D convolution with kernel centred at `(KH/2, KW/2)`:

```text
    out[r, c] = Σ_{kr, kc}  kernel[kr, kc]
                           · image_ext[r + ch - kr, c + cw - kc]
```

Boundary extension is applied per axis via the shared
`extend_index` helper (extracted from `conv1d`'s `sample`
during this PR — pure refactor, all prior tests still pass).

`O(H · W · KH · KW)` time, `O(H · W)` memory.  Phase 4 will
add `sep_conv2d` for separable kernels with `O(H · W · (KH + KW))`.

#### Validation

- `image_height == 0 || image_width == 0` → `ImageSizeMismatch`
- `image.len() != H · W` → `ImageSizeMismatch`
- `kernel_height == 0 || kernel_width == 0` → `EmptyKernel`
- `kernel.len() != KH · KW` → `ImageSizeMismatch`
- `kernel_height > image_height || kernel_width > image_width`
  → `KernelTooLarge` (V1 simplification; Reflect mode's
  formula assumes the kernel fits)
- `checked_mul` on `H · W` and `KH · KW` guards against
  overflow on huge u32 inputs.

#### New unit tests — 11

Error paths (6):
- `conv2d_rejects_zero_height`, `conv2d_rejects_zero_width`
- `conv2d_rejects_image_size_mismatch`,
  `conv2d_rejects_kernel_size_mismatch`
- `conv2d_rejects_kernel_too_large`
- `conv2d_rejects_empty_kernel`

Closed-form (2):
- `conv2d_identity_kernel_returns_image` — 1×1 kernel = identity
  under all modes.
- `conv2d_centred_delta_preserves_image` — 3×3 centred delta
  = identity under all modes.

Invariants (1):
- `conv2d_box_kernel_on_constant_image_passes_through` —
  3×3 normalised box on constant 5×5 image yields the same
  constant under Replicate (and at the interior under all
  modes).

Separability cross-check (1):
- `conv2d_outer_product_matches_sequential_conv1d` — a 3×3
  separable kernel `[1,2,1] ⊗ [1,2,1]` (normalised) computed
  via `conv2d` matches the row-then-column composition via
  `conv1d` along each axis with the same boundary mode.

Boundary modes spot-check (1):
- `conv2d_boundary_modes_differ_at_corner` — corner pixel
  (0, 0) of a 3×3 image with a 3×3 uniform kernel yields a
  distinct value under each of the 4 boundary modes
  (Zero=12, Replicate=21, Reflect and Wrap also distinct).

All 26 unit tests + 1 doctest pass (15 conv1d + 11 conv2d).

#### Internal refactor

The 1-D `sample()` helper has been split:
- `extend_index(idx, n, mode) -> Option<usize>` — maps an
  index to its valid source position via boundary extension,
  or returns `None` for Zero-mode out-of-bounds.  `pub(crate)`
  so 2-D conv2d can call it twice (once per axis).
- `sample` is now a thin wrapper around `extend_index`.

Pure refactor — all prior conv1d tests still pass.

### What this phase does NOT include

- Phase 4: `sep_conv2d` (separable 2-D convolution — much faster
  for blurs/Gaussians where the kernel factors).
- Phase 5: image filter design helpers (Gaussian, Sobel, box,
  Laplacian, sharpen).
- Phase 6: matrix-ir-lowered `conv1d` / `conv2d` via dsp-fft.
- Strided / dilated convolution.
- Multi-channel `[B, H, W, C]` images.

## 0.1.0 — 2026-05-16

### Added — DSP04 Phase 1 + 2 (crate skeleton + scalar conv1d with 4 boundary modes)

Initial release.  Same-size 1-D convolution with explicit
boundary control — the image-processing-friendly variant of
linear convolution that complements `dsp-filters::fir`'s
full-mode form.

#### Public API

```rust
pub enum BoundaryMode { Zero, Replicate, Reflect, Wrap }

pub fn conv1d(signal: &[f32], kernel: &[f32], mode: BoundaryMode)
    -> Result<Vec<f32>, ConvError>;

pub enum ConvError {
    EmptySignal,
    EmptyKernel,
    ImageSizeMismatch(String),  // Phase 3 (conv2d)
    KernelTooLarge(String),      // Phase 3 (conv2d)
}
```

#### Algorithm

Direct convolution with same-size output and boundary
extension per `mode`.  For each output index `i ∈ 0..N`:

```text
    out[i] = Σ_{k=0..K-1}  kernel[k] · signal_ext[i + (centre - k)]
```

where `centre = K / 2` (kernel-centered convolution; matches
`scipy.ndimage.convolve`) and `signal_ext[j]` is:

- `Zero`: 0 outside `[0, N)`
- `Replicate`: clamp to `[0, N-1]`
- `Reflect`: mirror about each boundary
- `Wrap`: modular `j mod N` (signed)

`O(N · K)` time, `O(N)` memory.

#### Tests (15)

Error paths (2):
- `conv1d_rejects_empty_signal`
- `conv1d_rejects_empty_kernel`

Closed-form (3):
- `conv1d_identity_kernel_returns_signal`
- `conv1d_centred_delta_preserves_signal`
- `conv1d_output_length_equals_signal_length`

Boundary modes — handwritten with kernel `[1.0, 1.0, 1.0]` (4):
- `conv1d_zero_mode` — kernel summed with implicit zero
  padding gives the expected hand-computed output.
- `conv1d_replicate_mode` — boundary samples replicate.
- `conv1d_reflect_mode` — boundary samples mirror.
- `conv1d_wrap_mode` — boundary samples wrap periodically.

Additional verification (3):
- `conv1d_zero_matches_fir_centre_slice` — for the Zero mode,
  `conv1d` output equals the centre `N` samples of the
  full-mode `dsp_filters::fir(signal, kernel)`.
- `conv1d_wrap_preserves_periodicity` — wrap mode applied to
  a periodic signal preserves periodicity.
- `conv1d_replicate_constant_signal_passes_through` — a
  constant signal under replicate mode yields a constant
  output (no edge artefacts).

Symmetry / integral checks (3):
- `conv1d_symmetric_kernel_symmetric_input` — odd-symmetric
  kernel · even-symmetric input produces a centred output.
- `conv1d_box_kernel_preserves_total_sum_wrap_mode` —
  wrap-mode box convolution preserves the input's total sum.
- `conv1d_short_kernel_each_mode_compiles` — quick coverage
  smoke test for `K = 1` under each mode.

#### Dependencies

- `dsp-fft` — declared but not used in Phase 2.  Reserved for
  Phase 6 (matrix-ir-lowered FFT-based convolution).
- `dsp-filters` (dev-dep only) — used by the `fir` cross-check
  test.

No FFI, no `unsafe`, no external crates.

#### What this phase does NOT include

- Phase 3: scalar `conv2d` for `[H, W]` row-major images.
- Phase 4: `sep_conv2d` (separable 2-D convolution).
- Phase 5: image filter design helpers (Gaussian, Sobel, box,
  Laplacian, sharpen).
- Phase 6: matrix-ir-lowered `conv1d` / `conv2d`.
