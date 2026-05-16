# Changelog — dsp-conv

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
