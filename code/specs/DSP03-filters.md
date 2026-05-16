# DSP03 — FIR / IIR Filters

**Status**: V1 spec (this document is Phase 0).

**Scope**: a new `dsp-filters` crate providing finite (FIR) and
infinite (IIR) impulse-response filters in pure Rust, plus the
canonical filter design helpers (Butterworth / Chebyshev /
windowed-sinc).  Built on top of `dsp-fft` for FFT-accelerated
convolution and lifts onto the matrix execution layer through
the same path as DSP01 / DSP02.

## Why filters?

Filters are the workhorse primitive of every signal-processing
pipeline.  They show up everywhere:

- **Audio EQ, noise gates, anti-aliasing** — IIR biquads
  (low-pass / high-pass / band-pass) at audio sample rates.
- **Image blurring, sharpening, edge detection** — small FIR
  kernels (3×3, 5×5) convolved over `[H, W]` images.
- **Communications** — root-raised-cosine, matched filtering,
  windowing.
- **Sensor preprocessing** — moving-average, exponential
  smoothing, Savitzky-Golay polynomial fits.
- **OpenCV-equivalent**: `filter2D`, `sepFilter2D`, `GaussianBlur`
  all reduce to FIR/IIR convolution.
- **scipy.signal.lfilter** / **scipy.signal.firwin** / **scipy.signal.butter** — V1 will match these contracts.

After DSP01 (FFT) and DSP02 (DCT), filters are the natural next
primitive.  FIR convolution lifts to FFT for long signals
(`O(N log N)` vs the `O(NK)` direct form), so `dsp-filters`
plugs directly into `dsp-fft`'s matrix-ir path.

## Filter taxonomy

### FIR (Finite Impulse Response)

```text
    y[n] = Σ_{k=0..K-1}  h[k] · x[n - k]
```

- Always stable (no feedback).
- Linear phase (when the kernel is symmetric).
- Easy to design — windowed sinc gives any low-pass / high-pass
  / band-pass at any cutoff.
- Computational cost `O(K)` per output sample for direct
  convolution; `O(log N)` per sample via FFT-based overlap-add
  for long `K`.

### IIR (Infinite Impulse Response)

```text
    y[n] = ( Σ_{k=0..M}  b[k] · x[n - k]
           - Σ_{k=1..N}  a[k] · y[n - k] ) / a[0]
```

- Recursive: each output depends on past outputs.
- Much smaller order than FIR for the same magnitude response
  (a 4th-order Butterworth IIR matches a ~64-tap FIR).
- Can be unstable if poles are outside the unit circle.
- Phase response is non-linear (matters for some applications,
  not for others).
- Implemented as cascades of biquads (2nd-order sections) for
  numerical stability at higher orders.

V1 ships **direct-form-II Transposed** for IIR, which is the
numerically-stable canonical form scipy / MATLAB use.

## V1 scope

**Phase 1**: `dsp-filters` crate skeleton (Cargo.toml,
README.md, CHANGELOG.md, src/lib.rs stub).

**Phase 2**: scalar FIR via direct convolution.

```rust
pub fn fir(signal: &[f32], kernel: &[f32]) -> Result<Vec<f32>, FilterError>;
```

Handles boundary mode = `Full` (length `N + K - 1`, the natural
linear convolution).  Tests: known impulse / box / Gaussian
kernels, scipy cross-check, edge cases (empty signal, empty
kernel).

**Phase 3**: FIR via FFT (overlap-add) for long signals.

```rust
pub fn fir_fft(signal: &[f32], kernel: &[f32]) -> Result<Vec<f32>, FilterError>;
```

Reuses `dsp-fft::fft_via_runtime` so the convolution lifts onto
the matrix execution layer.  Switches between direct and FFT
based on a heuristic (kernel length crossover ~64).

**Phase 4**: scalar IIR direct-form-II Transposed.

```rust
pub fn iir(signal: &[f32], b: &[f32], a: &[f32]) -> Result<Vec<f32>, FilterError>;
```

Matches `scipy.signal.lfilter(b, a, x)` exactly.  Tests: known
biquads (low-pass, high-pass), step response, scipy cross-check.

**Phase 5**: filter design helpers — Butterworth, Chebyshev,
windowed-sinc.

```rust
pub fn design_low_pass(cutoff_hz: f32, sample_rate: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>;
pub fn design_high_pass(cutoff_hz: f32, sample_rate: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>;
pub fn design_band_pass(low_hz: f32, high_hz: f32, sample_rate: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>;
pub fn butterworth_lowpass(order: u32, cutoff_hz: f32, sample_rate: f32)
    -> (Vec<f32>, Vec<f32>);  // (b, a)
pub fn chebyshev1_lowpass(order: u32, ripple_db: f32, cutoff_hz: f32, sample_rate: f32)
    -> (Vec<f32>, Vec<f32>);

pub enum WindowType { Rectangular, Hamming, Hann, Blackman, Kaiser(f32) }
```

**Phase 6**: matrix-ir-lowered FIR.

`build_fir_graph_with_input(signal, kernel) -> (Graph, TensorId)`
emits a matrix-ir Graph computing the FIR via either direct
convolution (small kernel) or FFT (long kernel via
`dsp-fft::build_fft_graph_with_input` × 2).  Lifts to GPU once
backends claim the relevant ops.

**Out of V1 scope**:

- 2-D image filters (`filter2D`, `sepFilter2D`).  Lives in DSP04
  per the DSP overview.
- Adaptive filters (LMS, RLS).  Specialised; defer.
- Wavelet filters.  Lives in DSP06.
- Polyphase resampling, Cascaded Integrator-Comb (CIC) filters.
  Audio-specific; defer.
- Fixed-point / integer kernels.  MX05 territory.

## Public API

Lives in a new crate **`dsp-filters`** depending on `dsp-fft`.

```rust
/// Finite-impulse-response filter via direct convolution.
///
/// `signal` is a length-N real signal; `kernel` is a length-K
/// real impulse response.  Returns a length-(N + K - 1) Vec
/// holding the linear convolution (mode = "full").
pub fn fir(signal: &[f32], kernel: &[f32])
    -> Result<Vec<f32>, FilterError>;

/// FIR via FFT-based overlap-add.  For long kernels (K > ~64)
/// this is asymptotically faster than direct convolution.
/// Reuses `dsp-fft::fft_via_runtime` so the convolution lifts
/// onto the matrix execution layer.
pub fn fir_fft(signal: &[f32], kernel: &[f32])
    -> Result<Vec<f32>, FilterError>;

/// Infinite-impulse-response filter via direct-form-II
/// Transposed.  `b` is the feed-forward (numerator) polynomial,
/// `a` is the feedback (denominator) polynomial.  Matches
/// `scipy.signal.lfilter(b, a, x)` exactly.
///
/// Both `b` and `a` are length ≥ 1.  `a[0]` must be non-zero.
pub fn iir(signal: &[f32], b: &[f32], a: &[f32])
    -> Result<Vec<f32>, FilterError>;

#[derive(Debug)]
pub enum FilterError {
    EmptySignal,
    EmptyKernel,
    InvalidCoefficient(String),  // a[0] == 0, NaN, etc.
    Fft(String),                  // wraps dsp_fft::FftError
}
```

## Numerical accuracy contract

Per the DSP roadmap:

- FIR direct vs FIR FFT round-trip within `1e-4` relative
  tolerance for `N ≤ 64K`, f32 dtype.
- IIR vs scipy reference output within `1e-4` for stable filters
  (poles strictly inside the unit circle).
- Closed-form impulse responses match analytic forms to ULP.

The reference implementation in Phase 2 (`fir`) and Phase 4
(`iir`) is the oracle for all later phases.

## Testing strategy

**Phase 2 (scalar FIR direct)**:

- Error paths: empty signal, empty kernel.
- Closed-form: convolution with `[1.0]` (identity), `[0, 1, 0]`
  (delay by 1), uniform kernel of length K (moving average).
- Known DSP textbook outputs: 5-tap Hamming low-pass on a 64-sample
  sinusoid; impulse response of a windowed-sinc filter.
- Cross-check against a naive O(N · K) reference (which is
  itself the implementation, so this is more of a smoke test).

**Phase 3 (FIR via FFT)**:

- `fir_fft(signal, kernel)` matches `fir(signal, kernel)` within
  `1e-4` tolerance for varying N, K.
- Edge cases: K = 1, N = K, K > N.

**Phase 4 (IIR direct-form-II)**:

- Error paths: empty signal, empty / `a[0] = 0` coefficient
  vectors.
- Closed-form: `b = [1.0], a = [1.0]` (identity).  `b = [1.0],
  a = [1.0, -0.9]` (single-pole low-pass).
- Step response of canonical biquads (low-pass, high-pass).
- Cross-check against scipy's `lfilter` output for several
  filter designs (saved as golden test vectors).

**Phase 5 (design helpers)**:

- `design_low_pass(0.25, 1.0, 33, Hamming)` matches scipy's
  `firwin(33, 0.25, window='hamming')` to ULP.
- `butterworth_lowpass(2, 0.1, 1.0)` matches scipy's
  `butter(2, 0.1)` coefficients.
- The composed `iir(x, *butterworth_lowpass(4, 0.1, 1.0))`
  matches scipy's full filter pipeline.

**Phase 6 (matrix-ir FIR)**:

- `fir_via_runtime` matches `fir` within `1e-4` for several
  N, K combinations.
- Round-trip via the matrix execution layer.

## Phase plan

| Phase  | Lands                                                     | Risk |
| ------ | --------------------------------------------------------- | ---- |
| 0      | Spec (this document)                                      | Low. |
| 1      | `dsp-filters` crate skeleton                              | Low. |
| 2      | Scalar FIR via direct convolution + tests                 | Low. |
| 3      | FIR via FFT (overlap-add) using `dsp-fft`                 | Medium — first FFT integration in DSP03. |
| 4      | Scalar IIR direct-form-II Transposed + tests              | Medium — recursion + numerical stability. |
| 5      | Filter design helpers (Butterworth, Chebyshev, windowed-sinc) | Medium-high — bilinear transform, classical filter math. |
| 6      | Matrix-ir-lowered FIR                                     | Medium — reuses dsp-fft graph builders. |

Phases 1+2 are typically bundled into one PR.  Phase 5 may
itself split into 5a (windowed-sinc), 5b (Butterworth), 5c
(Chebyshev) since each design family is independent.

## Dependencies

- `dsp-fft` — for `fft_scalar` (Phase 3 overlap-add) and
  `fft_via_runtime` / `build_fft_graph_with_input` (Phase 6).
- `dsp-complex` — for the FFT spectra in Phase 3.
- `matrix-ir`, `matrix-runtime`, `matrix-cpu`, `compute-ir`,
  `executor-protocol` — for Phase 6 (same set as `dsp-fft`
  already pulls in).

No FFI, no `unsafe`, no external crates beyond the DSP / matrix
layers we control.

## Open questions

1. **Overlap-save vs overlap-add for Phase 3.**  Both are
   standard; overlap-save is slightly faster but overlap-add
   is easier to verify against the direct path.  Defaulting
   to overlap-add for V1.
2. **Cascade biquad vs direct higher-order IIR.**  Higher-order
   direct forms accumulate noise quickly.  Phase 4 ships direct
   form for the API contract; Phase 5's design helpers will
   factor higher-order filters into biquad cascades and stack
   `iir(...)` calls.
3. **Group delay compensation.**  FIR with linear phase has
   constant group delay = `(K - 1) / 2`.  The Phase 2 API
   returns the full convolution and lets the caller trim;
   later phases may add an `fir_centered` variant.
4. **Streaming filter state.**  The Phase 4 `iir(signal, b, a)`
   API processes a whole signal at once.  A future
   `IirState::new(b, a)` + `state.feed(&samples)` would let
   callers stream samples one block at a time.  Defer until
   a real consumer needs it.
