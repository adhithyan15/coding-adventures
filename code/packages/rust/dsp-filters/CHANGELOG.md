# Changelog — dsp-filters

## 0.3.0 — 2026-05-16

### Added — DSP03 Phase 5 (filter design helpers)

Adds the canonical filter design functions: windowed-sinc FIR
(low-pass / high-pass with 4 windows) and Butterworth IIR
(low-pass, orders 1 and 2 via bilinear transform).

#### Public API

```rust
pub enum WindowType {
    Rectangular,
    Hamming,
    Hann,
    Blackman,
}

pub fn design_low_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>;
pub fn design_high_pass(cutoff_norm: f32, num_taps: u32, window: WindowType)
    -> Vec<f32>;
pub fn butterworth_lowpass(order: u32, cutoff_norm: f32)
    -> (Vec<f32>, Vec<f32>);   // (b, a) for use with iir()
```

All re-exported at the crate root.

`cutoff_norm` is normalised so `0.5 = Nyquist`.  FIR `num_taps`
must be odd (linear-phase symmetry + exact spectral inversion
for high-pass).  Butterworth supports orders 1 and 2 in this
phase; higher orders factor into cascaded biquads, a future
phase.

#### Algorithms

**Windowed-sinc low-pass.**  Ideal sinc impulse response
`h[k] = 2·fc · sinc(2·fc·(k - centre))` (with `sinc(0) = 1`),
multiplied by the chosen window, then normalised so the kernel
sums to 1 (DC gain = 1).

**Windowed-sinc high-pass.**  Spectral inversion of the
corresponding low-pass: negate every tap, then add 1.0 at the
centre tap.  Guarantees `lp[k] + hp[k] = δ[k - centre]`
exactly.

**Butterworth low-pass.**  Bilinear transform of the analog
prototype `H_s(s) = 1/(s + 1)` with pre-warping
`ω_c = tan(π · cutoff_norm)`:

- Order 1: `b = [α, α], a = [1, 2α-1]` where
  `α = ω_c / (1 + ω_c)`.  DC gain = 1 by construction.
- Order 2: standard RBJ biquad with `Q = 1/√2`.  Yields a
  3-tap `b` and 3-tap `a`.

#### New unit tests — 11

Windowed-sinc (6):
- `low_pass_sums_to_one` — kernel sums to 1 across all cutoffs / windows.
- `low_pass_kernel_is_symmetric` — linear phase property.
- `high_pass_sums_to_zero` — rejects DC.
- `high_pass_plus_low_pass_is_centred_impulse` — spectral
  inversion identity.
- `low_pass_attenuates_high_frequency` — applies LP to a
  high-freq sinusoid, output amplitude « 1.
- `low_pass_passes_dc` — applies LP to a constant, steady-state
  output ≈ 1.

Butterworth (5):
- `butterworth_order_1_dc_gain_is_one` and
  `butterworth_order_2_dc_gain_is_one` — sum-b / sum-a = 1
  across all cutoffs.
- `butterworth_order_1_step_response_asymptotes_to_one` and
  `butterworth_order_2_step_response_asymptotes_to_one` —
  step input converges to DC gain.
- `butterworth_order_1_attenuates_high_freq` — high-freq
  sinusoid attenuated past steady-state.

All 36 unit tests + 2 doctests pass (10 FIR + 15 IIR + 11
design).

#### Validation

Phase 5 validates via `assert!` for V1.  Panics on:

- Even `num_taps` (FIR designs require odd for linear phase).
- `cutoff_norm` outside `[0.0, 0.5]` (LP/HP) or `(0.0, 0.5)`
  (Butterworth — endpoints would produce degenerate filters).
- Butterworth `order != 1 && order != 2`.

Callers should validate parameters before calling.  Returning
`Result` is on the roadmap for future phases.

### What this phase does NOT include

- Kaiser window (parametric `β`).  Deferred.
- Band-pass / band-stop FIR designs.  Easy follow-up — they're
  high-pass·low-pass cascades.
- Chebyshev I / II IIR designs.  Same structure as Butterworth
  with different prototype; future phase.
- Higher-order Butterworth via cascaded biquads (order 3+).
  Future phase.
- Phase 3 (FIR via FFT overlap-add) is still deferred.
- Phase 6 (matrix-ir-lowered FIR) — pending.

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
