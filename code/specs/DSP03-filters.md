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
pub fn design_low_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>; // legacy panic-on-invalid API
pub fn try_design_low_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Result<Vec<f32>, FilterError>;
pub fn design_high_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>; // legacy panic-on-invalid API
pub fn try_design_high_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Result<Vec<f32>, FilterError>;
pub fn design_band_pass(low_cutoff_norm: f32, high_cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>; // legacy panic-on-invalid API
pub fn try_design_band_pass(low_cutoff_norm: f32, high_cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Result<Vec<f32>, FilterError>;
pub fn butterworth_lowpass(order: u32, cutoff_norm: f32)
    -> (Vec<f32>, Vec<f32>);  // (b, a)
pub fn chebyshev1_lowpass(order: u32, ripple_db: f32, cutoff_norm: f32)
    -> (Vec<f32>, Vec<f32>);

pub enum WindowType { Rectangular, Hamming, Hann, Blackman }
```

The FIR designer contract is lane-specific. `cutoff_norm` is a
cycles-per-sample frequency, with Nyquist at `0.5`; it is not a
frequency in Hz and takes no sample-rate argument. Low-pass and
high-pass accept finite cutoffs in the inclusive range `[0.0, 0.5]`.
Band-pass accepts finite cutoffs satisfying
`0.0 <= low_cutoff_norm < high_cutoff_norm <= 0.5`. Rust uses `f32`,
and Python/TypeScript use their native numeric types; negative zero is
accepted wherever the comparisons admit it.

Rust and Python allow at most `2^20` taps (largest accepted odd count
`1,048,575`); TypeScript allows at most `2^16` taps (largest accepted
odd count `65,535`). All three require a positive odd tap count. The
Rust `u32` parameter cannot represent a negative count. TypeScript
validates integer tap counts at runtime. Python's contract is for callers
supplying an integer as annotated: its implementation checks positivity,
parity, and the cap but does not enforce the runtime type, so wrong-object
behavior (including `bool` or floating-point values) is outside this
contract.

Rust window selection is the typed `WindowType` enum with Rectangular,
Hamming, Hann, and Blackman. Python accepts that lane's enum or a
trimmed, case-insensitive string with one of those four names. TypeScript
accepts those four normalized names plus `hanning` as a Hann alias and
`kaiser` (using the `dsp-fft` default beta of 5); unknown window errors
are lane-specific. Python and TypeScript report invalid design inputs
with their existing typed exceptions. The additive Rust `try_design_*`
APIs report validation errors through the existing
`FilterError::InvalidCoefficient(String)` variant; no new public error
variant is introduced.

Rust validates low/high taps (positive odd, then cap) before cutoff
finiteness and range. Band-pass validates taps, then cap, cutoff
finiteness, then ordering/range. High-pass delegates to low-pass
validation. The legacy Rust `design_*` wrappers and new `try_design_*`
functions share each operation's checked calculation path. The legacy
wrappers preserve their observable panic payload type, message, check
order, and valid-input coefficient bits. Validation precedes
tap-proportional coefficient allocation; this is not a global
no-allocation or no-OOM guarantee.

The current low-pass normalizer uses `sum(h)` and divides only when its
absolute value is positive. Band-pass normalizes by the absolute
center-frequency response, `abs(sum_k h[k] * cos(2*pi*f0*x_k))`, where
`f0=(low_cutoff_norm+high_cutoff_norm)/2` and
`x_k=k-(num_taps-1)/2`; it likewise divides only when the gain is
positive. This work preserves those zero/cancellation branches and
non-finite normalization behavior. A shared cancellation/output policy
is tracked separately in AF-01c. This contract does not promise finite
outputs for every otherwise accepted input.

### Minimal Python/TypeScript conformance fixture (AF-02)

`code/specs/fixtures/biquad-sos-v1/cases.json` is a versioned shared fixture
for exactly two existing public operations: stateful `Biquad.process` and
one-shot `sosfilt`. It uses JSON numbers decoded as finite binary64 values,
exact numeric equality for finite outputs (positive and negative zero compare
equal), and exact output length. No epsilon or platform-dependent
transcendental golden is involved. Fixture validation is test-harness behavior
and does not change either package API.

The input is a 16-sample unit impulse. The `biquad.process` record uses the
single section `[0.5, 0.25, 0.5, 2, -2, 0.5]`; its impulse response is
`h[0]=1/4` and `h[n]=(6n-3)/2^(n+2)` for `1 <= n < 16`. The `sosfilt`
record uses that section followed by `[1, -1, 0, 1, 0, 0]`; its response is
`g[0]=1/4`, `g[1]=1/8`, and `g[n]=(15-6n)/2^(n+2)` for `2 <= n < 16`.
These values are exact dyadic rationals representable in binary64. The fixture
README records the independent derivation and limits.

The Biquad operation is checked on one whole input, chunk lengths `[3, 1,
12]`, and 16 single-sample calls. One instance retains state within each run;
equivalent fresh instances are used for comparisons. A separate reset check
processes a nonempty prefix, resets, and compares replay with a fresh instance.
The test domain includes every nonempty contiguous partition of the 16
samples: `2^15 = 32,768` partitions and 524,288 output samples per lane. SOS
is invoked as a one-shot operation with fresh state each time; it is not a
streaming contract. Empty sections with nonempty input are identity, and empty
input is rejected even when sections are empty.

The fixture has exactly two required unique operation records. Python and
TypeScript test loaders reject missing or duplicate operation IDs, extra or
missing fields, malformed section/sample shapes, booleans or non-numeric and
non-finite values, and wrong expected lengths. Negative controls alter one
expected dyadic sample and must fail as an output-value mismatch. Shared
fixture or loader changes select both package suites in CI. This narrow
fixture does not establish Rust parity, general audio/math/units schemas,
native floating-point proofs, or package-wide exhaustive assurance; those
remain separate backlog work.

The Butterworth and Chebyshev signatures above are existing separate
IIR helpers; their cutoff parameter is also normalized in the current
implementation. AF-01b does not change their behavior or specify their
full validation policy.

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

### Filter-designer normalization compatibility

For the Python and TypeScript FIR design helpers, let `d` be the
normalization denominator: the coefficient sum for low-pass, and the
absolute center-frequency response gain for band-pass,
`abs(sum_k h[k] * cos(2*pi*f0*x_k))`, where `x_k=k-centre`,
`centre=(num_taps-1)/2`, and `f0` is the midpoint of the two band edges.
Preserve the existing result
`h * (1 / d)` bit-for-bit whenever `1 / d` is finite. When `1 / d` is
non-finite and `d` itself is finite and nonzero, use `h / d` so a finite
normalized coefficient is not lost to reciprocal overflow. Keep the
existing behavior when `d` is zero, NaN, or infinite. High-pass and the
`firwin` convenience entry points inherit the behavior of the design
helper they call (`firwin` exists in the TypeScript package).

This compatibility rule applies to the Python and TypeScript lanes. Rust
already divides directly and is not changed by this rule. It defines a
numerical compatibility target for these helpers; it does not establish
native floating-point or Warrant proof evidence.

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

- `design_low_pass(0.25, 33, Hamming)` matches scipy's
  `firwin(33, 0.25, window='hamming')` to ULP.
- `butterworth_lowpass(2, 0.1)` matches scipy's
  `butter(2, 0.1)` coefficients.
- The composed `iir(x, *butterworth_lowpass(2, 0.1))`
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
   form for the API contract. The current Phase 5 Butterworth and
   Chebyshev-I helpers support orders 1 and 2; higher-order
   factorization into biquad cascades and stacked `iir(...)` calls
   remains future work.
3. **Group delay compensation.**  FIR with linear phase has
   constant group delay = `(K - 1) / 2`.  The Phase 2 API
   returns the full convolution and lets the caller trim;
   later phases may add an `fir_centered` variant.
4. **Streaming filter state.**  The Phase 4 `iir(signal, b, a)`
   API processes a whole signal at once.  A future
   `IirState::new(b, a)` + `state.feed(&samples)` would let
   callers stream samples one block at a time.  Defer until
   a real consumer needs it.
