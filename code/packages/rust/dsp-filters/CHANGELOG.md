# Changelog — dsp-filters

## 0.2.0 — 2026-05-15

### Added — DSP03 Phase 4 (scalar IIR direct-form-II Transposed)

Adds `iir(signal, b, a)` — the recursive workhorse for audio
biquads, sensor exponential smoothing, and every "rational
transfer function" filter scipy / MATLAB users reach for.

#### Public API

```rust
pub fn iir(signal: &[f32], b: &[f32], a: &[f32])
    -> Result<Vec<f32>, FilterError>;
```

Re-exported at the crate root.  Matches `scipy.signal.lfilter(b,
a, x)` exactly.

#### Algorithm — Direct-Form-II Transposed

For `order = max(len(b), len(a)) - 1` state slots `z[0..order]`
initialised to zero, both `b` and `a` conceptually zero-padded
to length `order + 1`:

```text
    y[n]                  = (b[0] · x[n] + z[0]) / a[0]
    z[k]    (k = 0..order-2) = b[k+1] · x[n] - a[k+1] · y[n] + z[k+1]
    z[order-1]            = b[order] · x[n] - a[order] · y[n]
```

This is the canonical scipy/MATLAB form: one pass over the
signal, one state vector, no separate past-`x` / past-`y`
buffers.  Numerically stabler than the non-transposed direct
form at higher orders.

Implementation pre-scales `b` and `a` by `1/a[0]` before the
inner loop so the per-sample divide collapses to a multiply.

#### Validation

- `EmptySignal` — signal slice is empty.
- `EmptyKernel` — `a` or `b` is empty.
- `InvalidCoefficient(...)` — `a[0]` is zero, NaN, or infinite
  (can't safely divide).

#### New unit tests — 15

Error paths (6):
- Empty signal, empty `b`, empty `a`, `a[0] = 0`, `a[0] = NaN`,
  `a[0] = ∞`.

Closed-form / known vectors (8):
- Identity filter `b=[1], a=[1]` passes signal through.
- Pure gain `b=[2], a=[1]` doubles signal.
- `b=[1], a=[2]` exercises `a[0]` normalisation.
- Single-pole low-pass `b=[1], a=[1, -0.9]` step response
  asymptotes to `1/(1-0.9) = 10.0`.
- Same filter, first 4 samples match the closed-form geometric
  series.
- Impulse response of `b=[1], a=[1, -0.5]` is `[1, 0.5, 0.25, …]`.
- 2nd-order step response converges to the analytic
  `(Σ b) / (Σ a)` DC gain.
- `iir(x, b, [1.0])` matches `fir(x, b)` on the FIR special
  case — cross-validates the two paths in this crate.

Output length contract (1):
- `iir` output length equals input length for several N.

All 25 unit tests + 1 doctest pass (10 FIR + 15 IIR).

#### Stability note

V1 does not validate pole locations — passing `a` coefficients
whose roots lie outside the unit circle will cause the output
to diverge.  Phase 5's design helpers (Butterworth, Chebyshev)
produce stable coefficients by construction.

### What this phase does NOT include

- Phase 3: `fir_fft` (FFT-based overlap-add via dsp-fft).
- Phase 5: filter design helpers — Butterworth, Chebyshev,
  windowed-sinc.
- Phase 6: matrix-ir-lowered `fir_via_runtime`.
- Streaming filter state (a future `IirState::feed(&samples)`
  could let callers process samples block-by-block; defer
  until a real consumer needs it).

## 0.1.0 — 2026-05-15

### Added — DSP03 Phase 1 + 2 (crate skeleton + scalar FIR direct convolution)

Initial release.  The pure-Rust scalar oracle that Phase 3
(FIR via FFT) and Phase 6 (matrix-ir lowered) will test against.

#### Public API

```rust
pub fn fir(signal: &[f32], kernel: &[f32])
    -> Result<Vec<f32>, FilterError>;

pub enum FilterError {
    EmptySignal,
    EmptyKernel,
    InvalidCoefficient(String),
    Fft(String),
}
```

#### Algorithm

Direct linear convolution:

```text
    y[n] = Σ_{k=0..K-1}  kernel[k] · signal[n - k]
```

Output length is `N + K - 1` (the full convolution, matching
`numpy.convolve(signal, kernel, mode='full')`).  Boundary
handling: input is implicitly zero-padded outside `[0, N)`.
`O(N · K)` time, `O(N + K)` memory.

#### Tests (10)

Error paths:
- `fir_rejects_empty_signal`
- `fir_rejects_empty_kernel`

Closed-form known vectors:
- `fir_with_identity_kernel_returns_signal` — `[1.0]` is identity.
- `fir_with_delay_kernel_shifts_by_one` — `[0.0, 1.0, 0.0]` shifts.
- `fir_with_uniform_kernel_preserves_total_sum`
- `fir_with_box_kernel_3tap` — sums neighborhood of three.

Length contract:
- `fir_output_length_is_n_plus_k_minus_1` for several N, K.

Naive cross-check:
- `fir_matches_naive_reference_n5_k3`
- `fir_matches_naive_reference_n8_k4`
- `fir_matches_naive_reference_n100_k15`

#### Dependencies

- `dsp-fft` — declared but not used in Phase 2.  Reserved for
  Phase 3 (`fir_fft` overlap-add) and Phase 6 (matrix-ir
  graph builder).

No FFI, no `unsafe`, no external crates.

#### What this phase does NOT include

- Phase 3: `fir_fft` (FFT-based overlap-add for long kernels).
- Phase 4: `iir` (direct-form-II Transposed).
- Phase 5: filter design helpers (Butterworth, Chebyshev,
  windowed-sinc).
- Phase 6: matrix-ir-lowered `fir_via_runtime`.
- 2-D filters (`filter2D`).  Lives in DSP04.
